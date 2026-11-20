#!/usr/bin/env python3
"""Follow theme source on private Rex/Xvfb; never personal server/display/HOME/clipboard.

Requires Linux, rex, Xvfb, openbox, xdotool, ImageMagick and Pillow. REX_BIN selects an
already-built client (default target/release/rex-rune). Starts its own Rex server on a private
socket, with its own HOME, XDG config, data, state, runtime and cache folders and a /bin/sh
shell; the client gets the same environment, so its theme source is the private
`$XDG_STATE_HOME/rex-rune/theme-source`. Captures, the theme source files and results stay in
~/.cache/agent-ws.

Chooses Theme source with Follow chrome through the private workspace.json, then checks:
the chrome (the toolbar's most common pixel) and the owned pane's body take the file's colours
at launch; an in-place rewrite to a light theme and an atomic rename to a theme with no
`chrome-*` keys are each followed live; the server's terminal for that pane reports the
declared palette entry (`format` with `extra_palette`) and, to a program in it, the declared
background and foreground (OSC 11 and OSC 10); a broken file keeps the last colours, also
across a relaunch; and the four theme source files a real HOME could hold (the theme source, its
remembered copy, the theme source template and the DMS drop-in) are untouched. Never presses
Set up DMS and never runs dms or matugen. Does not test native Wayland, a pane another client
owns, or the one notice a broken file posts (the headless tests cover those).
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-rune"))
REAL_HOME = Path.home()


def real(var, default):
    value = os.environ.get(var)
    return Path(value) if value and os.path.isabs(value) else REAL_HOME / default


# The files a real HOME could hold for this feature, read before the environment is replaced.
REAL_FILES = [
    real("XDG_STATE_HOME", ".local/state") / "rex-rune/theme-source",
    real("XDG_STATE_HOME", ".local/state") / "rex-rune/last-good-theme-source",
    real("XDG_DATA_HOME", ".local/share") / "rex-rune/matugen/theme-source-template",
    real("XDG_CONFIG_HOME", ".config") / "matugen/dms/configs/rex-rune.toml",
]


def fingerprint(paths):
    out = {}
    for path in paths:
        try:
            stat = path.stat()
            out[str(path)] = (stat.st_size, stat.st_mtime_ns)
        except FileNotFoundError:
            out[str(path)] = None
    return out


before_real = fingerprint(REAL_FILES)
(REAL_HOME / ".cache/agent-ws").mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-theme-source-", dir=REAL_HOME / ".cache/agent-ws"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
for name in ("home", "config", "data", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
# A login shell that is zsh would otherwise greet the empty private HOME with its setup menu.
(work / "home/.zshrc").write_text("PS1='$ '\n")
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_DATA_HOME=str(work / "data"), XDG_STATE_HOME=str(work / "state"),
                  XDG_RUNTIME_DIR=str(work / "runtime"), XDG_CACHE_HOME=str(work / "cache"),
                  SHELL="/bin/sh", REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"),
                  REX_SERVER="unix://" + str(work / "rex.sock"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)

SOURCE = work / "state/rex-rune/theme-source"
REMEMBERED = work / "state/rex-rune/last-good-theme-source"
SIZE = (1024, 700)

# Three theme sources: dark with every chrome key, light with its own, and a plain Ghostty
# theme whose chrome is derived. Each palette 1 differs so the server's copy tells them apart.
DARK = {"background": "1b2433", "foreground": "d8dee9", "red": "e06c75", "ground": "26324a"}
LIGHT = {"background": "f4f1ea", "foreground": "2a2a2a", "red": "b3261e", "ground": "e4dccd"}
PLAIN = {"background": "282a36", "foreground": "f8f8f2", "red": "ff5555"}


def theme_text(theme, chrome=True):
    lines = ["# Private theme source written by theme-source-x11.py", "rex-rune-theme-source = 1",
             f"background = #{theme['background']}", f"foreground = #{theme['foreground']}",
             f"palette = 1=#{theme['red']}"]
    if chrome:
        light = int(theme["background"][:2], 16) > 0x80
        lines += ["scheme = " + ("light" if light else "dark"),
                  f"chrome-ground = #{theme['ground']}", f"chrome-card = #{theme['ground']}",
                  f"chrome-ink = #{theme['foreground']}"]
    return "\n".join(lines) + "\n"


def rgb(hex6):
    return tuple(int(hex6[i:i + 2], 16) for i in (0, 2, 4))


def near(a, b, slack=3):
    return all(abs(x - y) <= slack for x, y in zip(a, b))


def wait(predicate, description, timeout=15):
    """Wait for an asynchronous native/server event, never retry a failed assertion."""
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


shot = 0


def colours():
    """The toolbar's and the pane body's most common colours in a fresh capture."""
    global shot
    shot += 1
    desk.capture(window, f"shot-{shot:02}")
    image = Image.open(work / "captures" / f"shot-{shot:02}.png").convert("RGB")
    def mode(box):
        return max(image.crop(box).getcolors(SIZE[0] * SIZE[1]))[1]
    x0, y0 = accept.grid_origin()
    return mode((0, 0, SIZE[0], accept.TOOLBAR_HEIGHT)), mode((x0 + 40, y0 + 200, SIZE[0] - 40, SIZE[1] - 40))


def wait_colours(toolbar, pane, name):
    """Wait until the chrome and the pane body are drawn in `toolbar` and `pane`."""
    seen = []
    def drawn():
        seen.append(colours())
        got_toolbar, got_pane = seen[-1]
        return (toolbar is None or toolbar(got_toolbar)) and near(got_pane, rgb(pane))
    try:
        wait(drawn, name, timeout=10)
    except AssertionError:
        raise AssertionError(f"{name}: last toolbar/pane colours {seen[-1] if seen else None}")
    return seen[-1]


def server_red():
    """Palette entry 1 as the server's terminal for the pane holds it."""
    vt = accept.api("com.superlogical.terminal.format", session_id=sid, block_id=block,
                    args={"format": "vt", "extra_palette": True})["content"]
    marker = "\x1b]4;1;rgb:"
    start = vt.find(marker)
    return vt[start + len(marker):start + len(marker) + 8].replace("/", "") if start >= 0 else None


