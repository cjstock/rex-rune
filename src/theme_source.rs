//! The theme source: a file on this machine, written by DMS, matugen or by hand, that supplies
//! both a chrome palette and a terminal theme.
//!
//! The file is a Ghostty theme with a few more keys, read by the same parser
//! ([`crate::terminal_theme`]): `#` starts a comment, a later line wins and unknown keys are
//! ignored, so any Ghostty theme file is a theme source.
//!
//! - `background` and `foreground` are required.
//! - `scheme = dark|light` says which the theme is; without it the background decides.
//! - `rex-rune-theme-source = N` is the format's version. No version reads as 1, and a newer
//!   one is refused, so the format can grow without this client misreading it.
//! - `chrome-ground`, `chrome-card`, `chrome-popover`, `chrome-ink`, `chrome-muted`,
//!   `chrome-accent`, `chrome-accent2`, `chrome-live`, `chrome-warning`, `chrome-danger` and
//!   `chrome-on-accent` are chrome colours, used exactly as written. `chrome-card` defaults to
//!   the ground; every other slot left out takes its starting value from the terminal colours
//!   ([`ThemeSource::derive`]).
//!
//! Nothing here knows about the UI or about hosts: [`read`] turns a path into a chrome palette's
//! colours and a terminal theme, or says why it can't.
use crate::terminal_theme::{TerminalTheme, entries, is_light, parse_hex};
use legible::{KEYCAP_FILL, SIDEBAR_CHIP, TEXT, contrast, mix};
use std::path::{Path, PathBuf};

/// The newest format version this client reads.
pub const VERSION: u32 = 1;

/// The file's name under the state folder.
pub const FILE_NAME: &str = "theme-source";

/// Where the theme source is read from: `setting` (absolute, or starting with `~`) when given,
/// else `theme-source` in this client's state folder (`$XDG_STATE_HOME/rex-rune`).
pub fn path(setting: Option<&str>) -> PathBuf {
    dms::Places::from_env().theme_source(setting)
}

/// The chrome palette's slots. `Chrome<u32>` is a palette's colours; `Chrome<Option<u32>>` the
/// ones a file keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chrome<T> {
    pub ground: T,
    pub card: T,
    pub popover: T,
    pub ink: T,
    pub muted: T,
    pub accent: T,
    pub accent2: T,
    pub live: T,
    pub warning: T,
    pub danger: T,
    pub on_accent: T,
    /// Not keyed: it comes from `selection-background`, the file's one selection colour.
    pub terminal_selection: T,
}

/// A chrome palette's colours, `0xRRGGBB`.
pub type ChromeColors = Chrome<u32>;

impl<T> Chrome<T> {
    /// The slot a `chrome-*` key names.
    fn keyed(&mut self, key: &str) -> Option<&mut T> {
        Some(match key.strip_prefix("chrome-")? {
            "ground" => &mut self.ground,
            "card" => &mut self.card,
            "popover" => &mut self.popover,
            "ink" => &mut self.ink,
            "muted" => &mut self.muted,
            "accent" => &mut self.accent,
            "accent2" => &mut self.accent2,
            "live" => &mut self.live,
            "warning" => &mut self.warning,
            "danger" => &mut self.danger,
            "on-accent" => &mut self.on_accent,
            _ => return None,
        })
    }
}

/// A theme source as read: the terminal theme it declares and the chrome slots it keys.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSource {
    /// Its terminal theme, `scheme` always set: the file's own, else its background's.
    pub terminal: TerminalTheme,
    pub keyed: Chrome<Option<u32>>,
}

/// What a theme source gives: the chrome palette's colours and the terminal theme to declare.
#[derive(Clone, Debug, PartialEq)]
pub struct Derived {
    pub chrome: ChromeColors,
    pub terminal: TerminalTheme,
}

/// One reading of the theme source.
#[derive(Clone, Debug, PartialEq)]
pub enum Read {
    /// There is no file at the path.
    Missing,
    /// There is a file, but no theme in it; why, in a few words.
    Unreadable(String),
    Theme(Derived),
}

