//! Following the theme source: reading the file into the chrome palette and, under Follow
//! chrome, the declared terminal theme; and what Settings says about it.
//!
//! [`Workspace::read_theme_source`] is the one way in: choosing the entry, launch and (later)
//! the file watcher's wake all read the file through it. The derivation itself is the
//! library's ([`rex_rune::theme_source`]); hosts only ever see the resulting theme command.
use super::{
    settings_kit::ChipTone,
    theme::{Palette, PaletteRef, set_palette},
    workspace::Workspace,
};
use rex_rune::{
    settings::{ChromeChoice, Settings, TerminalThemeChoice},
    terminal_theme::{TerminalTheme, paired},
    theme_source::{self, Read},
};
use rune::Tx;
use std::{path::PathBuf, time::SystemTime};

/// The theme source as this run knows it.
pub struct Follow {
    /// Where the file is read from: the settings file's path, else the state folder's.
    pub path: PathBuf,
    /// The platform can follow a file (`platform::watch`); without it the entry is left out.
    pub available: bool,
    /// The last reading while following; none before the first.
    pub status: Option<Status>,
    /// The last theme the file gave this run, kept while it is missing or broken.
    good: Option<Good>,
}

/// What the last reading found.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Applied(SystemTime),
    Missing,
    /// Why, in a few words.
    Unreadable(String),
}

struct Good {
    palette: PaletteRef,
    terminal: TerminalTheme,
}

impl Follow {
    pub fn new(settings: &Settings) -> Self {
        // Tests never look at the person's own state folder: without a path of their own,
        // theirs is one that can't exist.
        let path = match settings.theme_source.path.as_deref() {
            None if cfg!(test) => PathBuf::from("/nonexistent/rex-test/theme-source"),
            path => theme_source::path(path),
        };
        Self {
            path,
            available: cfg!(test) || rex_rune::platform::watch::AVAILABLE,
            status: None,
            good: None,
        }
    }
}

/// What the Theme source row says: a word for its chip, and one sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    pub word: &'static str,
    pub tone: ChipTone,
    pub sentence: String,
}

impl Workspace {
    /// The chrome follows the theme source.
    pub fn following_theme_source(&self) -> bool {
        self.theme_source.available && self.settings.chrome_theme == ChromeChoice::ThemeSource
    }

    /// Follow the theme source from now on, as choosing its entry does: the palette in use is
    /// remembered, the file is read, and a terminal theme never chosen moves to Follow chrome.
    pub fn follow_theme_source(&mut self, tx: &mut Tx) {
        if !self.theme_source.available {
            return;
        }
        tx.instant();
        let mut moved = false;
        if !self.following_theme_source() {
            self.settings.theme_source.before = self.settings.chrome_palette();
            self.settings.chrome_theme = ChromeChoice::ThemeSource;
            if !self.settings.terminal_theme_chosen
                && self.settings.terminal_theme == TerminalThemeChoice::FollowServer
            {
                self.settings.terminal_theme = TerminalThemeChoice::FollowChrome;
                moved = true;
            }
        }
        self.save();
        self.read_theme_source(tx);
        if moved && self.theme_source.good.is_none() {
            // Nothing read yet: Follow chrome follows the palette still in use.
            self.term_theme = Some((paired(self.settings.chrome_palette()).theme)());
            self.declare_theme();
        }
    }

    /// Read the theme source again and apply what it gives, in one frame. A missing or broken
    /// file changes nothing on screen: the last good theme stays, or the palette from before.
    pub fn read_theme_source(&mut self, tx: &mut Tx) {
        if self.following_theme_source() && self.load_theme_source() {
            tx.instant();
        }
    }

    /// [`Self::read_theme_source`] without a transaction, for launch. Whether the chrome
    /// changed.
    pub(super) fn load_theme_source(&mut self) -> bool {
        let read = theme_source::read(&self.theme_source.path);
        let status = match read {
            Read::Theme(derived) => {
                self.theme_source.good = Some(Good {
                    palette: Palette::from(&derived.chrome).into(),
                    terminal: derived.terminal,
                });
                Status::Applied(SystemTime::now())
            }
            Read::Missing => Status::Missing,
            Read::Unreadable(why) => Status::Unreadable(why),
        };
        self.theme_source.status = Some(status);
        let Some(good) = &self.theme_source.good else {
            return false;
        };
        let terminal = (self.settings.terminal_theme == TerminalThemeChoice::FollowChrome)
            .then(|| good.terminal.clone());
        let changed = self.palette != good.palette;
        if changed {
            set_palette(good.palette.clone());
            self.palette = good.palette.clone();
        }
        if terminal.is_some() && terminal != self.term_theme {
            self.term_theme = terminal;
            self.declare_theme();
        }
        changed
    }

    /// Follow chrome's terminal theme while the theme source has given one.
    pub fn source_terminal_theme(&self) -> Option<TerminalTheme> {
        self.theme_source
            .good
            .as_ref()
            .filter(|_| self.following_theme_source())
            .map(|good| good.terminal.clone())
    }

    /// The Theme source row's chip and sentence.
    pub fn theme_source_said(&self) -> Said {
        let said = |word, tone, sentence: String| Said {
            word,
            tone,
            sentence,
        };
        let status = self
            .theme_source
            .status
            .as_ref()
            .filter(|_| self.following_theme_source());
        let Some(status) = status else {
            return if self.theme_source.path.is_file() {
                said(
                    "Found",
                    ChipTone::Good,
                    "Found. Choose Theme source to follow it.".into(),
                )
            } else {
                said(
                    "Not found",
                    ChipTone::Off,
                    "Not found. Nothing is there yet.".into(),
                )
            };
        };
        let meanwhile = if self.theme_source.good.is_some() {
            "the last colours stay".to_owned()
        } else {
            format!(
                "{} stays until it can be read",
                self.settings.chrome_palette().name()
            )
        };
        match status {
            Status::Applied(when) => said(
                "Applied",
                ChipTone::Good,
                match rex_rune::platform::clock::local_hour_minute(*when) {
                    Some(clock) => format!("Applied at {clock}."),
                    None => "Applied.".into(),
                },
            ),
            Status::Missing => said(
                "Not found",
                ChipTone::Attention,
                format!("Not found; {meanwhile}."),
            ),
            Status::Unreadable(why) => said(
                "Can't read",
                ChipTone::Attention,
                format!("Can't read it: {why}; {meanwhile}."),
            ),
        }
    }
}
