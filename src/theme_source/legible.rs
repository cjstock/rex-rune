//! The legibility checks every chrome palette passes: the built-ins, and every palette a theme
//! source derives. One definition, used by the derivation to adjust the slots it makes and by
//! the app's theme tests to hold the built-ins to the same bar.
//!
//! There is no accent/danger hue check, so a generated palette whose accent shares danger's
//! hue family passes: destructive rows always carry words. Warning and danger must differ.
use super::{Chrome, ChromeColors};

/// How much of its tone a selected row's fill is; the chrome's `theme::SELECTION_FILL`.
pub const SELECTION_FILL: f32 = 0.18;
/// A keycap's chip: an overlay at this opacity over its surface, and `KEYCAP_LIT` on a
/// selected row.
pub const KEYCAP_FILL: f32 = 0.07;
pub const KEYCAP_LIT: f32 = 0.12;
/// The sidebar's search chip, which a keycap sits on there: an overlay at this opacity over the
/// card.
pub const SIDEBAR_CHIP: f32 = 0.08;

/// Text on its surface (WCAG AA for normal text).
pub const TEXT: f64 = 4.5;
/// Outlines and coloured words that always carry a label beside them.
pub const MARK: f64 = 3.;
/// How far apart warning's and danger's hues are, in degrees.
pub const HUE_GAP: f64 = 30.;

/// A check a palette fails: which, what it reached (a contrast ratio, or degrees for the hue
/// check) and what it needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub check: &'static str,
    pub got: f64,
    pub needs: f64,
}

/// Every check `c` fails, in a fixed order; none when it is legible.
pub fn failures(c: &ChromeColors) -> Vec<Failure> {
    checks(c)
        .into_iter()
        .filter(Check::fails)
        .map(|check| Failure {
            check: check.name,
            got: check.got,
            needs: check.needs,
        })
        .collect()
}

/// Whether overlays on this ground darken it instead of lightening it: relative luminance
/// above 0.18. Then a "white" overlay is ink at 90% of its opacity, and shadows are softer.
pub fn is_light_ground(ground: u32) -> bool {
    luminance(ground) > 0.18
}

/// `a` over `b` at opacity `t`, mixed in sRGB as the chrome mixes: each channel rounds half
/// away from zero.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x * t + y * (1. - t)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// WCAG 2 contrast ratio of two sRGB colours.
pub fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The angle between two colours' hues, in degrees (0–180). A grey's hue is 0.
pub fn hue_gap(a: u32, b: u32) -> f64 {
    let gap = (super::hls::hue(a) - super::hls::hue(b)).abs();
    gap.min(360. - gap)
}

