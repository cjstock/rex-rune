#!/usr/bin/env python3
"""PROTOTYPE: derive a chrome palette from a Ghostty theme (plus any chrome-* keys), and run
theme.rs's legibility checks over every theme Ghostty ships.

  derive.py stats [THEME_DIR]        failure rates per rule, light and dark
  derive.py one FILE                 the derived palette (JSON) for one theme source file
  derive.py dump THEME_DIR OUT.json  every derived palette, for the viewer

Throwaway. Answers "Deriving a chrome palette from a terminal theme" (cjstock/rex-rune#7).
"""
import colorsys, json, sys
from pathlib import Path

THEMES = Path("/snap/ghostty/current/share/ghostty/themes")
# Ghostty's own default 16 colours, for a theme that leaves palette entries out.
DEFAULT16 = [0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6,
             0x666666, 0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea]
SELECTION_FILL, KEYCAP_FILL, KEYCAP_LIT = 0.18, 0.07, 0.12
SLOTS = ["ground", "card", "popover", "ink", "muted", "accent", "accent2", "live", "warning",
         "danger", "on_accent", "terminal_selection"]


# --- colour arithmetic (same as theme.rs) -----------------------------------------------
def mix(a, b, t):
    return sum(round(((a >> s) & 255) * t + ((b >> s) & 255) * (1 - t)) << s for s in (16, 8, 0))


def lum(c):
    ch = [((c >> s) & 255) / 255 for s in (16, 8, 0)]
    ch = [x / 12.92 if x <= .04045 else ((x + .055) / 1.055) ** 2.4 for x in ch]
    return .2126 * ch[0] + .7152 * ch[1] + .0722 * ch[2]


def cr(a, b):
    x, y = lum(a), lum(b)
    return (max(x, y) + .05) / (min(x, y) + .05)


def hls(c):
    return colorsys.rgb_to_hls(*[((c >> s) & 255) / 255 for s in (16, 8, 0)])


def from_hls(h, l, s):
    r, g, b = colorsys.hls_to_rgb(h, max(0, min(1, l)), s)
    return (round(r * 255) << 16) | (round(g * 255) << 8) | round(b * 255)


def hue(c):
    return hls(c)[0] * 360


def gap(a, b):
    g = abs(hue(a) - hue(b))
    return min(g, 360 - g)


def is_light(c):  # terminal_theme::is_light
    r, g, b = (c >> 16) & 255, (c >> 8) & 255, c & 255
    return (299 * r + 587 * g + 114 * b) // 1000 > 128


def hexs(c):
    return f"#{c:06x}"


# --- parsing ---------------------------------------------------------------------------
def parse(text):
    keys, pal = {}, {}
    for line in text.splitlines():
        if "=" not in line or line.lstrip().startswith("#"):
            continue
        k, v = (x.strip().strip('"') for x in line.split("=", 1))
        if k == "palette":
            n, _, col = v.partition("=")
            try:
                pal[int(n)] = int(col.strip().lstrip("#")[:6], 16)
            except ValueError:
                pass
            continue
        try:
            keys[k] = v if k == "scheme" else int(v.lstrip("#")[:6], 16)
        except ValueError:
            pass
    return keys, pal


# --- the derivation --------------------------------------------------------------------
def away(c, ground, need, check):
    """Move c's HLS lightness away from the ground (keeping hue and saturation) in small steps
    until check(c) holds. Returns (colour, steps taken) or (None, steps) if it never holds."""
    h, l, s = hls(c)
    step = 0.02 if not is_light(ground) else -0.02
    for i in range(51):
        cand = from_hls(h, l + step * i, s)
        if check(cand):
            return cand, i
    return None, 50


def pick(pal, normal, bright, popover):
    """Of a normal/bright pair, the one that stands out more from the popover."""
    a, b = pal[normal], pal[bright]
    return a if cr(a, popover) >= cr(b, popover) else b


FALLBACK = {  # canonical status hues, used only when a theme's own fail the hue check
    "dark": {"warning": 0xf5d48a, "danger": 0xff8a8a, "live": 0x5eead4},
    "light": {"warning": 0x9a6700, "danger": 0xc62828, "live": 0x1a7f37},
}


def derive(keys, pal16, rule="R"):
    """Returns (palette dict, mode, notes). Tries the plain rules first, then lifts ink (and so
    muted) away from the ground step by step until every check passes."""
    best = None
    for deepen in range(0, 16):
        for lift in range(0, 26):
            p, mode, notes = attempt(keys, pal16, rule, lift, deepen)
            fails = [x for x in checks(p, mode) if not x.startswith("on_accent")]
            if not fails:
                if deepen:
                    notes.insert(0, f"ground deepened {deepen}")
                if lift:
                    notes.insert(0, f"ink +{lift}")
                return p, mode, notes
            best = best or (p, mode, notes + ["unsolved"])
            if rule in ("nolift", "nofallback"):
                return best
        if rule != "deepen" and rule != "R":
            break
    return best


