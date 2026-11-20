//! Theme source: choosing it, reading the file into the chrome and the declared terminal
//! theme, leaving it, and what Settings says about it. Each test writes a real file in its own
//! folder and delivers the read the file watcher will later trigger.
use super::*;
use crate::app::{panel::SettingsSection, settings_kit::ChipTone, theme::rgb};
use rex_rune::{
    settings::{ChromeChoice, ChromeTheme},
    terminal_theme::TerminalTheme,
};
use serde_json::{Value, json};

const DARK: &str = "\
# A theme source written by hand
rex-rune-theme-source = 1
background = #1e1e2e
foreground = #cdd6f4
palette = 1=#f38ba8
palette = 4=#89b4fa
";

const LIGHT: &str = "\
background = #fafafa
foreground = #383a42
palette = 4=#0184bc
";

fn dark_args() -> Value {
    json!({
        "background": "#1e1e2e",
        "foreground": "#cdd6f4",
        "palette": ["1=#f38ba8", "4=#89b4fa"],
        "scheme": "dark",
    })
}

/// A window with `hosts` captured hosts whose theme source is `theme-source` in its own
/// folder, saved so a relaunch reads the same file.
fn source_workspace(
    hosts: usize,
) -> (
    Workspace,
    Vec<flume::Receiver<HostCommand>>,
    std::path::PathBuf,
) {
    let mut ws = workspace_hosts(hosts);
    let file = ws.test_folder.as_ref().unwrap().join("theme-source");
    ws.settings.theme_source.path = Some(file.display().to_string());
    ws.theme_source.path = file.clone();
    ws.save();
    let captured = (0..hosts).map(|i| capture_host(&mut ws, i)).collect();
    (ws, captured, file)
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
}

fn window(rt: &Runtime<Workspace>) -> rune::Color {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("window"))
        .unwrap()
        .fill
}

/// Every theme a host was told to declare since last asked, as JSON.
fn told(host: &flume::Receiver<HostCommand>) -> Vec<Value> {
    declared(host)
        .into_iter()
        .map(|args| serde_json::to_value(args).unwrap())
        .collect()
}

/// Press `label` in the open list, then draw exactly one frame.
fn choose_row(rt: &mut Runtime<Workspace>, label: &str) {
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.label.as_deref() == Some(label) && e.role == "menuitem")
        .unwrap_or_else(|| panic!("a {label} row"))
        .rect;
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1. / 60.);
}

/// Settings → Interface, at rest.
fn interface(ws: Workspace) -> Runtime<Workspace> {
    let mut ws = ws;
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Interface);
    let mut rt = runtime_at(ws, (1280., 900.));
    settle(&mut rt);
    rt
}

fn said(rt: &Runtime<Workspace>) -> (&'static str, ChipTone, String) {
    let said = rt.state.theme_source_said();
    (said.word, said.tone, said.sentence)
}

