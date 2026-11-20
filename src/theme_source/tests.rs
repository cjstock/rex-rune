use super::*;
use serde_json::json;

/// Catppuccin Mocha's colours as a Ghostty theme, palette 6–8 and 14–15 left out.
const MOCHA: &str = "\
palette = 0=#45475a
palette = 1=#f38ba8
palette = 2=#a6e3a1
palette = 3=#f9e2af
palette = 4=#89b4fa
palette = 5=#f5c2e7
palette = 9=#f38ba8
palette = 10=#a6e3a1
palette = 11=#f9e2af
palette = 12=#89b4fa
palette = 13=#f5c2e7
background = #1e1e2e
foreground = #cdd6f4
cursor-color = #f5e0dc
selection-background = #585b70
";

/// Atom One Light's ground and text with five palette colours.
const ONE_LIGHT: &str = "\
background = #fafafa
foreground = #383a42
palette = 1=#e45649
palette = 2=#50a14f
palette = 3=#c18401
palette = 4=#0184bc
palette = 5=#a626a4
";

fn derived(text: &str) -> Derived {
    ThemeSource::parse(text).unwrap().derive()
}

fn reason(text: &str) -> String {
    ThemeSource::parse(text).unwrap_err()
}

// Expected palettes are the prototype's reference derivation (`derive.py` on
// `prototype/terminal-chrome`), not this code.

#[test]
fn a_plain_ghostty_theme_gives_its_starting_values() {
    let d = derived(MOCHA);
    assert_eq!(
        d.chrome,
        Chrome {
            ground: 0x1e1e2e,
            card: 0x1e1e2e,
            popover: 0x2a2b3c,
            ink: 0xcdd6f4,
            muted: 0x8c92ab,
            accent: 0x89b4fa,
            accent2: 0xf5c2e7,
            live: 0xa6e3a1,
            warning: 0xf9e2af,
            danger: 0xf38ba8,
            on_accent: 0x1e1e2e,
            terminal_selection: 0x585b70,
        }
    );
    assert_eq!(
        serde_json::to_value(d.terminal.args()).unwrap(),
        json!({
            "background": "#1e1e2e",
            "foreground": "#cdd6f4",
            "cursor": "#f5e0dc",
            "palette": [
                "0=#45475a", "1=#f38ba8", "2=#a6e3a1", "3=#f9e2af", "4=#89b4fa", "5=#f5c2e7",
                "9=#f38ba8", "10=#a6e3a1", "11=#f9e2af", "12=#89b4fa", "13=#f5c2e7",
            ],
            "scheme": "dark",
        }),
        "the terminal theme is the file's own, judged dark by its background"
    );
}

#[test]
fn a_light_theme_gives_light_chrome_with_ink_tinted_keycaps() {
    let d = derived(ONE_LIGHT);
    assert_eq!(d.terminal.scheme, Some("light"));
    assert_eq!(
        d.chrome,
        Chrome {
            ground: 0xfafafa,
            card: 0xfafafa,
            popover: 0xeceded,
            ink: 0x383a42,
            // 75% of ink over the ground. The reference prints 0x686a70: Python rounds the
            // red channel's 104.5 to even, and the chrome's `mix` rounds half away from zero.
            muted: 0x696a70,
            accent: 0x0184bc,
            accent2: 0xa626a4,
            // Too faint on the popover as written: each moves darker until it is 3:1 (step 1).
            live: 0x499348,
            warning: 0xb77d01,
            // Ghostty's own bright red stands out more from the popover than the file's red.
            danger: 0xd54e53,
            on_accent: 0x000000,
            terminal_selection: 0xafd7e7,
        }
    );
}