/// WCAG 2 relative luminance.
fn luminance(c: u32) -> f64 {
    let channel = |shift: u32| {
        let v = ((c >> shift) & 0xff) as f64 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// What a light overlay at `a` over `under` comes to: white on a dark ground, ink at 90% of
/// `a` on a light one (the chrome's `Palette::white`).
pub(super) fn overlay(c: &ChromeColors, a: f32, under: u32) -> u32 {
    if is_light_ground(c.ground) {
        mix(c.ink, under, a * 0.9)
    } else {
        mix(0xffffff, under, a)
    }
}

/// The slots a check reads, as bits, so the derivation can tell the checks it can still fix
/// from the ones that only read keyed slots.
pub(super) mod slot {
    pub const CARD: u16 = 1 << 1;
    pub const POPOVER: u16 = 1 << 2;
    pub const INK: u16 = 1 << 3;
    pub const MUTED: u16 = 1 << 4;
    pub const ACCENT: u16 = 1 << 5;
    pub const ACCENT2: u16 = 1 << 6;
    pub const LIVE: u16 = 1 << 7;
    pub const WARNING: u16 = 1 << 8;
    pub const DANGER: u16 = 1 << 9;
}

/// The slots a file keys, as [`slot`] bits.
pub(super) fn keyed_slots(keyed: &Chrome<Option<u32>>) -> u16 {
    [
        (keyed.card, slot::CARD),
        (keyed.popover, slot::POPOVER),
        (keyed.ink, slot::INK),
        (keyed.muted, slot::MUTED),
        (keyed.accent, slot::ACCENT),
        (keyed.accent2, slot::ACCENT2),
        (keyed.live, slot::LIVE),
        (keyed.warning, slot::WARNING),
        (keyed.danger, slot::DANGER),
    ]
    .into_iter()
    .filter(|(color, _)| color.is_some())
    .fold(0, |bits, (_, bit)| bits | bit)
}

/// Whether every check that reads a slot `c` doesn't key passes. A check that reads only
/// keyed slots is the file author's choice and is never held against the derivation.
pub(super) fn passes_around(c: &ChromeColors, keyed: u16) -> bool {
    checks(c)
        .iter()
        .all(|check| !check.fails() || check.slots & !keyed == 0)
}

/// Whether every contrast check that reads `slot` passes: the bar a status colour's lightness
/// is moved to. The hue check is left to its own step.
pub(super) fn slot_passes(c: &ChromeColors, slot: u16) -> bool {
    checks(c)
        .iter()
        .filter(|check| check.slots & slot != 0 && !check.hue)
        .all(|check| !check.fails())
}

struct Check {
    name: &'static str,
    got: f64,
    needs: f64,
    /// The [`slot`] bits it reads.
    slots: u16,
    hue: bool,
}

impl Check {
    fn fails(&self) -> bool {
        self.got < self.needs
    }
}

fn checks(c: &ChromeColors) -> Vec<Check> {
    use slot::*;
    let ratio = |name, a: u32, b: u32, needs, slots| Check {
        name,
        got: contrast(a, b),
        needs,
        slots,
        hue: false,
    };
    let mut out = vec![
        ratio("muted on card", c.muted, c.card, TEXT, MUTED | CARD),
        ratio("ink on popover", c.ink, c.popover, TEXT, INK | POPOVER),
    ];
    // Keycaps: on a popover's rows, and on the sidebar's search chip (itself an overlay over
    // the card).
    let keys = mix(c.ink, c.muted, 0.5);
    for (name, under, lift, slots) in [
        ("keycaps on the popover", c.popover, 0., POPOVER),
        ("keycaps on the sidebar", c.card, SIDEBAR_CHIP, CARD),
    ] {
        let chip = overlay(c, KEYCAP_FILL + lift, under);
        out.push(ratio(name, keys, chip, TEXT, INK | MUTED | slots));
    }
    // A selected row keeps its label and its keycaps in ink on either tint, and its outline
    // stands clear of the popover around it.
    for (names, tone, slot) in [
        (
            [
                "ink on the selection",
                "keycaps on the selection",
                "selection outline",
            ],
            c.accent,
            ACCENT,
        ),
        (
            [
                "ink on the destructive selection",
                "keycaps on the destructive selection",
                "destructive selection outline",
            ],
            c.danger,
            DANGER,
        ),
    ] {
        let fill = mix(tone, c.popover, SELECTION_FILL);
        let chip = overlay(c, KEYCAP_LIT, fill);
        out.extend([
            ratio(names[0], c.ink, fill, TEXT, INK | POPOVER | slot),
            ratio(names[1], c.ink, chip, TEXT, INK | POPOVER | slot),
            ratio(names[2], tone, c.popover, MARK, POPOVER | slot),
        ]);
    }
    // Status colours on both surfaces. Trouble always carries a word, and the others mark
    // something labelled; Gruvbox's brightest red is 4.3:1 on its ground.
    for (names, color, slot) in [
        (
            ["accent2 on card", "accent2 on popover"],
            c.accent2,
            ACCENT2,
        ),
        (["live on card", "live on popover"], c.live, LIVE),
        (
            ["warning on card", "warning on popover"],
            c.warning,
            WARNING,
        ),
        (["danger on card", "danger on popover"], c.danger, DANGER),
    ] {
        out.extend([
            ratio(names[0], color, c.card, MARK, CARD | slot),
            ratio(names[1], color, c.popover, MARK, POPOVER | slot),
        ]);
    }
    out.push(Check {
        name: "warning and danger differ in hue",
        got: hue_gap(c.warning, c.danger),
        needs: HUE_GAP,
        slots: WARNING | DANGER,
        hue: true,
    });
    out
}

#[cfg(test)]
mod tests;
