# PROTOTYPE: chrome palette from a generated Material scheme

Throwaway. Answers [Chrome palette from a generated material scheme](https://github.com/cjstock/rex-rune/issues/4) on the [Follow theme source](https://github.com/cjstock/rex-rune/issues/1) map. Never merged.

- `src/app/theme.rs` (this branch): `REX_PROTO_SCHEME` / `REX_PROTO_MAP=A|B|C` build a leaked palette from a flat role JSON; `white()`/`black()` turn into ink-tinted overlays and softer shadows on a light ground.
- `flat.json`: matugen 4.2.0 `scheme-tonal-spot` roles plus `dms dank16 --variants` for five wallpapers and the desktop's `~/.cache/dms/dms-colors.json`, both modes.
- `capture.py FLAT OUT`: renders the release build with `--snap` against a private Rex server and HOME; panes declare a Ghostty theme from the same scheme.
- `checks.py`: `theme.rs`'s legibility test over every palette, plus an accent/danger hue gap.
- `index.html`: viewer (serve this folder: `python3 -m http.server`).