#[test]
fn palette_entries_left_out_read_as_ghosttys_defaults() {
    let d = derived("background = #000000\nforeground = #ffffff\n");
    assert_eq!(
        d.chrome,
        Chrome {
            ground: 0x000000,
            card: 0x000000,
            popover: 0x121212,
            ink: 0xffffff,
            muted: 0x8c8c8c,
            accent: 0x7aa6da,
            accent2: 0xc397d8,
            live: 0xb9ca4a,
            warning: 0xf0c674,
            danger: 0xcc6666,
            on_accent: 0x000000,
            terminal_selection: 0x253241,
        }
    );
    assert!(
        d.terminal.palette.is_empty(),
        "the declared theme stays the file's: the server keeps its own palette"
    );
}

#[test]
fn keyed_chrome_slots_are_used_verbatim_and_the_card_defaults_to_the_ground() {
    let text = format!(
        "{MOCHA}chrome-ground = #101010\nchrome-popover = #202020\nchrome-ink = #fefefe\n\
         chrome-muted = #333333\nchrome-accent = #ff0000\nchrome-accent2 = #00ff00\n\
         chrome-live = #0000ff\nchrome-warning = #ff0001\nchrome-danger = #ff0002\n\
         chrome-on-accent = #ff0003\n"
    );
    let d = derived(&text);
    assert_eq!(
        d.chrome,
        Chrome {
            ground: 0x101010,
            card: 0x101010,
            popover: 0x202020,
            ink: 0xfefefe,
            // Illegible on purpose: a keyed slot is never checked or adjusted.
            muted: 0x333333,
            accent: 0xff0000,
            accent2: 0x00ff00,
            live: 0x0000ff,
            warning: 0xff0001,
            danger: 0xff0002,
            on_accent: 0xff0003,
            terminal_selection: 0x585b70,
        }
    );
    assert_eq!(
        d.terminal.background,
        Some(0x1e1e2e),
        "the panes keep the terminal background"
    );
    let card = derived(&format!(
        "{MOCHA}chrome-ground = #101010\nchrome-card = #121212\n"
    ));
    assert_eq!((card.chrome.ground, card.chrome.card), (0x101010, 0x121212));
}

#[test]
fn scheme_says_dark_or_light_and_otherwise_the_background_decides() {
    let scheme = |text: &str| derived(text).terminal.scheme;
    assert_eq!(scheme(&format!("{MOCHA}scheme = light\n")), Some("light"));
    assert_eq!(scheme(&format!("{ONE_LIGHT}scheme = dark\n")), Some("dark"));
    assert_eq!(
        scheme(&format!("{ONE_LIGHT}scheme = sepia\n")),
        Some("light"),
        "an unknown scheme is ignored"
    );
}

#[test]
fn parsing_follows_ghostty_rules() {
    let d = derived(
        "# written by hand\n\
         background = #000000\n\
         foreground = #ffffff\n\
         font-size = 13\n\
         chrome-sparkle = #123456\n\
         # chrome-accent = #00ff00\n\
         chrome-accent = #ff0000\n\
         chrome-accent = \"#0000ff\"\n\
         chrome-live = green\n\
         background = #0a0a0a\n\
         scheme = light\n\
         scheme = dark\n",
    );
    assert_eq!(
        d.chrome.accent, 0x0000ff,
        "a later line wins, quoted or not"
    );
    assert_eq!(d.chrome.ground, 0x0a0a0a, "for terminal keys too");
    assert_eq!(d.terminal.scheme, Some("dark"));
    assert_eq!(
        d.chrome.live, 0xb9ca4a,
        "an unreadable colour is left out, so the slot is derived"
    );
}

