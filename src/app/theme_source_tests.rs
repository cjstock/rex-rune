//! Theme source: choosing it, reading the file into the chrome and the declared terminal
//! theme, leaving it, and what Settings says about it. Each test writes a real file in its own
//! folder and delivers the file watcher's wake (`read_theme_source`) once the file has settled.
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

    // The watcher's wake: a light file, light chrome, in one frame.
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
        write_settled(&file, text);
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
    write_settled(&file, "background = #000000\n");
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
    // Settings looks again when it is next drawn.
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
        write_settled(&file, "foreground = #ffffff\n");
        rt.update(|s, tx| s.read_theme_source(tx));
        settle(&mut rt);
        camera.snap(&mut rt, &format!("theme-source-cant-read{suffix}"));
        rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
    }
}

/// Make `file`'s last change older than the watcher's settle window, as it is by the time a
/// wake reads it: nothing is still writing it.
fn settled(file: &std::path::Path) {
    let then = std::time::SystemTime::now() - std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(then)
        .unwrap();
}

/// Write `text` and let it settle.
fn write_settled(file: &std::path::Path, text: &str) {
    std::fs::write(file, text).unwrap();
    settled(file);
}

/// One window following `DARK`, with what choosing it declared already taken.
fn following_dark() -> (
    Runtime<Workspace>,
    flume::Receiver<HostCommand>,
    std::path::PathBuf,
) {
    let (ws, mut hosts, file) = source_workspace(1);
    write_settled(&file, DARK);
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    rt.frame(1. / 60.);
    let host = hosts.remove(0);
    assert_eq!(told(&host), [dark_args()]);
    (rt, host, file)
}

/// The watcher's wake as it arrives on the UI thread, then one frame.
fn wake(rt: &mut Runtime<Workspace>) {
    rt.update(|s, tx| s.read_theme_source(tx));
    rt.frame(1. / 60.);
}

/// The notices that speak of the theme source.
fn source_notices(rt: &Runtime<Workspace>) -> Vec<String> {
    rt.state
        .notices
        .iter()
        .filter(|n| n.text.contains("theme source"))
        .map(|n| n.text.clone())
        .collect()
}

fn leave(rt: &mut Runtime<Workspace>) {
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

#[test]
fn a_wake_after_a_change_applies_it_in_one_frame() {
    let (mut rt, host, file) = following_dark();
    write_settled(&file, LIGHT);
    wake(&mut rt);
    assert_eq!(window(&rt), rgb(0xfafafa), "in one frame, without a spring");
    assert_eq!(told(&host).len(), 1, "declared once");
    assert_eq!(rt.state.term_colors().background, 0xfafafa);
    leave(&mut rt);
}

#[test]
fn a_wake_with_the_same_contents_does_nothing_and_declares_nothing() {
    let (mut rt, host, file) = following_dark();
    let before = rt.state.theme_source.status.clone();
    // Rewritten with the same bytes, as matugen does on every run.
    write_settled(&file, DARK);
    wake(&mut rt);
    assert!(told(&host).is_empty(), "no theme command");
    assert_eq!(
        rt.state.theme_source.status, before,
        "not applied again: the time it was applied stays"
    );
    leave(&mut rt);
}

#[test]
fn an_empty_or_partial_file_still_being_written_is_not_yet() {
    let (mut rt, host, file) = following_dark();
    let before = rt.state.theme_source.status.clone();
    for text in ["", "rex-rune-theme-source = 1\nbackground = #1e1e2e\n"] {
        // Changed just now: the writer may not be done.
        std::fs::write(&file, text).unwrap();
        wake(&mut rt);
        assert_eq!(
            rt.state.theme_source.status, before,
            "{text:?}: not reported"
        );
        assert!(source_notices(&rt).is_empty(), "{text:?}: no notice");
        assert!(told(&host).is_empty());
        assert_eq!(window(&rt), rgb(0x1e1e2e));
    }
    // Still partial once it has settled: now it is broken.
    settled(&file);
    wake(&mut rt);
    assert_eq!(said(&rt).0, "Can't read");
    assert_eq!(source_notices(&rt).len(), 1);
    leave(&mut rt);
}

#[test]
fn going_bad_says_so_once_and_recovering_says_nothing() {
    let (mut rt, host, file) = following_dark();
    write_settled(&file, "foreground = #ffffff\n");
    wake(&mut rt);
    assert_eq!(
        source_notices(&rt),
        ["Can't read the theme source: it has no background colour; the last colours stay"]
    );
    assert_eq!(window(&rt), rgb(0x1e1e2e), "the last good theme stays");
    assert!(told(&host).is_empty());

    // Still bad, differently: no second notice.
    write_settled(&file, "this is not a theme\n");
    wake(&mut rt);
    assert_eq!(
        said(&rt).2,
        "Can't read it: it has no theme; the last colours stay."
    );
    assert_eq!(source_notices(&rt).len(), 1, "{:?}", source_notices(&rt));

    // Good again: applied without a word, and the notice is gone.
    write_settled(&file, LIGHT);
    wake(&mut rt);
    assert_eq!(window(&rt), rgb(0xfafafa));
    assert_eq!(told(&host).len(), 1);
    assert!(source_notices(&rt).is_empty(), "{:?}", source_notices(&rt));

    // Bad again after good: one notice again.
    write_settled(&file, "background = #000000\n");
    wake(&mut rt);
    assert_eq!(source_notices(&rt).len(), 1);
    leave(&mut rt);
}

#[test]
fn deleting_the_file_keeps_the_last_good_theme() {
    let (mut rt, host, file) = following_dark();
    std::fs::remove_file(&file).unwrap();
    wake(&mut rt);
    assert_eq!(window(&rt), rgb(0x1e1e2e));
    assert_eq!(rt.state.term_colors().background, 0x1e1e2e);
    assert!(told(&host).is_empty(), "nothing declared");
    assert_eq!(said(&rt).2, "Not found; the last colours stay.");
    assert_eq!(
        source_notices(&rt),
        ["The theme source is gone; the last colours stay"]
    );
    // Back with the same colours: nothing to declare, nothing said.
    write_settled(&file, DARK);
    wake(&mut rt);
    assert!(told(&host).is_empty());
    assert_eq!(said(&rt).0, "Applied");
    assert!(source_notices(&rt).is_empty());
    leave(&mut rt);
}

#[test]
fn the_file_is_watched_only_while_following_it() {
    let (ws, _hosts, file) = source_workspace(1);
    write_settled(&file, DARK);
    let mut rt = runtime(ws);
    assert_eq!(rt.state.theme_source.watching(), None, "not following");
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    let expected = rex_rune::platform::watch::AVAILABLE.then_some(file.as_path());
    assert_eq!(rt.state.theme_source.watching(), expected);
    let again = relaunch(&rt.state);
    assert_eq!(again.theme_source.watching(), expected, "from launch");
    drop(again);
    leave(&mut rt);
    assert_eq!(rt.state.theme_source.watching(), None, "left");
}
