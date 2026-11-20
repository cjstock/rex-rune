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
use std::path::{Path, PathBuf};

/// The newest format version this client reads.
pub const VERSION: u32 = 1;

/// The file's name under the state folder.
pub const FILE_NAME: &str = "theme-source";

/// Where the theme source is read from: `setting` (absolute, or starting with `~`) when given,
/// else `theme-source` in this client's state folder (`$XDG_STATE_HOME/rex-rune`).
pub fn path(setting: Option<&str>) -> PathBuf {
    match setting.map(str::trim).filter(|path| !path.is_empty()) {
        Some(path) => crate::download::store::expand_home(path),
        None => crate::platform::dirs::state().join(FILE_NAME),
    }
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
    let Ok(text) = String::from_utf8(bytes) else {
        return Read::Unreadable("it isn't text".into());
    };
    match ThemeSource::parse(&text) {
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
    /// written; every other slot takes its starting value from the terminal colours:
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
    pub fn derive(&self) -> Derived {
        let t = &self.terminal;
        let k = &self.keyed;
        let light = t.scheme == Some("light");
        let ground = k.ground.or(t.background).unwrap_or_default();
        let card = k.card.unwrap_or(ground);
        let ink = k.ink.or(t.foreground).unwrap_or_default();
        let popover = k.popover.unwrap_or_else(|| mix(ink, ground, 0.07));
        // The light overlay a keycap's chip is drawn with: white on dark, ink at 90% on light.
        let (over, strength) = if light { (ink, 0.9) } else { (0xffffff, 1.) };
        let muted = k.muted.unwrap_or_else(|| {
            let passes = |m: u32| {
                let keys = mix(ink, m, 0.5);
                let sidebar_chip = mix(over, card, (KEYCAP_FILL + 0.08) * strength);
                contrast(m, card).min(contrast(m, popover)) >= 4.5
                    && contrast(keys, sidebar_chip) >= 4.5
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
        let pick = |keyed: Option<u32>, normal: u8| {
            keyed.unwrap_or_else(|| {
                let (a, b) = (palette(normal), palette(normal + 8));
                if contrast(a, popover) >= contrast(b, popover) {
                    a
                } else {
                    b
                }
            })
        };
        let accent = pick(k.accent, 4);
        let on_accent = k.on_accent.unwrap_or_else(|| {
            [ground, ink]
                .into_iter()
                .find(|&c| contrast(c, accent) >= 4.5)
                .unwrap_or(if contrast(0, accent) >= contrast(0xffffff, accent) {
                    0
                } else {
                    0xffffff
                })
        });
        Derived {
            chrome: Chrome {
                ground,
                card,
                popover,
                ink,
                muted,
                accent,
                accent2: pick(k.accent2, 5),
                live: pick(k.live, 2),
                warning: pick(k.warning, 3),
                danger: pick(k.danger, 1),
                on_accent,
                terminal_selection: t
                    .selection_background
                    .unwrap_or_else(|| mix(accent, ground, 0.3)),
            },
            terminal: t.clone(),
        }
    }
}

/// A keycap chip's fill over its surface; the chrome's `theme::KEYCAP_FILL`.
const KEYCAP_FILL: f32 = 0.07;

/// Ghostty's default palette colours 0–15, for entries a file leaves out.
const GHOSTTY_16: [u32; 16] = [
    0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x666666,
    0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea,
];

/// `a` over `b` at opacity `t`, mixed in sRGB as the chrome mixes.
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x * t + y * (1. - t)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// WCAG 2 contrast ratio of two sRGB colours.
fn contrast(a: u32, b: u32) -> f64 {
    let luminance = |c: u32| {
        let channel = |shift: u32| {
            let v = ((c >> shift) & 0xff) as f64 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    };
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests;
