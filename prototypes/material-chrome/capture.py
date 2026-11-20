#!/usr/bin/env python3
"""PROTOTYPE captures: chrome palettes from generated Material 3 schemes.

For each scheme x mode x mapping, renders the real app (prototype branch) headless with
REX_PROTO_SCHEME/REX_PROTO_MAP on a private Rex server and private HOME, with the panes declared
a Ghostty theme built from the same scheme (bg = the palette's ground, fg = on_surface, ANSI =
dank16). Usage: capture.py FLAT_JSON OUT_DIR [scheme ...]
"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path("/home/corey/ext/rex-rune")
APP = ROOT / "target/release/rex-rune"
SIZE, SCALE = "1280x800", "1.25"
GROUND = {"A": "surface", "B": "surface_container", "C": "surface_container_lowest"}
SEL = {"A": "primary_container", "B": "secondary_container", "C": "surface_variant"}
PS1 = r"\[\e[34m\]~/rex-rune\[\e[0m\] \[\e[35m\]main\[\e[0m\] \[\e[32m\]❯\[\e[0m\] "
DEMO = r"""demo() {
  printf '\e[1m%s\e[0m\n' 'Normal and bright ANSI'
  for i in 0 1 2 3 4 5 6 7; do printf '\e[4%sm  \e[0m' $i; done; echo
  for i in 0 1 2 3 4 5 6 7; do printf '\e[10%sm  \e[0m' $i; done; echo
  printf '\e[31merror\e[0m: \e[1mcannot borrow `x` as mutable\e[0m\n'
  printf '\e[33mwarning\e[0m: unused variable: `palette`\n'
  printf '\e[32m+ added line\e[0m\n\e[31m- removed line\e[0m\n'
  printf '\e[36mINFO\e[0m server listening on \e[4munix:///run/rex.sock\e[0m\n'
  printf '\e[2mdim text, the kind prompts use for hints\e[0m\n'
}"""
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}
SCREENS = [
    ("workspace", [], 2.0),
    ("commands", [{"t": 1.0, "type": "key", "name": "p", "ctrl": True, "shift": True},
                  {"t": 1.5, "type": "type", "text": "close"}], 2.4),
    ("settings", [{"t": 1.0, "type": "key", "name": "p", "ctrl": True, "shift": True},
                  {"t": 1.5, "type": "type", "text": "Settings"},
                  {"t": 2.0, "type": "key", "name": "Enter"},
                  {"t": 2.6, "type": "type", "text": "Interface"},
                  {"t": 3.0, "type": "key", "name": "Enter"}], 3.8),
    ("confirm", [{"t": 1.0, "type": "key", "name": "p", "ctrl": True, "shift": True},
                 {"t": 1.5, "type": "type", "text": "Close tab"},
                 {"t": 2.0, "type": "key", "name": "Enter"}], 2.8),
]


def ghostty(c, mapping):
    lines = [f"background = {c[GROUND[mapping]]}", f"foreground = {c['on_surface']}",
             f"cursor-color = {c['primary']}", f"cursor-text = {c['on_primary']}",
             f"selection-background = {c[SEL[mapping]]}", f"selection-foreground = {c['on_surface']}"]
    lines += [f"palette = {n}={c[f'color{n}']}" for n in range(16)]
    return "\n".join(lines) + "\n"


def main():
    flat = json.loads(Path(sys.argv[1]).read_text())
    out = Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True)
    wanted = sys.argv[3:] or sorted({k.rsplit(".", 1)[0] for k in flat})
    work = Path(tempfile.mkdtemp(prefix="rex-proto-"))
    home = work / "home"; home.mkdir()
    (home / ".bashrc").write_text(f"PS1='{PS1}'\n{DEMO}\nclear\n")
    (home / ".zshrc").write_text("PROMPT='%F{blue}~/rex-rune%f %F{magenta}main%f %F{green}❯%f '\n" + DEMO + "\nclear\n")
    env = {k: v for k, v in os.environ.items() if not (k in ("WAYLAND_DISPLAY", "DISPLAY", "REX_SERVER", "REX_CONFIG")
           or k.startswith(("XDG_CONFIG", "XDG_STATE", "XDG_CACHE", "XDG_DATA")))}
    env.update(HOME=str(home), SHELL="/bin/bash", USER="user", REX_RUNE_NO_UPDATE_CHECK="1")
    stub = work / "bin"; stub.mkdir()
    (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n"); (stub / "tailscale").chmod(0o755)
    env["PATH"] = f"{stub}:{env['PATH']}"
    log = open(work / "log", "w")
    sock = work / "srv.sock"; ep = f"unix://{sock}"
    srv = subprocess.Popen(["rex", "server", "run", "--listen", ep, "--restore=false", "--state-dir", str(work / "state")],
                           env=env, cwd=home, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
    try:
        while not sock.exists(): time.sleep(0.05)
        time.sleep(0.3)
        api = lambda m, **p: json.loads(subprocess.check_output(["rex", "--autostart=false", "-S", ep, "api", "call", m, json.dumps(p)], text=True))
        layout = {"split": {"direction": "horizontal", "ratio": 0.55, "before": SHELL, "after": SHELL}}
        sid = api("session.create", label="rex-rune", initial_windows=[
            {"window_label": "dev", "layout": layout}, {"window_label": "server", "layout": SHELL}])["session_id"]
        api("session.create", label="dotfiles", initial_windows=[{"window_label": "main", "layout": SHELL}])
        dev = api("session.view", session_id=sid)["windows"][0]
        blocks = [b["block_id"] for b in api("session.list_blocks", session_id=sid)["blocks"] if b.get("window_id") == dev["window_id"]]
        api("session.focus_window", session_id=sid, window_id=dev["window_id"])
        api("session.focus_block", session_id=sid, block_id=blocks[0])
        themes = home / ".config/ghostty/themes"; themes.mkdir(parents=True)
        first = True
        for scheme in wanted:
            for mode in ("dark", "light"):
                c = flat[f"{scheme}.{mode}"]
                path = work / f"{scheme}.{mode}.json"; path.write_text(json.dumps(c))
                for mapping in "ABC":
                    name = f"proto-{scheme}-{mode}-{mapping}"
                    (themes / name).write_text(ghostty(c, mapping))
                    config = work / "workspace.json"
                    config.write_text(json.dumps({"hosts": [{"label": "workstation", "endpoint": ep}], "last_host": ep,
                                                  "sessions": {ep: sid}, "font_size": 14, "sidebar_open": True,
                                                  "terminal_theme": f"named:{name}"}))
                    e = dict(env, REX_PROTO_SCHEME=str(path), REX_PROTO_MAP=mapping)

                    def render(sid_, script, at, png):
                        s = work / "script.json"; s.write_text(json.dumps(script))
                        subprocess.run([str(APP), "--endpoint", ep, "--session", sid_, "--config", str(config), "--",
                                        "--script", str(s), "--snap", str(png), "--at", str(at), "--size", SIZE, "--scale", SCALE],
                                       env=e, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
                    # Attach once so the theme is declared, then redraw the demo under it.
                    render(sid, [], 1.2, work / "warm.png")
                    for b in blocks:
                        subprocess.check_call(["rex", "--autostart=false", "-S", ep, "send", "-b", b, "clear; demo\r"], stdout=subprocess.DEVNULL)
                    time.sleep(1.5 if first else 0.6); first = False
                    for sid_name, script, at in SCREENS:
                        png = out / f"{scheme}.{mode}.{mapping}.{sid_name}.png"
                        render(sid, script, at, png)
                    print("captured", name, flush=True)
    finally:
        srv.terminate(); srv.wait(5)
        if os.environ.get("KEEP") != "1": shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
