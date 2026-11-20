//! Set up DMS on the Theme source row: when the button shows, the two files it writes, what the
//! row says after, and the template refreshed at launch. Every window has a private home,
//! XDG folders and tool search under its own folder (`theme_source::Follow::new`); a fake `dms`
//! there is only ever looked for, never run.
use super::*;
use crate::app::{panel::SettingsSection, settings_kit::ChipTone};
use rex_rune::{
    settings::{ChromeChoice, ChromeTheme},
    theme_source::{
        ThemeSource,
        dms::{HEADER, TEMPLATE},
    },
};
use std::path::{Path, PathBuf};

/// A window with no theme source path of its own, and a fake `dms` when `dms` is true.
fn dms_workspace(dms: bool) -> Workspace {
    let ws = workspace_hosts(1);
    if dms {
        install_fake_dms(&ws);
    }
    ws
}

fn install_fake_dms(ws: &Workspace) {
    let path = ws.theme_source.places.tools.path.clone().unwrap();
    let bin = PathBuf::from(path);
    assert!(
        bin.starts_with(ws.test_folder.as_ref().unwrap().path()),
        "the window's own folder"
    );
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("dms"), "#!/bin/sh\n# a test's fake dms\nexit 1\n").unwrap();
}

/// `ws`, following the theme source, as choosing it from the Theme list does.
fn following(mut ws: Workspace) -> Workspace {
    ws.perform(
        Intent::Chrome(ChromeChoice::ThemeSource),
        &mut rune::Tx::default(),
    );
    assert!(ws.following_theme_source());
    ws
}

/// The window's private home.
fn home(ws: &Workspace) -> PathBuf {
    ws.theme_source.places.home.clone()
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
}

/// Settings → Interface, at rest.
fn interface(mut ws: Workspace) -> Runtime<Workspace> {
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Interface);
    let mut rt = runtime_at(ws, (1280., 900.));
    settle(&mut rt);
    rt
}

fn button_shown(rt: &Runtime<Workspace>) -> bool {
    rt.elements()
        .iter()
        .any(|e| e.key.as_deref() == Some("set-up-dms"))
}

/// Draw Settings again, as it is when next shown.
fn redraw(rt: &mut Runtime<Workspace>) {
    rt.update(|_, _| {});
    settle(rt);
}

fn said(rt: &Runtime<Workspace>) -> (&'static str, ChipTone, String) {
    let said = rt.state.theme_source_said();
    (said.word, said.tone, said.sentence)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn leave(rt: &mut Runtime<Workspace>) {
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight.into()), tx));
}

const WAITING: (&str, ChipTone, &str) = (
    "Waiting",
    ChipTone::Off,
    "Waiting for DMS: change the wallpaper or switch light and dark.",
);

const ELSEWHERE: (&str, ChipTone, &str) = (
    "Other file",
    ChipTone::Attention,
    "DMS writes a different file; Set up DMS points it here.",
);