def nudge(warning, danger, ground, ok):
    """Turn warning's hue away from danger, toward yellow, until they are 30 degrees apart."""
    h, l, s = hls(warning)
    dh = hls(danger)[0]
    sign = 1 if ((h - dh) % 1.0) < 0.5 else -1
    for i in range(1, 60):
        cand = from_hls((h + sign * i / 360) % 1.0, l, s)
        if gap(cand, danger) >= 30:
            return away(cand, ground, 3, ok)[0] or cand, i
    return warning, 0


def attempt(keys, pal16, rule, lift, deepen=0):
    pal = {i: pal16.get(i, DEFAULT16[i]) for i in range(16)}
    notes = []
    bg, fg = keys["background"], keys["foreground"]
    mode = keys.get("scheme") or ("light" if is_light(bg) else "dark")
    p = {}
    ck = lambda k: keys.get(f"chrome-{k.replace('_', '-')}")

    p["ground"] = ck("ground") or (mix(0xffffff if mode == "light" else 0, bg, 0.05 * deepen) if deepen else bg)
    p["card"] = ck("card") or p["ground"]
    if ck("ink"):
        p["ink"] = ck("ink")
    else:
        h, l, s = hls(fg)
        p["ink"] = from_hls(h, l + (0.02 if not is_light(p["ground"]) else -0.02) * lift, s)
    p["popover"] = ck("popover") or mix(p["ink"], p["ground"], 0.07)
    over = p["ink"] if mode == "light" else 0xffffff
    k = 0.9 if mode == "light" else 1.0
    if not ck("muted"):
        for t in [x / 100 for x in range(55, 101)]:
            m = mix(p["ink"], p["ground"], t)
            keys_ = mix(p["ink"], m, .5)
            if min(cr(m, p["card"]), cr(m, p["popover"])) >= 4.5 and \
                    cr(keys_, mix(over, p["card"], (KEYCAP_FILL + .08) * k)) >= 4.5:
                break
        p["muted"] = m
    else:
        p["muted"] = ck("muted")

    def sel_ok(tone):
        fill = mix(tone, p["popover"], SELECTION_FILL)
        return (cr(p["ink"], fill) >= 4.5 and cr(p["ink"], mix(over, fill, KEYCAP_LIT * k)) >= 4.5
                and cr(tone, p["popover"]) >= 3)

    def fit(slot, normal, bright, check):
        if ck(slot):
            p[slot] = ck(slot)
            return
        if rule == "cursor" and slot == "accent" and "cursor-color" in keys and hls(keys["cursor-color"])[2] > .25:
            base = keys["cursor-color"]
        else:
            base = pick(pal, normal, bright, p["popover"])
        got, steps = away(base, p["ground"], 3, check)
        if got is None:
            notes.append(f"{slot}: no lightness passes")
            got = base
        elif steps:
            notes.append(f"{slot} +{steps}")
        p[slot] = got

    status_ok = lambda c: min(cr(c, p["card"]), cr(c, p["popover"])) >= 3
    fit("accent", 4, 12, sel_ok)
    fit("accent2", 5, 13, status_ok)
    fit("live", 2, 10, status_ok)
    fit("danger", 1, 9, lambda c: status_ok(c) and sel_ok(c))
    fit("warning", 3, 11, status_ok)
    # A grey (monochrome) theme has no hue to carry trouble: use the canonical ones.
    for slot, chk in (("danger", lambda c: status_ok(c) and sel_ok(c)), ("warning", status_ok)):
        if not ck(slot) and hls(p[slot])[2] < 0.2 and rule != "nofallback":
            p[slot] = away(FALLBACK[mode][slot], p["ground"], 3, chk)[0] or FALLBACK[mode][slot]
            notes.append(f"{slot}: grey, canonical")
    if gap(p["warning"], p["danger"]) < 30 and not ck("warning"):
        notes.append(f"warn~danger {gap(p['warning'], p['danger']):.0f}°")
        if rule == "fallback":
            p["warning"] = away(FALLBACK[mode]["warning"], p["ground"], 3, status_ok)[0] or FALLBACK[mode]["warning"]
            notes.append("status fallback")
        elif rule != "nofallback":
            p["warning"], turned = nudge(p["warning"], p["danger"], p["ground"], status_ok)
            notes.append(f"warning turned {turned}°")
    ok = lambda c: cr(c, p["accent"]) >= 4.5
    p["on_accent"] = ck("on_accent") or next((c for c in (p["ground"], p["ink"]) if ok(c)),
                                              max((0, 0xffffff), key=lambda c: cr(c, p["accent"])))
    p["terminal_selection"] = keys.get("selection-background") or mix(p["accent"], p["ground"], 0.3)
    return p, mode, notes