/// Read the theme source at `path` and derive what it gives.
pub fn read(path: &Path) -> Read {
    match std::fs::read(path) {
        Ok(bytes) => decode(bytes),
        Err(error) => unread(&error),
    }
}

/// What a theme source that could not be read gives: for a follower that reads the bytes
/// itself, to compare them before deriving them ([`decode`]).
pub fn unread(error: &std::io::Error) -> Read {
    match error.kind() {
        std::io::ErrorKind::NotFound => Read::Missing,
        kind => Read::Unreadable(kind.to_string()),
    }
}

/// What a theme source's bytes give.
pub fn decode(bytes: Vec<u8>) -> Read {
    match String::from_utf8(bytes) {
        Ok(text) => derive_text(&text),
        Err(_) => Read::Unreadable("it isn't text".into()),
    }
}

fn derive_text(text: &str) -> Read {
    match ThemeSource::parse(text) {
        Ok(source) => Read::Theme(source.derive()),
        Err(reason) => Read::Unreadable(reason),
    }
}

impl ThemeSource {
    /// A theme source's text, or why it isn't one.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut keyed = Chrome::<Option<u32>>::default();
        let (mut scheme, mut version) = (None, None);
        for (key, value) in entries(text) {
            match key {
                "scheme" => scheme = Some(value),
                "rex-rune-theme-source" => version = Some(value),
                _ => {
                    if let Some(slot) = keyed.keyed(key)
                        && let Some(color) = parse_hex(value)
                    {
                        *slot = Some(color);
                    }
                }
            }
        }
        if let Some(version) = version {
            match version.parse::<u32>() {
                Ok(v) if v <= VERSION => {}
                Ok(v) => return Err(format!("version {v} is newer than this client reads")),
                Err(_) => return Err(format!("version “{version}” is not a number")),
            }
        }
        let mut terminal =
            TerminalTheme::parse("theme source", text).map_err(|_| "it has no theme".to_owned())?;
        let background = terminal.background.ok_or("it has no background colour")?;
        terminal.foreground.ok_or("it has no foreground colour")?;
        terminal.scheme = Some(match scheme {
            Some("light") => "light",
            Some("dark") => "dark",
            _ if is_light(background) => "light",
            _ => "dark",
        });
        Ok(Self { terminal, keyed })
    }

    /// The chrome palette and terminal theme this source gives. A keyed slot is used as
    /// written, never adjusted and never checked; every other slot takes its starting value
    /// from the terminal colours:
    ///
    /// | Slot | Starting value |
    /// | --- | --- |
    /// | ground | `background` |
    /// | card | the ground |
    /// | ink | `foreground` |
    /// | popover | ink at 7% over the ground |
    /// | muted | the least ink-over-ground mix, from 55% up, that is 4.5:1 on the card and the popover and leaves the sidebar's keycaps 4.5:1 |
    /// | accent, accent2, live, warning, danger | blue, magenta, green, yellow, red: of palette N and N + 8, the one that stands out more from the popover |
    /// | on_accent | the ground if it is 4.5:1 on the accent, else ink, else black or white |
    /// | terminal selection | `selection-background`, else accent at 30% over the ground |
    ///
    /// Palette entries the file leaves out read as Ghostty's default 16 colours. The cursor
    /// colour is never used.
    ///
    /// The derived slots are then adjusted until the palette passes [`legible::failures`],
    /// stopping at the first palette that does:
    ///
    /// 1. A failing accent, accent2, live, warning or danger moves its lightness away from the
    ///    ground, 2% at a time, keeping its hue, until every contrast check on it passes.
    /// 2. A warning less than 30° from danger turns its hue away from danger, just far enough,
    ///    then moves as in step 1.
    /// 3. A grey (monochrome) danger or warning takes a canonical red or amber instead, then
    ///    moves as in step 1.
    /// 4. Ink's lightness lifts away from the ground 2% at a time (up to 25 steps); popover and
    ///    muted follow it and every slot is derived again.
    /// 5. As a last resort the chrome ground steps 5% toward black, or white for a light theme
    ///    (up to 15 steps), and everything is derived again. The terminal theme keeps its own
    ///    background, so the panes keep theirs.
    ///
    /// Within one pass, grey trouble colours are replaced before the warning turns, as in the
    /// reference derivation: a canonical red can bring a theme's own warning within 30° of it.
    /// Derived slots adjust around keyed ones (warning turns away from a keyed danger), and a
    /// check that reads only keyed slots never holds the rest back. If nothing passes, the
    /// palette is the unadjusted one: never a built-in.
    pub fn derive(&self) -> Derived {
        let keyed = legible::keyed_slots(&self.keyed);
        let chrome = (0..=GROUND_STEPS)
            .flat_map(|ground| (0..=INK_STEPS).map(move |ink| (ink, ground)))
            .map(|(ink, ground)| self.attempt(ink, ground))
            .find(|chrome| legible::passes_around(chrome, keyed))
            .unwrap_or_else(|| self.attempt(0, 0));
        Derived {
            chrome,
            terminal: self.terminal.clone(),
        }
    }

    /// The palette with ink lifted `lift` steps and the ground moved `deepen` steps, its
    /// status colours adjusted (steps 1–3).
    fn attempt(&self, lift: u32, deepen: u32) -> ChromeColors {
        let t = &self.terminal;
        let k = &self.keyed;
        let light = t.scheme == Some("light");
        let background = t.background.unwrap_or_default();
        let ground = k.ground.unwrap_or_else(|| {
            let toward = if light { 0xffffff } else { 0 };
            mix(toward, background, GROUND_STEP * deepen as f32)
        });
        let card = k.card.unwrap_or(ground);
        let ink = k.ink.unwrap_or_else(|| {
            let foreground = t.foreground.unwrap_or_default();
            if lift == 0 {
                return foreground;
            }
            let (h, l, s) = hls::from_rgb(foreground);
            hls::to_rgb(h, l + away(ground) * LIGHTNESS_STEP * lift as f64, s)
        });
        let popover = k.popover.unwrap_or_else(|| mix(ink, ground, 0.07));
        let mut c = Chrome {
            ground,
            card,
            popover,
            ink,
            ..Chrome::default()
        };
        let sidebar_chip = legible::overlay(&c, KEYCAP_FILL + SIDEBAR_CHIP, card);
        c.muted = k.muted.unwrap_or_else(|| {
            let passes = |m: u32| {
                contrast(m, card).min(contrast(m, popover)) >= TEXT
                    && contrast(mix(ink, m, 0.5), sidebar_chip) >= TEXT
            };
            (55..=100)
                .map(|percent| mix(ink, ground, percent as f32 / 100.))
                .find(|&m| passes(m))
                .unwrap_or(ink)
        });
        let palette = |index: u8| {
            t.palette
                .iter()
                .find(|&&(i, _)| i == index)
                .map_or(GHOSTTY_16[index as usize], |&(_, color)| color)
        };
        // Step 1: of the pair, the one that stands out more from the popover, moved away from
        // the ground until every contrast check on its slot passes.
        for (slot, keyed, normal) in [
            (Slot::Accent, k.accent, 4),
            (Slot::Accent2, k.accent2, 5),
            (Slot::Live, k.live, 2),
            (Slot::Danger, k.danger, 1),
            (Slot::Warning, k.warning, 3),
        ] {
            *slot.of(&mut c) = keyed.unwrap_or_else(|| {
                let (a, b) = (palette(normal), palette(normal + 8));
                let base = if contrast(a, popover) >= contrast(b, popover) {
                    a
                } else {
                    b
                };
                lighten(&c, slot, base).unwrap_or(base)
            });
        }
        // Step 3: a grey has no hue to carry trouble.
        let canonical = if light {
            [
                (Slot::Danger, k.danger, 0xc62828),
                (Slot::Warning, k.warning, 0x9a6700),
            ]
        } else {
            [
                (Slot::Danger, k.danger, 0xff8a8a),
                (Slot::Warning, k.warning, 0xf5d48a),
            ]
        };
        for (slot, keyed, color) in canonical {
            if keyed.is_none() && hls::from_rgb(*slot.of(&mut c)).2 < GREY {
                *slot.of(&mut c) = lighten(&c, slot, color).unwrap_or(color);
            }
        }
        // Step 2: warning turns away from danger's hue until they are far enough apart.
        if k.warning.is_none() && legible::hue_gap(c.warning, c.danger) < legible::HUE_GAP {
            c.warning = turn_from_danger(&c);
        }
        let accent = c.accent;
        c.on_accent = k.on_accent.unwrap_or_else(|| {
            [ground, ink]
                .into_iter()
                .find(|&color| contrast(color, accent) >= TEXT)
                .unwrap_or(if contrast(0, accent) >= contrast(0xffffff, accent) {
                    0
                } else {
                    0xffffff
                })
        });
        c.terminal_selection = t
            .selection_background
            .unwrap_or_else(|| mix(accent, ground, 0.3));
        c
    }
}