fn is(said: (&'static str, ChipTone, String), expected: (&str, ChipTone, &str)) -> bool {
    (said.0, said.1, said.2.as_str()) == expected
}

#[test]
fn the_button_shows_only_with_dms_and_a_drop_in_to_write() {
    let mut rt = interface(dms_workspace(false));
    assert!(
        element_text(&rt, "theme-source").len() > 1,
        "the row is there"
    );
    assert!(!button_shown(&rt), "no dms: no button");

    install_fake_dms(&rt.state);
    redraw(&mut rt);
    assert!(button_shown(&rt), "dms, and no drop-in yet");
    let drop_in = rt.state.theme_source.places.drop_in();
    assert!(!drop_in.exists(), "nothing written before the press");
    assert_eq!(
        element_text(&rt, "dms-path"),
        [crate::app::downloads::tilde(&drop_in)],
        "the footnote names the drop-in"
    );

    click(&mut rt, "set-up-dms");
    assert!(drop_in.is_file());
    redraw(&mut rt);
    assert!(!button_shown(&rt), "up to date: nothing to do");

    // Ours, out of date.
    let text = read(&drop_in);
    std::fs::write(
        &drop_in,
        text.replace("theme-source-template", "old-template"),
    )
    .unwrap();
    redraw(&mut rt);
    assert!(button_shown(&rt), "stale");

    // Someone else's file of that name is left alone.
    std::fs::write(&drop_in, "[templates.rex-rune]\n").unwrap();
    redraw(&mut rt);
    assert!(!button_shown(&rt), "not this client's");
}

#[test]
fn pressing_set_up_dms_writes_the_template_and_the_drop_in_and_waits_for_dms() {
    let ws = following(dms_workspace(true));
    let home = home(&ws);
    let mut rt = interface(ws);
    let source = rt.state.theme_source.path.clone();
    assert_eq!(source, home.join(".local/state/rex-rune/theme-source"));
    assert!(!source.exists());

    click(&mut rt, "set-up-dms");
    let template = home.join(".local/share/rex-rune/matugen/theme-source-template");
    assert_eq!(read(&template), TEMPLATE);
    let drop_in = read(&home.join(".config/matugen/dms/configs/rex-rune.toml"));
    assert!(drop_in.starts_with(HEADER), "{drop_in}");
    assert_eq!(drop_in.matches("[templates.").count(), 1, "{drop_in}");
    assert!(drop_in.contains("[templates.rex-rune]\n"), "{drop_in}");
    assert!(
        drop_in
            .contains("\ninput_path = '~/.local/share/rex-rune/matugen/theme-source-template'\n"),
        "{drop_in}"
    );
    assert!(
        drop_in.contains("\noutput_path = '~/.local/state/rex-rune/theme-source'\n"),
        "{drop_in}"
    );
    assert!(
        !home.join(".config/matugen/config.toml").exists(),
        "the user's own matugen config is never written"
    );
    assert!(
        source.parent().unwrap().is_dir(),
        "a folder DMS can write in"
    );
    assert!(is(said(&rt), WAITING), "{:?}", said(&rt));
    assert_eq!(
        element_text(&rt, "theme-source-state").join(""),
        "Waiting",
        "{:?}",
        element_text(&rt, "theme-source")
    );

    // DMS's first run writes the theme source: the watcher's wake applies it.
    std::fs::write(&source, "background = #141218\nforeground = #e6e0e9\n").unwrap();
    rt.update(|s, tx| s.read_theme_source(tx));
    settle(&mut rt);
    assert_eq!(said(&rt).0, "Applied");
    assert_eq!(rt.state.palette.ground, 0x141218);
    leave(&mut rt);
    crate::app::theme::set_palette(crate::app::theme::palette_of(ChromeTheme::Midnight));
}

#[test]
fn set_up_while_not_following_waits_until_the_file_is_there() {
    let mut rt = interface(dms_workspace(true));
    click(&mut rt, "set-up-dms");
    assert!(is(said(&rt), WAITING), "{:?}", said(&rt));
    std::fs::write(
        &rt.state.theme_source.path,
        "background = #141218\nforeground = #e6e0e9\n",
    )
    .unwrap();
    redraw(&mut rt);
    assert_eq!(said(&rt).0, "Found");
    assert_eq!(
        rt.state.settings.chrome_theme,
        ChromeTheme::Midnight,
        "setting up never chooses the entry"
    );
}

#[test]
fn a_new_path_in_settings_says_dms_writes_a_different_file_until_set_up_again() {
    let mut rt = interface(following(dms_workspace(true)));
    click(&mut rt, "set-up-dms");
    let drop_in_path = rt.state.theme_source.places.drop_in();
    let before = read(&drop_in_path);

    // The person points settings.json somewhere else, and launches again.
    let other = rt.state.test_folder.as_ref().unwrap().join("mine/theme");
    rt.state.settings.theme_source.path = Some(other.display().to_string());
    rt.state.save();
    let mut again = relaunch_owning(&mut rt.state);
    again.perform(Intent::Settings, &mut rune::Tx::default());
    again.open_section(SettingsSection::Interface);
    drop(rt);
    let mut rt = runtime_at(again, (1280., 900.));
    settle(&mut rt);
    assert_eq!(rt.state.theme_source.path, other);
    assert!(is(said(&rt), ELSEWHERE), "{:?}", said(&rt));
    assert_eq!(read(&drop_in_path), before, "nothing rewritten on its own");
    assert!(button_shown(&rt));

    click(&mut rt, "set-up-dms");
    let after = read(&drop_in_path);
    assert!(
        after.contains(&format!("\noutput_path = '{}'\n", other.display())),
        "absolute, outside home: {after}"
    );
    assert!(
        !other.parent().unwrap().exists(),
        "the person's folder is theirs to make"
    );
    assert!(is(said(&rt), WAITING), "{:?}", said(&rt));
    redraw(&mut rt);
    assert!(!button_shown(&rt));
    leave(&mut rt);
}

#[test]
fn launch_refreshes_the_template_this_client_wrote_and_leaves_the_drop_in() {
    let mut rt = interface(dms_workspace(true));
    click(&mut rt, "set-up-dms");
    let places = rt.state.theme_source.places.clone();
    let older = format!("{HEADER} long ago\nbackground = {{{{colors.background.default.hex}}}}\n");
    std::fs::write(places.template(), &older).unwrap();
    let stale = read(&places.drop_in()).replace("theme-source-template", "old-template");
    std::fs::write(places.drop_in(), &stale).unwrap();

    let again = relaunch_owning(&mut rt.state);
    assert_eq!(read(&places.template()), TEMPLATE, "refreshed");
    assert_eq!(
        read(&places.drop_in()),
        stale,
        "the drop-in only by the button"
    );
    drop(again);
}

#[test]
fn a_template_without_the_header_is_left_alone_at_launch() {
    let mut ws = dms_workspace(true);
    let template = ws.theme_source.places.template();
    std::fs::create_dir_all(template.parent().unwrap()).unwrap();
    std::fs::write(&template, "my own\n").unwrap();
    let again = relaunch_owning(&mut ws);
    assert_eq!(read(&template), "my own\n");
    drop(again);
}

#[test]
fn following_the_default_path_makes_its_folder_so_it_can_be_watched() {
    let ws = dms_workspace(false);
    let mut rt = runtime(ws);
    let source = rt.state.theme_source.path.clone();
    let folder = source.parent().unwrap().to_path_buf();
    assert!(!folder.exists());
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    assert!(folder.is_dir(), "the client's own state folder");
    assert_eq!(
        rt.state.theme_source.watching(),
        rex_rune::platform::watch::AVAILABLE.then_some(source.as_path()),
        "followed from the start, before the file exists"
    );
    leave(&mut rt);
}

#[test]
fn a_path_the_person_chose_gets_no_folder_made_for_it() {
    let mut ws = dms_workspace(false);
    let other = ws.test_folder.as_ref().unwrap().join("not-yet/theme");
    ws.settings.theme_source.path = Some(other.display().to_string());
    ws.save();
    let again = relaunch_owning(&mut ws);
    drop(ws);
    let mut rt = runtime(again);
    assert_eq!(rt.state.theme_source.path, other);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeChoice::ThemeSource), tx));
    assert!(!other.parent().unwrap().exists());
    assert_eq!(rt.state.theme_source.watching(), None);
    leave(&mut rt);
}