# --- checks (every_palette_is_legible_and_pairs_with_its_terminal_theme) -------------------
def checks(p, mode):
    out = []
    need = lambda name, v, t: out.append(f"{name} {v:.2f}<{t}") if v < t else None
    over = p["ink"] if mode == "light" else 0xffffff
    k = 0.9 if mode == "light" else 1.0
    need("muted/card", cr(p["muted"], p["card"]), 4.5)
    need("ink/popover", cr(p["ink"], p["popover"]), 4.5)
    keys = mix(p["ink"], p["muted"], .5)
    for s, u, l in (("popover", p["popover"], 0), ("sidebar", p["card"], .08)):
        need(f"keycaps/{s}", cr(keys, mix(over, u, (KEYCAP_FILL + l) * k)), 4.5)
    for kind, tone in (("sel", p["accent"]), ("dsel", p["danger"])):
        fill = mix(tone, p["popover"], SELECTION_FILL)
        need(f"ink/{kind}", cr(p["ink"], fill), 4.5)
        need(f"keycap/{kind}", cr(p["ink"], mix(over, fill, KEYCAP_LIT * k)), 4.5)
        need(f"{kind} outline", cr(tone, p["popover"]), 3)
    for w in ("warning", "danger"):
        for s in ("card", "popover"):
            need(f"{w}/{s}", cr(p[w], p[s]), 3)
    need("warn~danger hue", gap(p["warning"], p["danger"]), 30)
    # Not in theme.rs, reported for the record:
    need("on_accent/accent", cr(p["on_accent"], p["accent"]), 4.5)
    return out


def load(path):
    return parse(Path(path).read_text(errors="replace"))


def main():
    cmd = sys.argv[1]
    if cmd == "one":
        keys, pal = load(sys.argv[2])
        p, mode, notes = derive(keys, pal)
        print(json.dumps({"mode": mode, "notes": notes, "fails": checks(p, mode),
                          "tokens": {s: hexs(p[s]) for s in SLOTS}}, indent=1))
        return
    d = Path(sys.argv[2]) if len(sys.argv) > 2 and cmd == "stats" else THEMES
    if cmd == "dump":
        d = Path(sys.argv[2])
    files = sorted(f for f in d.iterdir() if f.is_file())
    if cmd == "stats":
        for rule in ("raw", "nolift", "R", "cursor"):
            tally = {"dark": [0, 0, 0], "light": [0, 0, 0]}
            why = {}
            for f in files:
                keys, pal = load(f)
                if "background" not in keys or "foreground" not in keys:
                    continue
                if rule == "raw":  # no adjustment at all: straight picks, to show why adjusting matters
                    pal2 = {i: pal.get(i, DEFAULT16[i]) for i in range(16)}
                    bg, fg = keys["background"], keys["foreground"]
                    mode = "light" if is_light(bg) else "dark"
                    pop = mix(fg, bg, .07)
                    p = dict(ground=bg, card=bg, popover=pop, ink=fg, muted=mix(fg, bg, .6),
                             accent=pick(pal2, 4, 12, pop), accent2=pick(pal2, 5, 13, pop), live=pick(pal2, 2, 10, pop),
                             warning=pick(pal2, 3, 11, pop), danger=pick(pal2, 1, 9, pop))
                    p["on_accent"] = max((bg, fg), key=lambda c: cr(c, p["accent"]))
                    notes = []
                else:
                    p, mode, notes = derive(keys, pal, rule)
                fails = checks(p, mode)
                t = tally[mode]
                t[0] += 1
                t[1] += bool([x for x in fails if not x.startswith("on_accent")])
                t[2] += any("fallback" in n or "turned" in n for n in notes); t.append(int(any(n.startswith("ink +") for n in notes)))
                for x in fails:
                    why[x.split()[0] if "hue" not in x else "warn~danger hue"] = why.get(x.split()[0] if "hue" not in x else "warn~danger hue", 0) + 1
            print(f"rule {rule:10} " + "  ".join(f"{m}: {t[1]}/{t[0]} fail, {t[2]} warning changed, {sum(t[3:])} ink lifted" for m, t in tally.items()))
            print("   ", dict(sorted(why.items(), key=lambda kv: -kv[1])))
    if cmd == "dump":
        out = {}
        for f in files:
            keys, pal = load(f)
            if "background" not in keys or "foreground" not in keys:
                continue
            p, mode, notes = derive(keys, pal)
            out[f.name] = {"mode": mode, "notes": notes, "fails": checks(p, mode),
                           "tokens": {s: hexs(p[s]) for s in SLOTS}}
        Path(sys.argv[3]).write_text(json.dumps(out))


if __name__ == "__main__":
    main()