queries = 0


def queried():
    """What a program in the pane hears back for OSC 11 and OSC 10: (background, foreground)."""
    global queries
    queries += 1
    reply = work / f"reply-{queries}"
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": f"python3 {probe} {reply}"}})
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})
    wait(lambda: reply.exists(), "colour query program did not finish")
    answers = reply.read_bytes().decode(errors="replace").split("\x07")
    def colour(code):
        for answer in answers:
            if f"]{code};rgb:" in answer:
                r, g, b = answer.split("rgb:")[1].split("/")
                return r[:2] + g[:2] + b[:2]
    return colour(11), colour(10)


def follows(theme, name):
    """The server's terminal for the owned pane holds `theme`."""
    wait(lambda: server_red() == theme["red"], f"{name}: palette 1 never reached the server")
    assert queried() == (theme["background"], theme["foreground"]), f"{name}: OSC 11/10 replies"


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          cwd=work / "home", stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    probe = work / "query.py"
    probe.write_text("import os,sys,tty,termios,select,time\n"
                     "old=termios.tcgetattr(0); tty.setraw(0)\n"
                     "os.write(1,b'\\x1b]11;?\\x07\\x1b]10;?\\x07')\n"
                     "data=b''; until=time.monotonic()+2\n"
                     "while time.monotonic()<until and data.count(b'\\x07')<2:\n"
                     " if select.select([0],[],[],.05)[0]: data+=os.read(0,4096)\n"
                     "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                     "open(sys.argv[1]+'.part','wb').write(data); os.rename(sys.argv[1]+'.part',sys.argv[1])\n"
                     "print('\\x1b[2J\\x1b[H',end='',flush=True)\n")
    sid, block = accept.session("Theme source")
    SOURCE.parent.mkdir(parents=True)
    SOURCE.write_text(theme_text(DARK))
    config = accept.pin_layout(work / "workspace.json", sidebar_open=False, font_family="IBM Plex Mono",
                               chrome_theme="theme_source", terminal_theme="follow_chrome",
                               theme_source={"before": "midnight"})
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="%dx%d" % SIZE, log=log)
    owner = accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block).get("owner")
    print("Pane owner after launch:", owner, flush=True)

    wait_colours(lambda c: near(c, rgb(DARK["ground"])), DARK["background"], "launch: dark theme source")
    follows(DARK, "launch")
    print("PASS launch: chrome ground, pane body and the server's terminal take the dark theme source", flush=True)

    SOURCE.write_text(theme_text(LIGHT))  # in place, as matugen writes
    wait_colours(lambda c: near(c, rgb(LIGHT["ground"])), LIGHT["background"], "in-place rewrite: light")
    follows(LIGHT, "in-place rewrite")
    print("PASS in-place rewrite: light chrome and the owned pane follow without a relaunch", flush=True)

    part = SOURCE.with_name("theme-source.part")
    part.write_text(theme_text(PLAIN, chrome=False))
    os.rename(part, SOURCE)  # atomic replacement
    toolbar, _ = wait_colours(lambda c: sum(c) < 3 * 0x60, PLAIN["background"], "rename: derived dark chrome")
    follows(PLAIN, "rename")
    print("PASS atomic rename: chrome derived from a plain Ghostty theme (toolbar #%02x%02x%02x) and the pane follow"
          % toolbar, flush=True)

    SOURCE.write_text("this is not a theme\n")
    time.sleep(1)  # Past the 100 ms settle; nothing is expected to change.
    assert colours() == (toolbar, rgb(PLAIN["background"])), "a broken file changed the colours"
    assert server_red() == PLAIN["red"], "a broken file changed the server's palette"
    assert REMEMBERED.read_text() == theme_text(PLAIN, chrome=False), "last good theme not remembered"
    app.terminate()
    app.wait(timeout=5)
    app, window = desk.launch(sid, config, size="%dx%d" % SIZE, log=log)
    wait_colours(lambda c: near(c, toolbar), PLAIN["background"], "relaunch with a broken file")
    assert json.loads(config.read_text())["chrome_theme"] == "theme_source"
    print("PASS broken file: last colours kept, remembered in the private state folder and across a relaunch",
          flush=True)

    after_real = fingerprint(REAL_FILES)
    assert after_real == before_real, f"a real theme source file changed: {before_real} -> {after_real}"
    print("PASS nothing written to the real HOME's theme source, remembered copy, template or drop-in", flush=True)
    (work / "results.json").write_text(json.dumps({
        "owner": owner, "toolbar_plain": toolbar, "preferences": json.loads(config.read_text())}, indent=2))
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=5)
