//! The last good theme source, kept in local state so a launch that finds the file missing or
//! broken still has the colours it last gave.
//!
//! The record is the theme source's own text, written as it was read and re-read at launch by
//! the same parser and derivation. So a record needs no format of its own: one a newer client
//! wrote (a newer `rex-rune-theme-source` version), or one that was damaged, is refused exactly
//! as the file would be, and a change to how the chrome is derived reaches it too.
use super::{Derived, Read, derive_text, text};
use std::path::{Path, PathBuf};

/// The record's name in its folder.
const FILE_NAME: &str = "last-good-theme-source";

/// The last good theme source, kept at one path.
#[derive(Debug)]
pub struct Remembered {
    path: PathBuf,
    /// The text known to be at `path`, once this run has read or written it.
    kept: Option<String>,
}

impl Remembered {
    /// The record in `folder`: this client's state folder (`$XDG_STATE_HOME/rex-rune`), or a
    /// test's own. Nothing is read or written until asked.
    pub fn in_folder(folder: &Path) -> Self {
        Self {
            path: folder.join(FILE_NAME),
            kept: None,
        }
    }

    /// Read the theme source at `source`, as [`super::read`] does, and keep its text when it
    /// gives a theme that isn't the one already kept. Keeping is best effort: a record that
    /// can't be written leaves the reading as it is.
    pub fn read(&mut self, source: &Path) -> Read {
        let text = match text(source) {
            Ok(text) => text,
            Err(none) => return none.into(),
        };
        let read = derive_text(&text);
        if matches!(read, Read::Theme(_)) {
            self.keep(text);
        }
        read
    }

    /// The theme the record gives, if there is one and it can still be read.
    pub fn recall(&self) -> Option<Derived> {
        match derive_text(&text(&self.path).ok()?) {
            Read::Theme(derived) => Some(derived),
            _ => None,
        }
    }

    fn keep(&mut self, text: String) {
        if self.kept.is_none() {
            self.kept = std::fs::read_to_string(&self.path).ok();
        }
        if self.kept.as_deref() == Some(text.as_str()) {
            return;
        }
        if write(&self.path, &text).is_ok() {
            self.kept = Some(text);
        }
    }
}

/// Replace the file at `path` whole: written beside it, then renamed over it.
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::rename(&temporary, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::tests::Temp;

    const GOOD: &str = "background = #1e1e2e\nforeground = #cdd6f4\n";

    #[test]
    fn a_good_reading_is_recalled_and_a_bad_one_leaves_it() {
        let temp = Temp::new("remembered");
        let source = temp.0.join("theme-source");
        let mut record = Remembered::in_folder(&temp.0.join("state"));
        assert_eq!(record.recall(), None, "nothing kept yet");
        std::fs::write(&source, GOOD).unwrap();
        let Read::Theme(good) = record.read(&source) else {
            panic!("a theme")
        };
        assert_eq!(record.recall(), Some(good.clone()));
        std::fs::write(&source, "background = #000000\n").unwrap();
        assert!(matches!(record.read(&source), Read::Unreadable(_)));
        std::fs::remove_file(&source).unwrap();
        assert_eq!(record.read(&source), Read::Missing);
        let again = Remembered::in_folder(&temp.0.join("state"));
        assert_eq!(again.recall(), Some(good), "kept across runs");
    }

    #[test]
    fn the_record_is_written_only_when_the_theme_changes() {
        let temp = Temp::new("remembered");
        let source = temp.0.join("theme-source");
        let path = temp.0.join(FILE_NAME);
        std::fs::write(&source, GOOD).unwrap();
        std::fs::write(&path, GOOD).unwrap();
        let written = |path: &Path| std::fs::metadata(path).unwrap().modified().unwrap();
        let before = written(&path);
        let mut record = Remembered::in_folder(&temp.0);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record.read(&source);
        record.read(&source);
        assert_eq!(written(&path), before, "the same theme: not written again");
        std::fs::write(&source, format!("{GOOD}palette = 1=#f38ba8\n")).unwrap();
        record.read(&source);
        assert!(written(&path) > before, "a new theme is written");
        let names: Vec<_> = std::fs::read_dir(&temp.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 2, "no temporary left behind: {names:?}");
    }

    #[test]
    fn a_damaged_or_newer_record_is_refused() {
        let temp = Temp::new("remembered");
        let path = temp.0.join(FILE_NAME);
        let record = Remembered::in_folder(&temp.0);
        for bytes in [
            b"not a theme\n".to_vec(),
            format!("rex-rune-theme-source = 2\n{GOOD}").into_bytes(),
            vec![0xff, 0xfe, 0],
        ] {
            std::fs::write(&path, &bytes).unwrap();
            assert_eq!(record.recall(), None, "{bytes:?}");
        }
    }
}