#[test]
fn choosing_theme_source_draws_its_file_at_once_and_declares_it_to_every_host() {
    let (ws, hosts, file) = source_workspace(2);
    std::fs::write(&file, DARK).unwrap();
    let mut rt = interface(ws);
    click(&mut rt, "chrome-theme");
    let rows: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.current))
        .collect();
    assert_eq!(
        rows.last(),
        Some(&("Theme source".to_owned(), false)),
        "last, after the built-in palettes"
    );
    choose_row(&mut rt, "Theme source");
    assert_eq!(
        window(&rt),
        rgb(0x1e1e2e),
        "the ground changes in one frame, without a spring"
    );
    assert_eq!(crate::app::theme::kit().c.page, rgb(0x1e1e2e));
    assert_eq!(crate::app::theme::kit().c.text_base, rgb(0xcdd6f4));
    let s = &rt.state.settings;
    assert_eq!(s.chrome_theme, ChromeChoice::ThemeSource);
    assert_eq!(
        s.terminal_theme,
        TerminalThemeChoice::FollowChrome,
        "never chosen: the terminals follow the chrome, which follows the file"
    );
    for host in &hosts {
        assert_eq!(told(host), [dark_args()]);
    }
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(saved.chrome_theme, ChromeChoice::ThemeSource);
    assert_eq!(
        saved.theme_source.before,
        ChromeTheme::Midnight,
        "the palette it replaced"
    );

    // The read the watcher will deliver: a light file, light chrome, in one frame.
    std::fs::write(&file, LIGHT).unwrap();
    rt.update(|s, tx| s.read_theme_source(tx));
    rt.frame(1. / 60.);
    assert_eq!(window(&rt), rgb(0xfafafa));
    assert_eq!(crate::app::theme::kit().c.page, rgb(0xfafafa));
    assert!(rt.state.palette.is_light());
    for host in &hosts {
        assert_eq!(
            told(host),
            [json!({
                "background": "#fafafa",
                "foreground": "#383a42",
                "palette": ["4=#0184bc"],
                "scheme": "light",
            })]
        );
    }
    assert_eq!(rt.state.term_colors().background, 0xfafafa);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn a_terminal_theme_chosen_before_stays_and_only_the_chrome_follows() {
    let (mut ws, hosts, file) = source_workspace(1);
    std::fs::write(&file, DARK).unwrap();
    ws.apply_theme(TerminalThemeChoice::RexDefault).unwrap();
    declared(&hosts[0]);
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    rt.frame(1. / 60.);
    assert_eq!(window(&rt), rgb(0x1e1e2e));
    assert_eq!(
        rt.state.settings.terminal_theme,
        TerminalThemeChoice::RexDefault
    );
    assert_eq!(rt.state.term_theme, Some(TerminalTheme::default()));
    assert!(told(&hosts[0]).is_empty(), "nothing new is declared");

    // Follow chrome, chosen later, is the file's terminal theme.
    rt.update(|s, tx| s.perform(Intent::Theme(TerminalThemeChoice::FollowChrome), tx));
    assert_eq!(told(&hosts[0]), [dark_args()]);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn a_broken_file_says_why_and_changes_nothing() {
    let (ws, hosts, file) = source_workspace(1);
    std::fs::write(&file, DARK).unwrap();
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    told(&hosts[0]);
    for (text, why) in [
        ("foreground = #ffffff\n", "it has no background colour"),
        ("background = #000000\n", "it has no foreground colour"),
        ("this is not a theme\n", "it has no theme"),
        (
            "rex-rune-theme-source = 2\nbackground = #000000\nforeground = #ffffff\n",
            "version 2 is newer than this client reads",
        ),
    ] {
        std::fs::write(&file, text).unwrap();
        rt.update(|s, tx| s.read_theme_source(tx));
        rt.frame(1. / 60.);
        assert_eq!(
            said(&rt),
            (
                "Can't read",
                ChipTone::Attention,
                format!("Can't read it: {why}; the last colours stay.")
            )
        );
        assert_eq!(window(&rt), rgb(0x1e1e2e), "{why}: the chrome stays");
        assert!(told(&hosts[0]).is_empty(), "{why}: nothing is declared");
    }
    // Deleted later: the last good theme stays too.
    std::fs::remove_file(&file).unwrap();
    rt.update(|s, tx| s.read_theme_source(tx));
    assert_eq!(
        said(&rt),
        (
            "Not found",
            ChipTone::Attention,
            "Not found; the last colours stay.".to_owned()
        )
    );
    assert_eq!(window(&rt), rgb(0x1e1e2e));
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn until_the_file_is_read_once_the_palette_from_before_stays_and_launch_reads_it() {
    let (mut ws, hosts, file) = source_workspace(1);
    ws.set_chrome(ChromeTheme::GruvboxDark, &mut rune::Tx::default());
    // Leave the terminal theme unchosen, as it was before the palette: Follow server.
    ws.settings.terminal_theme = TerminalThemeChoice::FollowServer;
    ws.term_theme = None;
    declared(&hosts[0]);
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    rt.frame(1. / 60.);
    assert_eq!(window(&rt), rgb(0x282828), "still Gruvbox Dark");
    assert_eq!(
        said(&rt),
        (
            "Not found",
            ChipTone::Attention,
            "Not found; Gruvbox Dark stays until it can be read.".to_owned()
        )
    );
    assert_eq!(
        told(&hosts[0]),
        [serde_json::to_value(TerminalTheme::gruvbox_dark().args()).unwrap()],
        "Follow chrome follows the palette still in use"
    );
    std::fs::write(&file, "background = #000000\n").unwrap();
    rt.update(|s, tx| s.read_theme_source(tx));
    assert_eq!(
        said(&rt).2,
        "Can't read it: it has no foreground colour; Gruvbox Dark stays until it can be read."
    );
    assert_eq!(window(&rt), rgb(0x282828));

    // Written while the client was closed: launch reads it.
    std::fs::write(&file, DARK).unwrap();
    let again = relaunch(&rt.state);
    assert_eq!(again.palette.ground, 0x1e1e2e);
    assert_eq!(again.term_colors().background, 0x1e1e2e);
    assert_eq!(
        again
            .declared_theme()
            .map(|a| serde_json::to_value(a).unwrap()),
        Some(dark_args()),
        "what every host is told when it connects"
    );
    drop(again);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn picking_a_palette_leaves_theme_source_and_declares_its_paired_theme() {
    let (ws, hosts, file) = source_workspace(2);
    std::fs::write(&file, DARK).unwrap();
    let mut rt = interface(ws);
    click(&mut rt, "chrome-theme");
    choose_row(&mut rt, "Theme source");
    settle(&mut rt);
    hosts.iter().for_each(|h| drop(told(h)));
    click(&mut rt, "chrome-theme");
    assert!(
        rt.state
            .entries()
            .iter()
            .any(|e| e.label == "Theme source" && e.current)
    );
    choose_row(&mut rt, "Rosé Pine");
    assert_eq!(window(&rt), rgb(0x191724), "in one frame");
    assert_eq!(rt.state.settings.chrome_theme, ChromeTheme::RosePine);
    for host in &hosts {
        assert_eq!(
            told(host),
            [serde_json::to_value(TerminalTheme::rose_pine().args()).unwrap()]
        );
    }
    assert!(!rt.state.following_theme_source());
    // The file changing now changes nothing.
    std::fs::write(&file, LIGHT).unwrap();
    rt.update(|s, tx| s.read_theme_source(tx));
    rt.frame(1. / 60.);
    assert_eq!(window(&rt), rgb(0x191724));
    assert!(hosts.iter().all(|h| told(h).is_empty()));
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn the_row_says_found_applied_and_where_the_file_is() {
    let (ws, _hosts, file) = source_workspace(1);
    let mut rt = interface(ws);
    let row = |rt: &Runtime<Workspace>| element_text(rt, "theme-source").join(" ");
    assert_eq!(
        said(&rt),
        (
            "Not found",
            ChipTone::Off,
            "Not found. Nothing is there yet.".to_owned()
        )
    );
    assert!(row(&rt).contains("Not found"), "{}", row(&rt));
    let note = element_text(&rt, "theme-source-note").join(" ");
    assert!(
        note.contains("written by DMS, matugen or by hand"),
        "{note}"
    );
    assert_eq!(
        element_text(&rt, "theme-source-path"),
        [crate::app::downloads::tilde(&file)],
        "the path, as the file names it"
    );

    std::fs::write(&file, DARK).unwrap();
    // Settings looks again when it is next drawn (the watcher will wake it).
    rt.update(|_, _| {});
    settle(&mut rt);
    assert_eq!(
        said(&rt),
        (
            "Found",
            ChipTone::Good,
            "Found. Choose Theme source to follow it.".to_owned()
        ),
        "off, with the file there"
    );
    assert!(row(&rt).contains("Found"));

    click(&mut rt, "chrome-theme");
    choose_row(&mut rt, "Theme source");
    settle(&mut rt);
    let (word, tone, sentence) = said(&rt);
    assert_eq!((word, tone), ("Applied", ChipTone::Good));
    let clock = sentence
        .strip_prefix("Applied at ")
        .and_then(|rest| rest.strip_suffix('.'))
        .unwrap_or_else(|| panic!("{sentence}"));
    assert_eq!(clock.len(), 5, "the local hour and minute: {clock}");
    assert!(row(&rt).contains(&sentence), "{} / {sentence}", row(&rt));
    assert_eq!(
        element_text(&rt, "chrome-theme").join(""),
        "Theme source",
        "the Theme row names the choice"
    );
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn settings_search_finds_the_row_by_what_people_call_it() {
    let (ws, _hosts, _file) = source_workspace(1);
    let mut rt = interface(ws);
    for term in [
        "theme source",
        "follow",
        "dynamic",
        "wallpaper",
        "DMS",
        "matugen",
        "material",
    ] {
        rt.update(|s, _| s.settings_nav.search = rune::TextEdit::new(term));
        rt.update(|s, _| s.settings_search_changed());
        let hits: Vec<_> = rt
            .state
            .settings_nav
            .hits()
            .iter()
            .map(|h| (h.label.clone(), h.place.clone()))
            .collect();
        assert!(
            hits.contains(&("Theme source".to_owned(), "Interface".to_owned())),
            "{term}: {hits:?}"
        );
    }
}

#[test]
fn where_the_platform_cant_follow_a_file_the_entry_is_absent() {
    let (mut ws, _hosts, file) = source_workspace(1);
    std::fs::write(&file, DARK).unwrap();
    ws.theme_source.available = false;
    let mut rt = interface(ws);
    assert!(element_text(&rt, "theme-source").is_empty(), "no row");
    assert!(element_text(&rt, "theme-source-note").is_empty(), "no note");
    click(&mut rt, "chrome-theme");
    assert!(
        rt.state.entries().iter().all(|e| e.label != "Theme source"),
        "not in the list"
    );
    rt.update(|s, tx| s.follow_theme_source(tx));
    assert_eq!(rt.state.settings.chrome_theme, ChromeTheme::Midnight);
    assert_eq!(rt.state.palette.ground, 0x001528);
    rt.update(|s, _| s.settings_nav.search = rune::TextEdit::new("matugen"));
    rt.update(|s, _| s.settings_search_changed());
    assert!(rt.state.settings_nav.hits().is_empty());
}

/// Settings → Interface following a dark and a light theme source, one that can't be read,
/// and the palette list, at the usual and the minimum size. Real fonts are process-global:
/// run on its own with `REX_TEST_SNAP_DIR`.
#[test]
fn theme_source_settings_draw() {
    let Some(mut camera) = super::pane_verb_tests::Camera::new() else {
        return;
    };
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    for (suffix, size) in [("", (1024., 700.)), ("-minimum", (640., 480.))] {
        let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
        let (mut ws, _rx) = tabbed_workspace("split");
        let file = ws.test_folder.as_ref().unwrap().join("theme-source");
        ws.theme_source.path = file.clone();
        let (sender, _sent) = flume::unbounded();
        super::pane_verb_tests::framed_terminal(
            &mut ws,
            "block:b",
            &sender,
            b"$ cargo test\r\nrunning 12 tests",
        );
        std::fs::write(&file, DARK).unwrap();
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        ws.open_section(SettingsSection::Interface);
        let mut rt = Runtime::with_text(
            ws,
            view,
            size,
            Env::new(true, 1.),
            rune::text::TextSystem::new(fonts.files, fonts.families),
        );
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-found{suffix}"));
        click(&mut rt, "chrome-theme");
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-list{suffix}"));
        choose_row(&mut rt, "Theme source");
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-dark{suffix}"));
        std::fs::write(&file, LIGHT).unwrap();
        rt.update(|s, tx| s.read_theme_source(tx));
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-light{suffix}"));
        std::fs::write(&file, "foreground = #ffffff\n").unwrap();
        rt.update(|s, tx| s.read_theme_source(tx));
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-cant-read{suffix}"));
        rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
    }
}
