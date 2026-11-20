//! Following the theme source: reading the file into the chrome palette and, under Follow
//! chrome, the declared terminal theme; and what Settings says about it.
//!
//! [`Workspace::read_theme_source`] is the file watcher's wake (`platform::watch`, followed
//! while the entry is chosen); choosing the entry and launch read the file the same way, but
//! always apply it. A wake whose bytes are the ones last read does nothing, and a broken read
//! of a file changed within the watcher's settle window is a writer not done yet: neither is
//! reported. Going from good to broken or missing posts one notice, taken back silently once
//! the file is good again; meanwhile the last good theme stays. The derivation itself is the
//! library's ([`rex_rune::theme_source`]); hosts only ever see the resulting theme command.
use super::{
    settings_kit::ChipTone,
    theme::{Palette, PaletteRef, set_palette},
    workspace::Workspace,
};
use rex_rune::{
    platform::watch::{self, Watch},
    settings::{ChromeChoice, Settings, TerminalThemeChoice},
    terminal_theme::{TerminalTheme, paired},
    theme_source::{self, Read},
};
use rune::Tx;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

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
    /// A hash of the bytes last read; none when there were none.
    seen: Option<u64>,
    /// The notice that said the file went bad, taken back when it is good again.
    bad_notice: Option<u64>,
    /// The file being followed, while the entry is chosen.
    watch: Option<(PathBuf, Watch)>,
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
            seen: None,
            bad_notice: None,
            watch: None,
        }
    }

    /// The file being followed as it changes, if any.
    pub fn watching(&self) -> Option<&Path> {
        self.watch.as_ref().map(|(path, _)| path.as_path())
    }
}

/// What the file holds now, and a hash of its bytes when it has any.
fn reading(path: &Path) -> (Read, Option<u64>) {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            (theme_source::decode(bytes), Some(hasher.finish()))
        }
        Err(error) => (theme_source::unread(&error), None),
    }
}

/// The file changed within the watcher's settle window, so a writer may not be done with it.
fn still_settling(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .is_ok_and(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .map_or(true, |age| age < watch::SETTLE)
        })
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
        self.load_theme_source();
        self.watch_theme_source();
        if moved && self.theme_source.good.is_none() {
            // Nothing read yet: Follow chrome follows the palette still in use.
            self.term_theme = Some((paired(self.settings.chrome_palette()).theme)());
            self.declare_theme();
        }
    }

    /// The file watcher's wake: read the theme source again and apply what it gives, in one
    /// frame. The same bytes as last time do nothing; a broken file changed within the settle
    /// window is not done being written and is not reported. A missing or broken file changes
    /// nothing on screen: the last good theme stays, or the palette from before.
    pub fn read_theme_source(&mut self, tx: &mut Tx) {
        if !self.following_theme_source() {
            return;
        }
        let (read, seen) = reading(&self.theme_source.path);
        if seen.is_some() && seen == self.theme_source.seen {
            return;
        }
        if seen.is_some()
            && matches!(read, Read::Unreadable(_))
            && still_settling(&self.theme_source.path)
        {
            return;
        }
        if self.apply_theme_source(read, seen) {
            tx.instant();
        }
    }

    /// Read the theme source and apply it whatever it holds, for launch and for choosing the
    /// entry. Whether the chrome changed.
    pub(super) fn load_theme_source(&mut self) -> bool {
        let (read, seen) = reading(&self.theme_source.path);
        self.apply_theme_source(read, seen)
    }

    /// Follow the file while the entry is chosen, and only then: started, stopped or moved to
    /// a new path to match. Where the platform can't, nothing follows it.
    pub(super) fn watch_theme_source(&mut self) {
        let wanted = self
            .following_theme_source()
            .then(|| self.theme_source.path.clone());
        if self.theme_source.watching() == wanted.as_deref() {
            return;
        }
        self.theme_source.watch = None;
        let Some(path) = wanted else {
            return;
        };
        let handle = self.handle.clone();
        match watch::file(&path, move || {
            handle.update(|s, tx| s.read_theme_source(tx));
        }) {
            Ok(watch) => self.theme_source.watch = Some((path, watch)),
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {}
            Err(error) => eprintln!(
                "Could not follow the theme source at {} ({error}); it is read at launch and when chosen.",
                path.display()
            ),
        }
    }

    /// Apply one reading: the theme it gives, or the last good one; and the one notice when the
    /// file goes from good to bad. Whether the chrome changed.
    fn apply_theme_source(&mut self, read: Read, seen: Option<u64>) -> bool {
        self.theme_source.seen = seen;
        let was_good = matches!(self.theme_source.status, Some(Status::Applied(_)));
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
        let went_bad = match &status {
            Status::Applied(_) => None,
            Status::Missing => Some("The theme source is gone; the last colours stay".to_owned()),
            Status::Unreadable(why) => Some(format!(
                "Can't read the theme source: {why}; the last colours stay"
            )),
        };
        match went_bad {
            None => {
                if let Some(id) = self.theme_source.bad_notice.take() {
                    self.dismiss_notice(id);
                }
            }
            Some(text) if was_good && self.theme_source.good.is_some() => {
                self.notify(text.clone());
                self.theme_source.bad_notice =
                    self.notices.iter().find(|n| n.text == text).map(|n| n.id);
            }
            Some(_) => {}
        }
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