/// A DMS colour set for one mode: the Material roles the template names, and the two dank16
/// palettes. Material 3's baseline scheme (seed `#6750a4`), with dank16 colours of the kind
/// DMS makes for it.
struct Sample {
    mode: &'static str,
    roles: [(&'static str, u32); 9],
    dank16: [u32; 16],
    dank16term: [u32; 16],
}

const DARK: Sample = Sample {
    mode: "dark",
    roles: [
        ("background", 0x141218),
        ("surface", 0x141218),
        ("surface_container_high", 0x2b2930),
        ("on_surface", 0xe6e0e9),
        ("on_surface_variant", 0xcac4d0),
        ("primary", 0xd0bcff),
        ("on_primary", 0x381e72),
        ("primary_container", 0x4f378b),
        ("tertiary", 0xefb8c8),
    ],
    dank16: [
        0x141218, 0xf2697d, 0x8fd694, 0xf2d38a, 0xa8c7fa, 0xd0bcff, 0x8ed6e0, 0xe6e0e9, 0x5c5866,
        0xff8a9a, 0xa8e6ad, 0xffe2a3, 0xc2d7ff, 0xe3d5ff, 0xb0e8ef, 0xffffff,
    ],
    dank16term: [
        0x141218, 0xee6b7e, 0x8cd492, 0xefd08a, 0xa5c4f7, 0xcdb9fc, 0x8bd3dd, 0xe3dde6, 0x5a5664,
        0xfb8797, 0xa5e3aa, 0xfcdfa0, 0xbfd4fc, 0xe0d2fc, 0xade5ec, 0xfcfcfc,
    ],
};

const LIGHT: Sample = Sample {
    mode: "light",
    roles: [
        ("background", 0xfef7ff),
        ("surface", 0xfef7ff),
        ("surface_container_high", 0xece6f0),
        ("on_surface", 0x1d1b20),
        ("on_surface_variant", 0x49454f),
        ("primary", 0x6750a4),
        ("on_primary", 0xffffff),
        ("primary_container", 0xeaddff),
        ("tertiary", 0x7d5260),
    ],
    dank16: [
        0x1d1b20, 0xb3261e, 0x2e7d32, 0x8a5a00, 0x3f5f9f, 0x6750a4, 0x00696f, 0x49454f, 0x79747e,
        0xd32f2f, 0x388e3c, 0xa86f00, 0x4f6fb0, 0x7e67bb, 0x00838a, 0xfef7ff,
    ],
    dank16term: [
        0x1d1b20, 0xb02a22, 0x2b7a30, 0x875800, 0x3c5c9c, 0x644da1, 0x00666c, 0x46424c, 0x76717b,
        0xd02c2c, 0x358b39, 0xa56c00, 0x4c6cad, 0x7b64b8, 0x008087, 0xfbf4fc,
    ],
};

/// The template as matugen fills it for `sample`: each `{{…}}` replaced by its value, written
/// in matugen's `hex` format. A placeholder the sample doesn't know fails the test.
fn render(sample: &Sample) -> String {
    let hex = |c: u32| format!("#{c:06x}");
    let value = |name: &str| -> String {
        if name == "mode" {
            return sample.mode.into();
        }
        let (path, format) = name
            .rsplit_once(".default.")
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(format, "hex", "{name}");
        if let Some(role) = path.strip_prefix("colors.") {
            let (_, c) = sample
                .roles
                .iter()
                .find(|(r, _)| *r == role)
                .unwrap_or_else(|| panic!("{role}"));
            return hex(*c);
        }
        let (palette, n) = path
            .split_once(".color")
            .unwrap_or_else(|| panic!("{name}"));
        let n: usize = n.parse().unwrap();
        match palette {
            "dank16" => hex(sample.dank16[n]),
            "dank16term" => hex(sample.dank16term[n]),
            other => panic!("{other}"),
        }
    };
    let mut out = String::new();
    let mut rest = TEMPLATE;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("}}").unwrap() + start;
        out.push_str(&value(rest[start + 2..end].trim()));
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    out
}

#[test]
fn the_template_filled_with_dms_colours_is_a_legible_theme_source() {
    for sample in [DARK, LIGHT] {
        let text = render(&sample);
        assert!(!text.contains("{{") && !text.contains("<*"), "{text}");
        let source = ThemeSource::parse(&text).unwrap_or_else(|why| panic!("{why}: {text}"));
        let role = |name: &str| sample.roles.iter().find(|(r, _)| *r == name).unwrap().1;
        let t = &source.terminal;
        assert_eq!(t.scheme, Some(sample.mode));
        assert_eq!(t.background, Some(role("background")));
        assert_eq!(t.foreground, Some(role("on_surface")));
        assert_eq!(t.cursor, Some(role("primary")));
        assert_eq!(t.selection_background, Some(role("primary_container")));
        assert_eq!(t.selection_foreground, Some(role("on_surface")));
        assert_eq!(
            t.palette,
            (0..16u8)
                .map(|n| (n, sample.dank16term[n as usize]))
                .collect::<Vec<_>>(),
            "dank16term, as DMS's own Ghostty template"
        );
        let derived = source.derive();
        let c = &derived.chrome;
        assert_eq!(
            [
                c.ground,
                c.card,
                c.popover,
                c.ink,
                c.muted,
                c.accent,
                c.accent2,
                c.on_accent
            ],
            [
                role("surface"),
                role("surface"),
                role("surface_container_high"),
                role("on_surface"),
                role("on_surface_variant"),
                role("primary"),
                role("tertiary"),
                role("on_primary"),
            ],
            "{}: keyed, so used as written",
            sample.mode
        );
        assert_eq!(
            [c.danger, c.live, c.warning],
            [sample.dank16[1], sample.dank16[2], sample.dank16[3]]
        );
        let palette = crate::app::theme::Palette::from(c);
        assert_eq!(palette.is_light(), sample.mode == "light");
        crate::app::theme::tests::assert_legible(sample.mode, &palette);
    }
}

/// Settings → Interface with the Set up DMS button on the Theme source row, then waiting for
/// DMS, at the usual and the minimum size. Real fonts are process-global: run on its own with
/// `REX_TEST_SNAP_DIR`.
#[test]
fn dms_setup_settings_draw() {
    let Some(mut camera) = super::pane_verb_tests::Camera::new() else {
        return;
    };
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    for (suffix, size) in [("", (1024., 700.)), ("-minimum", (640., 480.))] {
        let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
        let mut ws = dms_workspace(true);
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
        camera.snap(&mut rt, &format!("dms-set-up{suffix}"));
        click(&mut rt, "set-up-dms");
        settle(&mut rt);
        camera.snap(&mut rt, &format!("dms-waiting{suffix}"));
    }
}
