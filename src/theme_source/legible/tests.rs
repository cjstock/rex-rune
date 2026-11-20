use super::*;

/// Midnight, the default built-in chrome palette.
const MIDNIGHT: ChromeColors = Chrome {
    ground: 0x001528,
    card: 0x001528,
    popover: 0x0d1524,
    ink: 0xe8eef6,
    muted: 0x93a3b8,
    accent: 0x7cc4ff,
    accent2: 0xa78bfa,
    live: 0x5eead4,
    warning: 0xf5d48a,
    danger: 0xff8a8a,
    on_accent: 0x061220,
    terminal_selection: 0x2f4a6e,
};

fn names(c: &ChromeColors) -> Vec<&'static str> {
    failures(c).into_iter().map(|f| f.check).collect()
}

#[test]
fn contrast_is_wcag_2s_ratio() {
    assert!((contrast(0xffffff, 0x000000) - 21.).abs() < 1e-9);
    assert!((contrast(0x777777, 0xffffff) - 4.478).abs() < 1e-3);
    assert_eq!(contrast(0x123456, 0x123456), 1.);
}

#[test]
fn a_legible_palette_has_no_failures() {
    assert_eq!(names(&MIDNIGHT), Vec::<&str>::new());
}

#[test]
fn each_check_names_what_failed() {
    let muted = Chrome {
        muted: 0x203040,
        ..MIDNIGHT
    };
    assert!(
        names(&muted).contains(&"muted on card"),
        "{:?}",
        names(&muted)
    );
    assert!(names(&muted).contains(&"keycaps on the sidebar"));
    let dim_danger = Chrome {
        danger: 0x401010,
        ..MIDNIGHT
    };
    assert_eq!(
        names(&dim_danger),
        [
            "destructive selection outline",
            "danger on card",
            "danger on popover"
        ]
    );
    let dim_status = Chrome {
        accent2: 0x202030,
        live: 0x102020,
        ..MIDNIGHT
    };
    assert_eq!(
        names(&dim_status),
        [
            "accent2 on card",
            "accent2 on popover",
            "live on card",
            "live on popover"
        ]
    );
}

#[test]
fn warning_and_danger_differ_in_hue_but_the_accent_may_sit_near_danger() {
    // Rosy orange warning 10° from the danger red.
    let close = Chrome {
        warning: 0xffa08a,
        ..MIDNIGHT
    };
    assert_eq!(names(&close), ["warning and danger differ in hue"]);
    // A generated palette's accent can share danger's hue family: there is no such check.
    let warm = Chrome {
        accent: 0xffa08a,
        ..MIDNIGHT
    };
    assert_eq!(names(&warm), Vec::<&str>::new());
}

#[test]
fn a_light_ground_draws_keycaps_with_ink_instead_of_white() {
    assert!(is_light_ground(0xffffff) && is_light_ground(0xfff4ee));
    assert!(!is_light_ground(0x001528) && !is_light_ground(0x707070));
    let paper = Chrome {
        ground: 0xffffff,
        card: 0xffffff,
        popover: 0xf6f8fa,
        ink: 0x1f2328,
        muted: 0x59636e,
        accent: 0x0969da,
        accent2: 0x8250df,
        live: 0x1a7f37,
        warning: 0x9a6700,
        danger: 0xcf222e,
        on_accent: 0xffffff,
        terminal_selection: 0xb6e3ff,
    };
    assert_eq!(names(&paper), Vec::<&str>::new());
    // A keycap's chip over the destructive tint is ink-tinted on a light ground, so it is darker
    // than a white one and grey ink no longer clears it.
    let grey = Chrome {
        ink: 0x585858,
        muted: 0x585858,
        ..paper
    };
    assert_eq!(names(&grey), ["keycaps on the destructive selection"]);
}
