# How DMS and matugen deliver a theme change

Research for the *Follow theme source* map, ticket 01: when the wallpaper
changes or light/dark is toggled in DankMaterialShell (DMS), how do DMS and
matugen produce their output, and what can a third-party program that watches a
file rely on?

## Sources and versions

Everything below was read from source at the exact versions installed on the
reference desktop (`dms v2.1.2`, `matugen 4.2.0`), plus read-only inspection of
that desktop. Nothing was regenerated, toggled or reconfigured for this
research; the timings come from the journal and file timestamps of changes the
owner made earlier.

| Source | Revision |
| --- | --- |
| DankMaterialShell, tag `v2.1.2` | [`43255a46`](https://github.com/AvengeMedia/DankMaterialShell/tree/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8) |
| matugen, tag `v4.2.0` | [`bb27a35d`](https://github.com/InioX/matugen/tree/bb27a35d7f9e0f57cf638703f3d8e3182aed0304) |
| Local | `dms matugen --help`, `dms matugen check`, `~/.config/DankMaterialShell/settings.json`, `~/.config/matugen/`, `~/.cache/dms/`, `journalctl --user` |

Link shorthands used below:

- **DMS** `path#Lx` = `https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/path#Lx`
- **MG** `path#Lx` = `https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/path#Lx`

## 1. The pipeline, end to end

1. **Trigger (QML shell).** Every theme-affecting change calls
   `Theme.generateSystemThemesFromCurrentTheme()`, which restarts a 100 ms
   debounce timer (`_themeGenerateDebounce`) and then calls `setDesiredTheme`
   ([DMS `quickshell/Common/Theme.qml#L1977-L2001`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1977-L2001),
   [`#L2581-L2586`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L2581-L2586)).
   Callers include:
   - setting a wallpaper, per-monitor or per-mode wallpaper changes
     ([DMS `quickshell/Common/SessionData.qml#L900-L1195`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/SessionData.qml#L900-L1195));
   - `setLightMode` / `toggleLightMode`, which is what `dms ipc call theme
     dark|light|toggle` invokes (the IPC goes through a further 100 ms
     transition timer first)
     ([Theme.qml#L1467-L1493](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1467-L1493),
     [`#L2558-L2606`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L2558-L2606));
   - theme switches, and any setting declared with `onChange:
     "regenSystemThemes"` — `matugenScheme`, `matugenSpec`,
     `runUserMatugenTemplates`, `terminalPalette`, `terminalsAlwaysDark`, seed
     colour, contrast, target monitor
     ([DMS `quickshell/Common/settings/SettingsSpec.js#L36-L67`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/settings/SettingsSpec.js#L36-L67));
   - shell start-up.
2. **Skip if nothing changed.** `setDesiredTheme` builds the full `dms matugen
   queue …` argument list, hashes it, and returns without running anything if
   the hash equals the last successful run (except the first run at start-up)
   ([Theme.qml#L1906-L1911](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1906-L1911)).
   The journal shows this: `Theme inputs unchanged since the last run, skipping
   generation`.
3. **Coalescing.** While a worker runs, QML keeps only the newest request in
   `pendingThemeRequest` and starts it when the worker exits
   ([Theme.qml#L1786-L1797](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1786-L1797),
   [`#L2402-L2415`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L2402-L2416)).
   The Go side does the same: one current job, one pending job, a newer
   submission cancels the pending one
   ([DMS `core/internal/matugen/queue.go#L44-L77`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/queue.go#L44-L77)).
   Runs are also serialised across processes by an `flock` on
   `~/.cache/dms/matugen.lock`
   ([matugen.go#L260-L283](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L260-L283)).
4. **Go backend (`matugen.Run` → `buildOnce`)**
   ([matugen.go#L292-L563](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L292-L563)):
   1. Resolve the seed. For an image, a cached seed per image
      (`~/.cache/dms/matugen-seeds.json`) or a `matugen … --dry-run --json` run;
      the image source is then rewritten to `color hex <seed>`, and `image` is
      passed back in through the import so `{{image}}` still works.
   2. If `matugenSpec` is `2025` or `dms` (and the scheme supports it), generate
      the full Material role map in-process (see §3).
   3. Generate two dank16 palettes: `dank16` (default profile) and `dank16term`
      (the *Terminal palette* setting's profile).
   4. **Pass A, colours only:** run matugen with a merged config holding only
      the `dank` template, writing `dms-colors.json.tmp`; if its bytes differ
      from the current `~/.cache/dms/dms-colors.json`, `os.Rename` it over
      ([`renderColors`, #L565-L586](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L565-L586)).
   5. **Pass B, all templates:** run matugen again with a merged config holding
      every enabled DMS template, the user's templates and any drop-in configs
      ([`renderTemplates`, #L588-L594](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L588-L594)).
      **This pass runs whether or not the colours changed.**
   6. Send `SIGUSR1` to kitty and `SIGUSR2` to Ghostty
      ([`signalTerminals`, #L1513-L1522](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L1513-L1522)).
   7. Only if the colours changed: GTK accent and portal colour-scheme sync,
      GTK4 reduced-motion pulse, KDE colour scheme, qt6ct touch, fcitx5 reload.
5. **Shell reload.** On worker exit the shell reloads `dms-colors.json` (it also
   watches it with a `FileView`) and emits its internal `matugenCompleted`
   signal ([Theme.qml#L2382-L2401](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L2382-L2401),
   [`#L2447-L2471`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L2447-L2471)).

Wallpaper *transitions* (the cross-fade) do not cause repeated generations:
generation is requested once when the wallpaper path is set, not per animation
frame. A video wallpaper waits for its extracted still frame
([Theme.qml#L1968-L1975](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1968-L1975)).
Rapid wallpaper changes (cycling, scrolling through a picker) collapse to "the
run in flight, then the newest one".

## 2. User templates

**Yes, DMS runs the user's own `~/.config/matugen/config.toml` templates on
every regeneration** — wallpaper change, light/dark toggle, scheme or spec
change — as long as *Run user matugen templates* (`runUserMatugenTemplates`,
default `true`) is on. It is a single flag sent with every run
([Theme.qml#L1805-L1812](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/Common/Theme.qml#L1805-L1812),
[DMS `core/internal/server/matugen_handler.go#L26`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/server/matugen_handler.go#L26)).
There is no separate path for the light/dark toggle: it is a full run with
`-m light` or `-m dark`.

How the merged config is built
([`buildMergedConfig`, matugen.go#L657-L743](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L657-L743)):

1. The user's `[config]` section (text from `[config]` up to the first
   `\n[templates`), if user templates are on.
2. DMS's `matugen/configs/base.toml`, then the internal `[templates.dank]`.
3. Each DMS app template from `templateRegistry`
   ([#L59-L85](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L59-L85)),
   included only if not in `--skip-templates` (each per-app switch in Settings →
   Theme → Apps, or all of them when *Run DMS templates* is off) **and** the app
   is detected (command on `PATH`, Flatpak or config dir). Its TOML comes from
   `quickshell/matugen/configs/<app>.toml` with `SHELL_DIR/`, `CONFIG_DIR/`
   etc. substituted. Terminal templates (`ghostty`, `kitty`, `foot`,
   `alacritty`, `wezterm`, `nvim`) are copied to the temp dir first so DMS can
   rewrite `.default.` → `.dark.` (*Terminals always dark*) and append an
   opacity line ([#L796-L884](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L796-L884)).
   `dms matugen check` prints the detection result.
4. The user's `[templates…]` text, if user templates are on.
5. **Every `~/.config/matugen/dms/configs/*.toml`, appended verbatim and not
   gated by `runUserMatugenTemplates`** (only pass B). This is a DMS-only
   drop-in directory; standalone `matugen` does not read it.

Consequences for a shipped template:

- The merged file lives in `/tmp/matugen-templates-*/templates.toml`, and
  matugen resolves relative paths against the config path
  ([MG `src/template.rs#L493-L533`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L493-L533)),
  so template paths must be absolute or `~/…`. The reference desktop's zsh
  template uses `~/…` and works.
- Template names must be unique across all sources (TOML table keys).
- The user section is spliced by string search, so a `config.toml` that does
  not follow the plain `[config]` then `[templates.x]` layout can be mangled.
- The user's `[config]` reaches the dry runs too, so a broken user config fails
  the whole run.

**`matugenSpec`** (`"2021"` default, `"2025"`, `"dms"`) is a *palette* choice,
not a template switch. With `2025`/`dms` and a supported scheme (tonal spot,
vibrant, expressive, neutral) DMS generates every Material role for dark and
light in-process and passes it to matugen through `--import-json-string` under
`colors`, overriding matugen's own roles for **all** templates, user templates
included ([DMS `core/internal/matugen/spec.go#L14-L24`, `#L65-L178`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/spec.go#L14-L24);
matugen merges imports over its own context:
[MG `src/main.rs#L253-L290`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/main.rs#L253-L290)).
`dms` is the 2025 palettes with raised accent chroma. The reference desktop uses
`matugenSpec: "dms"` with `scheme-tonal-spot`. One caveat: a template that sets
matugen's per-template `type` makes matugen rebuild `colors` from its own
scheme, discarding the import for that template *and every later one*
([MG `src/template.rs#L163-L253`, `#L341-L362`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L163-L253)).
A Rex Rune template must not set `type`.

## 3. What a template can see

Context assembled by matugen
([MG `src/main.rs#L310-L332`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/main.rs#L310-L332),
[`src/helpers.rs#L45-L77`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/helpers.rs#L45-L77))
plus DMS's import
([`buildImportData`, matugen.go#L615-L640](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L615-L640)):

| Key | Contents |
| --- | --- |
| `colors.<role>.{dark,light,default}.<format>` | Every Material 3 role (`background`, `surface*`, `on_surface`, `primary`, `on_primary`, `primary_container`, `secondary_fixed_dim`, `outline`, `source_color`, … about 50) for **both** modes. `default` is the side chosen by `-m`. |
| `dank16.colorN.{dark,light,default}.<format>` | DMS's 16-colour palette (N = 0–15) from primary/surface/container, default profile. |
| `dank16term.colorN.{dark,light,default}.<format>` | Same, using the *Terminal palette* profile; DMS's terminal templates use this one. Present for every template, user ones included. |
| `base16.*`, `palettes.*` | matugen's own base16 scheme and tonal palettes (matugen-generated, not overridden by the DMS spec). |
| `mode` | `"dark"` or `"light"` (smart mode is resolved first). |
| `is_dark_mode` | boolean. |
| `image` | wallpaper path, or null for a colour source. |
| any namespace from `~/.config/matugen/palette-inject.json` | user-supplied external palettes ([paletteinject.go](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/paletteinject.go#L17-L35)). |

Formats (the last path segment):
`hex`, `hex_stripped`, `hex_alpha`, `hex_alpha_stripped`, `alpha_hex`,
`alpha_hex_stripped`, `rgb`, `rgba`, `hsl`, `hsla`, `red`, `green`, `blue`,
`alpha`, `hue`, `saturation`, `lightness`
([MG `src/parser/engine/replace.rs#L21-L39`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/parser/engine/replace.rs#L21-L39);
dank16 provides the same set, [DMS `core/internal/dank16/dank16.go#L26-L50`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/dank16/dank16.go#L26-L50)).

Syntax (matugen's own Chumsky engine, not Jinja): expressions `{{ … }}`,
blocks `<* … *>`, both changeable per template with
`expr_prefix`/`expr_postfix`/`block_prefix`/`block_postfix`
([MG `src/parser/engine.rs#L137-L146`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/parser/engine.rs#L137-L146),
[`src/template.rs#L118-L137`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L118-L137)).
Filters are piped: `set_red`, `set_green`, `set_blue`, `set_alpha`, `set_hue`,
`set_saturation`, `set_lightness`, `lighten`, `saturate`, `auto_lightness`,
`invert`, `grayscale`, `blend`, `harmonize`, `to_color`, `format`, plus string
filters `lower_case`, `snake_case`, `camel_case`, `pascal_case`, `kebab_case`,
`replace` ([MG `src/main.rs#L334-L682`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/main.rs#L334-L682);
docs: [templates](https://iniox.github.io/#matugen/templates),
[filters](https://iniox.github.io/#matugen/filters)). A conditional, from DMS's
foot template:
`dim-blend-towards=<* if {{ is_dark_mode }} *>black<* else *>white<* endif *>`.

Because both modes are always present, a single Rex Rune theme file can carry
the dark and the light scheme plus the current `mode`; the light/dark toggle
then only changes `mode` and the `default` side. DMS's own
`~/.cache/dms/dms-colors.json` is exactly that shape (from
[`templates/dank.json`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/matugen/templates/dank.json)):
`{"dank16": {...}, "mode": "dark", "colors": {"dark": {...}, "light": {...}}}`,
but with `dank16`, not `dank16term`.

## 4. Write pattern

**matugen writes each output in place, not atomically**: `OpenOptions::new()
.create(true).truncate(true).write(true).open(out)` then one `write_all`
([MG `src/template.rs#L259-L339`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L259-L339)).
A watcher can observe the file empty or partly written between the truncate
and the close. A read-only output is skipped with an error. (Aside: with an
array `output_path`, the name-keyed map keeps only the last path, so only one
file is written —
[`#L141-L148`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L141-L148).)

Per regeneration, **each template output is written exactly once** (pass B;
pass A only renders the DMS colours file), followed by a close. It is written
**even when the colours did not change**: pass B is unconditional, so a run
with identical inputs that got past the QML hash check — a forced run, or a
settings change that does not affect colours — rewrites identical bytes and
re-runs hooks. DMS's own `dms-colors.json` is the exception: staged to
`.tmp` and renamed, and only when its bytes change.

**Hooks**: for each template in ascending `index` (default 0; ties in
`HashMap` order, i.e. unspecified), matugen runs `pre_hook`, writes the file,
then runs `post_hook` through the shell and **waits for it**
([MG `src/template.rs#L155-L253`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L155-L253),
[`#L364-L430`](https://github.com/InioX/matugen/blob/bb27a35d7f9e0f57cf638703f3d8e3182aed0304/src/template.rs#L365-L431)).
So a `post_hook` sees its own template's file complete, but not necessarily the
other templates. A slow hook delays every later template and the whole run; a
backgrounded one (`… &`) does not, and DMS reads matugen's stderr through a
file so it does not hang on it
([matugen.go#L1125-L1155](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L1125-L1155)).
DMS always passes `--continue-on-error` on matugen ≥ 3.1, so one broken
template does not stop the others
([#L1088-L1101](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L1088-L1101)).

**Timing on the reference desktop** (journal `Setting desired theme` →
`Matugen worker completed`, and file mtimes; read-only):

| Change | Worker time |
| --- | --- |
| Colour source (`hex`), five runs | 110–155 ms |
| New wallpaper image, seed not cached | ~1.37 s (1.26 s of it before matugen starts: seed extraction) |
| Another wallpaper image | ~0.40 s |

In the last image run, files landed at: `matugen-seeds.json` +0 ms,
`dms-colors.json` +33 ms (pass A), Ghostty theme +89 ms, the user's zsh
template +104 ms (pass B). Add ~100 ms of debounce before the run, and ~100 ms
more for an IPC light/dark toggle. Bursts are therefore at most "one write per
completed run"; during rapid wallpaper changes the runs are back to back, a
few hundred milliseconds to ~1.5 s apart, and intermediate wallpapers are
dropped.

## 5. Notification besides the file write

There is no public "theme generated" signal.

- **DMS socket (`dms` server).** `matugen.queue` replies to its own caller when
  the job finishes and `matugen.status` reports running/pending
  ([matugen_handler.go](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/server/matugen_handler.go#L16-L91)),
  but nothing is broadcast to other clients. The subscribable services include
  `theme.auto` (the automatic day/night *scheduler's* state: `isLight`,
  `nextTransition`) and `wallpaper` (rotation schedule)
  ([DMS `core/internal/server/server.go#L654-L670`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/server/server.go#L654-L670),
  [`thememode/types.go`](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/server/thememode/types.go#L19-L23)).
  Both describe requests, not finished output, and neither covers a manual
  toggle or wallpaper set from the shell.
- **QML** `Theme.matugenCompleted(mode, result)` is internal to the shell.
- **`dms ipc call theme getMode`** reads the current mode; it is a poll.
- **D-Bus**: with *Sync mode with portal* (default on) DMS sets
  `org.gnome.desktop.interface color-scheme`, which the portal re-emits as
  `org.freedesktop.appearance color-scheme`
  ([`syncColorScheme`, matugen.go#L1543-L1554](https://github.com/AvengeMedia/DankMaterialShell/blob/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/core/internal/matugen/matugen.go#L1543-L1554)).
  This carries only dark/light, happens after the templates (inside
  `buildOnce`) or after the run, and is skipped when DMS's GTK theme is not
  active. It says nothing about wallpaper changes.
- **Signals**: DMS sends `SIGUSR1`/`SIGUSR2` only to processes named `kitty`
  and `ghostty`.

So the file write is the only signal a third party gets, and a `post_hook` is
the only way to run code at a known point after that write.

## 6. How DMS's terminal templates map colours

From [`quickshell/matugen/templates/`](https://github.com/AvengeMedia/DankMaterialShell/tree/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/matugen/templates)
(`ghostty.conf`, `kitty.conf`, `foot.ini`, `wezterm.toml`, `alacritty.toml`).
All use the `default` side, so they follow the current mode unless *Terminals
always dark* rewrites them to `dark`.

| Slot | Ghostty | kitty | foot | WezTerm | Alacritty |
| --- | --- | --- | --- | --- | --- |
| background | `background` | `background` | `background` | `background` | `background` |
| foreground | `on_surface` | `on_surface` | `on_surface` | `on_surface` | `on_surface` |
| cursor | `primary` | `primary` | `primary` | `primary` (bg and border) | `primary` |
| cursor text | — | `on_primary` | `background` | `background` | `background` |
| selection background | `primary_container` | `secondary_fixed_dim` | `primary_container` | `primary_container` | `primary_container` |
| selection foreground | `on_surface` | `on_secondary` | `on_surface` | `on_surface` | `on_surface` |
| ANSI 0–7 | `dank16term.color0–7` | same | `regular0–7` | `ansi` | `normal` |
| ANSI 8–15 | `dank16term.color8–15` | same | `bright0–7` | `brights` | `bright` |
| other | — | `url_color` = `primary` | `dim-blend-towards` by mode | — | — |

Outputs: Ghostty `~/.config/ghostty/themes/dankcolors`, kitty
`~/.config/kitty/dank-theme.conf`, foot `~/.config/foot/dank-colors.ini`,
WezTerm `~/.config/wezterm/colors/dank-theme.toml`, Alacritty
`~/.config/alacritty/dank-theme.toml`
([configs/](https://github.com/AvengeMedia/DankMaterialShell/tree/43255a46ea01e9d0de1fe8ce9897c0d71566b2a8/quickshell/matugen/configs)).
The Ghostty theme file is in the Ghostty theme format Rex Rune already
accepts, but it is written only when Ghostty is installed and its template is
enabled, and it holds only the current mode.

## What Rex Rune can rely on

- **A user template runs on every regeneration** (wallpaper, light/dark,
  scheme, spec, terminal palette) when *Run user matugen templates* is on (the
  default). `~/.config/matugen/dms/configs/*.toml` runs under DMS even when it
  is off. Use absolute or `~/` paths and do not set `type`.
- **The template sees both modes and the mode**: `colors.<role>.{dark,light}`,
  `dank16term.colorN.{dark,light}` and `{{mode}}`/`{{is_dark_mode}}`. One file
  can carry both schemes and the current mode, matching what DMS's terminals
  get: background `background`, foreground `on_surface`, cursor `primary`,
  selection `primary_container`/`on_surface`, ANSI from `dank16term`.
- **The write is not atomic**: truncate, then one write. Watch for the close
  (or rename) rather than any modify event, treat an empty or unparseable read
  as "not yet", and never apply a half file. Alternatively the shipped template
  writes to a temporary path and its `post_hook` does `mv` into place, which
  matugen runs synchronously right after the write, giving an atomic rename.
- **Expect duplicates**: the file is rewritten (and hooks re-run) even when
  nothing changed, so compare contents and do nothing when they are equal.
- **Expect at most one write per run**: runs are serialised and coalesced, take
  about 0.1–0.2 s for a colour and 0.4–1.4 s for an image, and start ~100–200 ms
  after the action. A short settle delay (on the order of 100 ms) after the
  last close is enough; no longer debounce is needed.
- **There is no other notification**: no DMS socket event, no D-Bus signal for
  colours; the portal `color-scheme` covers only dark/light and is not ordered
  against the file. The file is the only source of truth.
- **Not a contract**: `~/.cache/dms/dms-colors.json` is written atomically and
  has both modes, but it is DMS-internal state (cache dir, no `dank16term`,
  shape may change); prefer Rex Rune's own template output.