/// How far one step moves a lightness (out of 1), and how many steps a colour may take.
const LIGHTNESS_STEP: f64 = 0.02;
const LIGHTNESS_STEPS: u32 = 50;
/// How many steps ink may lift.
const INK_STEPS: u32 = 25;
/// How far toward black (or white) one step moves the chrome ground, and how many it may take.
const GROUND_STEP: f32 = 0.05;
const GROUND_STEPS: u32 = 15;
/// HLS saturation below which a trouble colour counts as grey.
const GREY: f64 = 0.2;

/// Which way is away from `ground` in lightness: up from a dark ground, down from a light one.
fn away(ground: u32) -> f64 {
    if is_light(ground) { -1. } else { 1. }
}

/// A status slot the derivation adjusts.
#[derive(Clone, Copy)]
enum Slot {
    Accent,
    Accent2,
    Live,
    Warning,
    Danger,
}

impl Slot {
    fn of(self, c: &mut ChromeColors) -> &mut u32 {
        match self {
            Slot::Accent => &mut c.accent,
            Slot::Accent2 => &mut c.accent2,
            Slot::Live => &mut c.live,
            Slot::Warning => &mut c.warning,
            Slot::Danger => &mut c.danger,
        }
    }

    fn bit(self) -> u16 {
        use legible::slot::*;
        match self {
            Slot::Accent => ACCENT,
            Slot::Accent2 => ACCENT2,
            Slot::Live => LIVE,
            Slot::Warning => WARNING,
            Slot::Danger => DANGER,
        }
    }
}

