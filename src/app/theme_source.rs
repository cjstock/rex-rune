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
//!
//! Set up DMS ([`Workspace::set_up_dms`]) writes the theme source template and the DMS
//! drop-in ([`rex_rune::theme_source::dms`]) when pressed, and only then; the row then waits
//! for DMS's next run, or says DMS writes a different file once the path no longer matches.
use super::{
    settings_kit::ChipTone,
    theme::{Palette, PaletteRef, set_palette},
    workspace::Workspace,
};
use rex_rune::{
    platform::{
        tools::Search,
        watch::{self, Watch},
    },
    settings::{ChromeChoice, Settings, TerminalThemeChoice},
    terminal_theme::{TerminalTheme, paired},
    theme_source::{
        self, Derived, Read, Remembered,
        dms::{self, DropIn, Places},
    },
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
    /// The last theme the file gave, kept while it is missing or broken: this run's, else the
    /// one remembered from an earlier run.
    good: Option<Good>,
    /// The last good theme source, kept in local state for the next launch.
    remembered: Remembered,
    /// A hash of the bytes last read; none when there were none.
    seen: Option<u64>,
    /// The notice that said the file went bad, taken back when it is good again.
    bad_notice: Option<u64>,
    /// The file being followed, while the entry is chosen.
    watch: Option<(PathBuf, Watch)>,
    /// Home, the XDG folders and the tool search Set up DMS uses: the person's, or a test
    /// window's own under its folder.
    pub places: Places,
    /// Set up DMS was pressed and the file has not given a theme since.
    waiting: bool,
}

/// A test window's home, XDG folders and tool search, all under its own `folder`, so no test
/// reads or writes the person's, or finds their `dms`.
fn private_places(folder: &Path) -> Places {
    let home = folder.join("home");
    Places {
        data: home.join(".local/share"),
        config: home.join(".config"),
        state: home.join(".local/state/rex-rune"),
        tools: Search {
            path: Some(folder.join("bin").into_os_string()),
            user_bin: home.join(".local/bin"),
        },
        home,
    }
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

impl From<Derived> for Good {
    fn from(derived: Derived) -> Self {
        Self {
            palette: Palette::from(&derived.chrome).into(),
            terminal: derived.terminal,
        }
    }
}

impl Follow {
    /// `settings_path` is where settings are saved; a test window's folder also holds its
    /// remembered theme.
    pub fn new(settings: &Settings, settings_path: &Path) -> Self {
        let folder = settings_path.parent().unwrap_or(Path::new(""));
        let places = if cfg!(test) {
            private_places(folder)
        } else {
            Places::from_env()
        };
        // A newer client's template replaces the one an older one wrote; the drop-in waits
        // for the button.
        if let Err(error) = dms::refresh_template(&places) {
            eprintln!(
                "Could not refresh the theme source template at {} ({error}).",
                places.template().display()
            );
        }
        Self {
            path: places.theme_source(settings.theme_source.path.as_deref()),
            places,
            waiting: false,
            available: cfg!(test) || rex_rune::platform::watch::AVAILABLE,
            status: None,
            good: None,
            remembered: Remembered::in_folder(&if cfg!(test) {
                folder.to_path_buf()
            } else {
                rex_rune::platform::dirs::state()
            }),
            seen: None,
            bad_notice: None,
            watch: None,
        }
    }

    /// The file being followed as it changes, if any.
    pub fn watching(&self) -> Option<&Path> {
        self.watch.as_ref().map(|(path, _)| path.as_path())
    }

    /// The file is the client's own default in its state folder, not a path the person set:
    /// the client may make its folder.
    fn own_folder(&self) -> bool {
        self.path == self.places.theme_source(None)
    }

    /// The DMS drop-in as it is now, for this file.
    pub fn drop_in(&self) -> DropIn {
        dms::drop_in(&self.places, &self.path)
    }

    /// DMS is installed and its drop-in is missing or not what this client would write now.
    pub fn dms_offered(&self) -> bool {
        self.available
            && matches!(
                self.drop_in(),
                DropIn::Missing | DropIn::Stale | DropIn::WritesElsewhere(_)
            )
            && self.places.dms()
    }
}

/// What the file holds now, and a hash of its bytes when it has any. A theme it gives is
/// remembered for the next launch.
fn reading(path: &Path, remembered: &mut Remembered) -> (Read, Option<u64>) {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            (remembered.decode(bytes), Some(hasher.finish()))
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
        let (read, seen) = reading(&self.theme_source.path, &mut self.theme_source.remembered);
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
        let (read, seen) = reading(&self.theme_source.path, &mut self.theme_source.remembered);
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
        // A folder is watched, so it must be there before its file is. The client's own is
        // made here; a folder the person named is theirs to make.
        if self.theme_source.own_folder()
            && let Some(folder) = path.parent()
            && let Err(error) = std::fs::create_dir_all(folder)
        {
            eprintln!(
                "Could not make the theme source's folder {} ({error}).",
                folder.display()
            );
        }
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
                self.theme_source.good = Some(derived.into());
                self.theme_source.waiting = false;
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
        if self.theme_source.good.is_none() {
            // Missing or broken before anything was read this run: an earlier run's theme.
            self.theme_source.good = self.theme_source.remembered.recall().map(Good::from);
        }
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

    /// Set up DMS, pressed: write the theme source template and the DMS drop-in that points
    /// it at this file, make the file's folder when it is the client's own, and wait for DMS
    /// to run. DMS is never asked to; its next colour change writes the file.
    pub fn set_up_dms(&mut self) {
        let follow = &self.theme_source;
        match dms::set_up(&follow.places, &follow.path, follow.own_folder()) {
            Ok(()) => {
                self.theme_source.waiting = true;
                // Its folder may be new: follow the file now if that failed before.
                self.watch_theme_source();
            }
            Err(error) => self.notify(format!("Couldn't set up DMS: {error}")),
        }
    }

    /// The Theme source row's chip and sentence.
    pub fn theme_source_said(&self) -> Said {
        let said = |word, tone, sentence: String| Said {
            word,
            tone,
            sentence,
        };
        let follow = &self.theme_source;
        if matches!(follow.drop_in(), DropIn::WritesElsewhere(_)) && follow.places.dms() {
            return said(
                "Other file",
                ChipTone::Attention,
                "DMS writes a different file; Set up DMS points it here.".into(),
            );
        }
        let following = self.following_theme_source();
        // Set up, and DMS hasn't written a theme yet: following, until the file gives one;
        // otherwise until there is a file.
        if follow.waiting && (following || !follow.path.is_file()) {
            return said(
                "Waiting",
                ChipTone::Off,
                "Waiting for DMS: change the wallpaper or switch light and dark.".into(),
            );
        }
        let status = follow.status.as_ref().filter(|_| following);
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
