# PROTOTYPE: chrome palette derived from a terminal theme

Throwaway. Answers [Deriving a chrome palette from a terminal theme](https://github.com/cjstock/rex-rune/issues/7) on the [Follow theme source](https://github.com/cjstock/rex-rune/issues/1) map. Never merged. Built on `prototype/material-chrome` (its light-ground `white()`/`black()` and leaked palette).

- `derive.py`: the derivation rules and `theme.rs`'s legibility checks. `derive.py stats` runs every rule variant over all 819 Ghostty themes (`stats.txt`); `derive.py one FILE` derives one palette.
- `sources/`: 13 real Ghostty themes (dark, light, mid-tone and monochrome) and four partially keyed matugen files (M3 `chrome-*` keys, no dank16, no palette; one also keys `chrome-danger`).
- `capture.py SOURCES OUT`: real-build `--snap` renders on a private Rex server and HOME (`REX_PROTO_MAP=T` reads the derived slots directly).
- `index.html` + `palettes.json`: viewer (`python3 -m http.server` in this folder).
