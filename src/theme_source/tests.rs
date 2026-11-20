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
// `prototype/terminal-chrome`, its starting values with no adjustment), not this code.

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
            live: 0x50a14f,
            warning: 0xc18401,
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