/// `color` with its lightness moved away from `c`'s ground, 2% at a time with hue and
/// saturation kept, to the first lightness at which every contrast check on `slot` passes;
/// `None` if none does.
fn lighten(c: &ChromeColors, slot: Slot, color: u32) -> Option<u32> {
    let (h, l, s) = hls::from_rgb(color);
    let away = away(c.ground);
    (0..=LIGHTNESS_STEPS)
        .map(|i| hls::to_rgb(h, l + away * LIGHTNESS_STEP * i as f64, s))
        .find(|&candidate| {
            let mut c = *c;
            *slot.of(&mut c) = candidate;
            legible::slot_passes(&c, slot.bit())
        })
}

/// `c`'s warning with its hue turned away from danger's, a degree at a time, until they are
/// [`legible::HUE_GAP`] apart, then lightened as in step 1.
fn turn_from_danger(c: &ChromeColors) -> u32 {
    let (h, l, s) = hls::from_rgb(c.warning);
    let danger = hls::from_rgb(c.danger).0;
    let turn = if (h - danger).rem_euclid(1.) < 0.5 {
        1.
    } else {
        -1.
    };
    (1..60)
        .map(|degree| hls::to_rgb((h + turn * degree as f64 / 360.).rem_euclid(1.), l, s))
        .find(|&turned| legible::hue_gap(turned, c.danger) >= legible::HUE_GAP)
        .map_or(c.warning, |turned| {
            lighten(c, Slot::Warning, turned).unwrap_or(turned)
        })
}

/// Ghostty's default palette colours 0–15, for entries a file leaves out.
const GHOSTTY_16: [u32; 16] = [
    0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x666666,
    0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea,
];

mod hls;
pub mod legible;

pub mod dms;
mod remembered;
pub use remembered::Remembered;

#[cfg(test)]
mod tests;