#[test]
fn a_file_without_its_required_keys_or_with_a_newer_version_is_refused_with_a_reason() {
    assert_eq!(reason(""), "it has no theme");
    assert_eq!(reason("font-size = 13\nhello\n"), "it has no theme");
    assert_eq!(
        reason("foreground = #ffffff\n"),
        "it has no background colour"
    );
    assert_eq!(
        reason("background = #000000\n"),
        "it has no foreground colour"
    );
    assert_eq!(
        reason("background = nope\nforeground = #ffffff\n"),
        "it has no background colour"
    );
    assert_eq!(
        reason(&format!("{MOCHA}rex-rune-theme-source = 2\n")),
        "version 2 is newer than this client reads"
    );
    assert_eq!(
        reason(&format!("{MOCHA}rex-rune-theme-source = two\n")),
        "version “two” is not a number"
    );
    assert!(ThemeSource::parse(&format!("{MOCHA}rex-rune-theme-source = 1\n")).is_ok());
    assert!(
        ThemeSource::parse(MOCHA).is_ok(),
        "no version reads as version 1"
    );
}

#[test]
fn reading_tells_a_missing_file_from_one_without_a_theme() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("theme-source");
    assert_eq!(read(&file), Read::Missing);
    std::fs::write(&file, MOCHA).unwrap();
    assert_eq!(read(&file), Read::Theme(derived(MOCHA)));
    std::fs::write(&file, b"background = #000000\nforeground = #ffffff\n\xff\n").unwrap();
    assert_eq!(read(&file), Read::Unreadable("it isn't text".into()));
    assert!(
        matches!(read(folder.path()), Read::Unreadable(why) if !why.is_empty()),
        "a folder is there but has no theme"
    );
}

#[test]
fn the_path_is_the_state_folders_unless_settings_name_one() {
    let default = path(None);
    assert!(default.ends_with("rex-rune/theme-source"), "{default:?}");
    assert_eq!(default, crate::platform::dirs::state().join("theme-source"));
    assert_eq!(path(Some("  ")), default);
    assert_eq!(path(Some("/etc/x/theme")), PathBuf::from("/etc/x/theme"));
    assert_eq!(
        path(Some("~/.cache/matugen/rex")),
        crate::platform::dirs::home().join(".cache/matugen/rex")
    );
}

// Legible derived chrome. The samples are real Ghostty themes and partially keyed matugen files
// (`tests/data/theme-source`); expected palettes are the reference derivation's output for them.

