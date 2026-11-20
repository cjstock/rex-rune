//! Hue, lightness and saturation, as Python's `colorsys` computes them (the reference
//! derivation's arithmetic), for moving a colour's lightness or turning its hue.

/// A colour as (hue 0–1, lightness 0–1, saturation 0–1).
pub(super) fn from_rgb(c: u32) -> (f64, f64, f64) {
    let [r, g, b] = [16, 8, 0].map(|shift| ((c >> shift) & 0xff) as f64 / 255.);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.;
    if max == min {
        return (0., l, 0.);
    }
    let range = max - min;
    let s = if l <= 0.5 {
        range / (max + min)
    } else {
        range / (2. - max - min)
    };
    let (rc, gc, bc) = ((max - r) / range, (max - g) / range, (max - b) / range);
    let h = if r == max {
        bc - gc
    } else if g == max {
        2. + rc - bc
    } else {
        4. + gc - rc
    };
    ((h / 6.).rem_euclid(1.), l, s)
}

/// The colour with hue `h`, lightness `l` (clamped to 0–1) and saturation `s`. Channels round
/// half away from zero, where Python rounds half to even.
pub(super) fn to_rgb(h: f64, l: f64, s: f64) -> u32 {
    let l = l.clamp(0., 1.);
    let [r, g, b] = if s == 0. {
        [l; 3]
    } else {
        let m2 = if l <= 0.5 {
            l * (1. + s)
        } else {
            l + s - l * s
        };
        let m1 = 2. * l - m2;
        let v = |hue: f64| {
            let hue = hue.rem_euclid(1.);
            if hue < 1. / 6. {
                m1 + (m2 - m1) * hue * 6.
            } else if hue < 0.5 {
                m2
            } else if hue < 2. / 3. {
                m1 + (m2 - m1) * (2. / 3. - hue) * 6.
            } else {
                m1
            }
        };
        [v(h + 1. / 3.), v(h), v(h - 1. / 3.)]
    };
    [r, g, b]
        .into_iter()
        .fold(0, |rgb, x| (rgb << 8) | (x * 255.).round() as u32)
}

/// Hue in degrees, 0–360.
pub(super) fn hue(c: u32) -> f64 {
    from_rgb(c).0 * 360.
}