/// A sample theme source's text.
fn sample(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/theme-source")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

fn sample_chrome(name: &str) -> ChromeColors {
    derived(&sample(name)).chrome
}

/// A palette written as the reference prints it: `slot=rrggbb` in slot order.
fn palette(tokens: &str) -> ChromeColors {
    let colors: Vec<u32> = tokens
        .split_whitespace()
        .map(|token| u32::from_str_radix(token.split_once('=').unwrap().1, 16).unwrap())
        .collect();
    let [
        ground,
        card,
        popover,
        ink,
        muted,
        accent,
        accent2,
        live,
        warning,
        danger,
        on_accent,
        terminal_selection,
    ] = colors[..]
    else {
        panic!("twelve slots: {tokens}")
    };
    Chrome {
        ground,
        card,
        popover,
        ink,
        muted,
        accent,
        accent2,
        live,
        warning,
        danger,
        on_accent,
        terminal_selection,
    }
}

/// Every sample with what the reference derives from it: dark, light, mid-tone (Fairyfloss,
/// Hot Dog Stand, and Great Pumpkin on the light side), monochrome (X Berlin), warning on
/// danger's hue (Aura) and matugen files that key the chrome but not its status colours.
const SAMPLES: [(&str, &str); 13] = [
    (
        "dracula",
        "ground=282a36 card=282a36 popover=373843 ink=f8f8f2 muted=a1a1a3 accent=d6acff \
         accent2=ff92df live=69ff94 warning=ffffa5 danger=ff6e6e on_accent=282a36 \
         terminal_selection=44475a",
    ),
    (
        "nord",
        "ground=2e3440 card=2e3440 popover=3a404c ink=dfe4ed muted=a6acb6 accent=81a1c1 \
         accent2=b48ead live=a3be8c warning=ebcb8b danger=c8777f on_accent=2e3440 \
         terminal_selection=eceff4",
    ),
    (
        "aura",
        "ground=15141b card=15141b popover=24232a ink=edecee muted=8c8b8f accent=a277ff \
         accent2=a277ff live=61ffca warning=f7ff85 danger=ffca85 on_accent=15141b \
         terminal_selection=a277ff",
    ),
    (
        "x-berlin",
        "ground=000000 card=000000 popover=0e0e0e ink=cccccc muted=7c7c7c accent=aaaaaa \
         accent2=cccccc live=dddddd warning=f5d48a danger=ff8a8a on_accent=000000 \
         terminal_selection=cccccc",
    ),
    (
        "fairyfloss",
        "ground=363246 card=363246 popover=444053 ink=fcfcf9 muted=b1afb5 accent=c2ffdf \
         accent2=ffb8d1 live=c2ffdf warning=ffea00 danger=ff857f on_accent=363246 \
         terminal_selection=8077a8",
    ),
    (
        "hot-dog-stand",
        "ground=751a12 card=751a12 popover=7f2a23 ink=ffffff muted=cfafac accent=949494 \
         accent2=ffff54 live=ffff54 warning=a7ff54 danger=ffff54 on_accent=000000 \
         terminal_selection=ffff54",
    ),
    (
        "great-pumpkin",
        "ground=f98733 card=f98733 popover=e87e30 ink=060403 muted=4c2a11 accent=143142 \
         accent2=421d3e live=283719 warning=443106 danger=58160b on_accent=f98733 \
         terminal_selection=f6c89b",
    ),
    (
        "catppuccin-latte",
        "ground=eff1f5 card=eff1f5 popover=e3e5ea ink=43465d muted=626578 accent=1e66f5 \
         accent2=e138b4 live=3a9027 warning=b27117 danger=d20f39 on_accent=ffffff \
         terminal_selection=dc8a78",
    ),
    (
        "github-light-default",
        "ground=ffffff card=ffffff popover=eff0f0 ink=1f2328 muted=696c6f accent=0969da \
         accent2=8250df live=116329 warning=4d2d00 danger=a40e26 on_accent=ffffff \
         terminal_selection=1f2328",
    ),
    (
        "matugen-no-dank16-dark",
        "ground=130d09 card=130d09 popover=261e18 ink=fceee5 muted=b4a9a1 accent=faae66 \
         accent2=fffbce live=b9ca4a warning=f0c674 danger=cc6666 on_accent=5a3000 \
         terminal_selection=d8914c",
    ),
    (
        "matugen-no-dank16-light",
        "ground=fff4ee card=fff4ee popover=efe0d5 ink=352d28 muted=635a53 accent=874e0c \
         accent2=605e14 live=7e853a warning=967a14 danger=d54e53 on_accent=fff0e6 \
         terminal_selection=faae66",
    ),
    (
        "matugen-error-only-dark",
        "ground=130d09 card=130d09 popover=261e18 ink=fceee5 muted=b4a9a1 accent=faae66 \
         accent2=fffbce live=b9ca4a warning=f0cc74 danger=ff7351 on_accent=5a3000 \
         terminal_selection=d8914c",
    ),
    (
        "matugen-error-only-light",
        "ground=fff4ee card=fff4ee popover=efe0d5 ink=352d28 muted=635a53 accent=874e0c \
         accent2=605e14 live=7e853a warning=967a14 danger=b02500 on_accent=fff0e6 \
         terminal_selection=faae66",
    ),
];

#[test]
fn every_sample_derives_legible_chrome_as_the_reference_does() {
    for (name, expected) in SAMPLES {
        let chrome = sample_chrome(name);
        assert_eq!(legible::failures(&chrome), [], "{name}");
        assert_eq!(chrome, palette(expected), "{name}");
    }
}

/// The colour a theme source gives terminal palette entry `index`.
fn entry(text: &str, index: u8) -> u32 {
    let terminal = ThemeSource::parse(text).unwrap().terminal;
    terminal
        .palette
        .iter()
        .find(|&&(i, _)| i == index)
        .map_or(GHOSTTY_16[index as usize], |&(_, color)| color)
}

fn same_hue(a: u32, b: u32) -> bool {
    legible::hue_gap(a, b) < 1.
}

#[test]
fn step_1_a_faint_status_colour_moves_away_from_the_ground_keeping_its_hue() {
    // No dank16: live and warning start as Ghostty's own green and yellow, too pale for a
    // light popover.
    let text = sample("matugen-no-dank16-light");
    let starting = derived(&text).terminal;
    let c = sample_chrome("matugen-no-dank16-light");
    for (word, color, normal) in [("live", c.live, 2), ("warning", c.warning, 3)] {
        // Of the pair, the one that stands out more from the popover.
        let own = [entry(&text, normal), entry(&text, normal + 8)]
            .into_iter()
            .find(|&own| same_hue(color, own));
        assert!(
            own.is_some_and(|own| own != color),
            "{word} moved, keeping its hue"
        );
        assert!(
            legible::contrast(color, c.popover) >= legible::MARK,
            "{word} now reads"
        );
        assert!(
            legible::contrast(color, c.popover) < legible::MARK + 0.3,
            "{word} moved only as far as it had to"
        );
    }
    assert_eq!(
        (c.live, c.warning),
        (0x7e853a, 0x967a14),
        "darker, on a light ground"
    );
    assert_eq!(
        (Some(c.ground), Some(c.ink)),
        (starting.background, starting.foreground),
        "nothing else had to change"
    );
}

#[test]
fn step_2_a_warning_on_dangers_hue_turns_away_just_far_enough() {
    // Aura's yellow is its red: both #ffca85.
    let text = sample("aura");
    assert!(legible::hue_gap(entry(&text, 11), entry(&text, 9)) < 1.);
    let c = sample_chrome("aura");
    let gap = legible::hue_gap(c.warning, c.danger);
    assert!((30. ..32.).contains(&gap), "{gap}°");
    assert_eq!(c.danger, 0xffca85, "danger keeps its colour");
    assert_eq!(c.warning, 0xf7ff85, "toward yellow");
    // And around a keyed danger: matugen's error colour is 28° from Ghostty's yellow.
    let c = sample_chrome("matugen-error-only-dark");
    assert_eq!(c.danger, 0xff7351, "keyed, so verbatim");
    assert!(legible::hue_gap(GHOSTTY_16[3], c.danger) < 30.);
    assert!(legible::hue_gap(c.warning, c.danger) >= 30.);
    assert_eq!(c.warning, 0xf0cc74);
}

#[test]
fn step_3_grey_trouble_colours_take_canonical_amber_and_red() {
    let c = sample_chrome("x-berlin");
    assert_eq!((c.warning, c.danger), (0xf5d48a, 0xff8a8a), "dark");
    let grey_light = "background = #f4f4f4\nforeground = #303030\npalette = 1=#707070\n\
                      palette = 3=#909090\npalette = 9=#606060\npalette = 11=#808080\n";
    let c = derived(grey_light).chrome;
    assert_eq!((c.warning, c.danger), (0x9a6700, 0xc62828), "light");
    assert_eq!(legible::failures(&c), []);
}

#[test]
fn step_4_ink_lifts_and_popover_and_muted_follow_it() {
    // Nord's danger can't reach the destructive selection's keycaps under its own ink.
    let text = sample("nord");
    let starting = derived(&text).terminal;
    let c = sample_chrome("nord");
    let foreground = starting.foreground.unwrap();
    assert_eq!(foreground, 0xd8dee9);
    assert_eq!(c.ink, 0xdfe4ed, "one step lighter");
    assert!(same_hue(c.ink, foreground));
    assert_eq!(
        c.popover,
        legible::mix(c.ink, c.ground, 0.07),
        "popover follows"
    );
    assert_eq!(
        c.muted, 0xa6acb6,
        "and muted is derived again from the new ink"
    );
    assert_eq!(Some(c.ground), starting.background, "the ground stays");
}

#[test]
fn step_5_the_ground_steps_toward_black_or_white_and_the_panes_keep_theirs() {
    let d = derived(&sample("fairyfloss"));
    assert_eq!(d.terminal.background, Some(0x5a5475), "the panes' own");
    assert_eq!(
        d.chrome.ground,
        legible::mix(0x000000, 0x5a5475, 0.4),
        "eight steps toward black"
    );
    assert_eq!(d.chrome.card, d.chrome.ground);
    let d = derived(&sample("great-pumpkin"));
    assert_eq!(d.terminal.scheme, Some("light"));
    assert_eq!(d.terminal.background, Some(0xf98128));
    assert_eq!(
        d.chrome.ground,
        legible::mix(0xffffff, 0xf98128, 0.05),
        "one step toward white"
    );
}

#[test]
fn keyed_slots_stay_verbatim_when_they_fail_and_the_rest_still_adjust() {
    // A keyed danger far too pale for a light ground.
    let text = format!(
        "{}chrome-danger = #ffeeee\n",
        sample("matugen-no-dank16-light")
    );
    let c = derived(&text).chrome;
    assert_eq!(c.danger, 0xffeeee);
    assert!(
        legible::failures(&c)
            .iter()
            .all(|f| f.check.contains("danger") || f.check.contains("destructive")),
        "{:?}",
        legible::failures(&c)
    );
    assert!(!legible::failures(&c).is_empty());
    assert_eq!(
        (c.live, c.warning),
        (0x7e853a, 0x967a14),
        "the derived slots adjust as without it"
    );
    // A keyed ink that fails is never lifted, and the ground never moves under a keyed one.
    let text = format!(
        "{}chrome-ground = #5a5475\nchrome-ink = #7a7495\n",
        sample("fairyfloss")
    );
    let c = derived(&text).chrome;
    assert_eq!((c.ground, c.ink), (0x5a5475, 0x7a7495));
}

#[test]
fn a_generated_accent_may_share_dangers_hue() {
    // matugen's orange primary and its red error.
    let c = sample_chrome("matugen-error-only-dark");
    assert_eq!((c.accent, c.danger), (0xfaae66, 0xff7351));
    assert!(legible::hue_gap(c.accent, c.danger) < 30.);
    assert_eq!(legible::failures(&c), []);
}

/// The whole Ghostty theme collection: every theme Ghostty ships derives legible chrome. Not
/// run in CI; run with `cargo test --lib -- --ignored every_ghostty_theme`, pointing
/// `REX_GHOSTTY_THEMES` at a themes folder when Ghostty's isn't found in the usual places.
#[test]
#[ignore = "needs Ghostty's themes installed"]
fn every_ghostty_theme_derives_legible_chrome() {
    let dir = std::env::var_os("REX_GHOSTTY_THEMES")
        .map(PathBuf::from)
        .or_else(|| {
            crate::terminal_theme::ThemeDirs::from_env()
                .system
                .into_iter()
                .find(|dir| dir.is_dir())
        })
        .expect("Ghostty's themes: set REX_GHOSTTY_THEMES");
    let mut checked = 0;
    let mut failing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(source) = ThemeSource::parse(&text) else {
            continue;
        };
        checked += 1;
        let chrome = source.derive().chrome;
        if std::env::var_os("REX_GHOSTTY_DUMP").is_some() {
            println!("{:?} {chrome:06x?}", entry.file_name());
        }
        let failures = legible::failures(&chrome);
        if !failures.is_empty() {
            failing.push((entry.file_name(), failures));
        }
    }
    assert!(checked > 0, "no themes in {dir:?}");
    assert!(
        failing.is_empty(),
        "{} of {checked}: {failing:#?}",
        failing.len()
    );
}
