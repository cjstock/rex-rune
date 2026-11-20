//! Command-line tools this client runs, and where a platform keeps them when they are not
//! on `PATH`.
//!
//! On macOS `tailscale` and `rex` ship inside their apps, not in a folder `PATH` names:
//! the Tailscale app's own program answers as the CLI, and the Rex app carries its server
//! and CLI as a helper.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Where a Mac app keeps `name`, in the order they are tried. Empty for any other tool
/// and on every other platform.
fn bundled_candidates(name: &str) -> &'static [&'static str] {
    if !cfg!(target_os = "macos") {
        return &[];
    }
    match name {
        "tailscale" => &["/Applications/Tailscale.app/Contents/MacOS/Tailscale"],
        "rex" => &[
            "/Applications/Rex.app/Contents/Helpers/Rex Server.rex/Contents/MacOS/rex",
            "/Applications/Rex Beta.app/Contents/Helpers/Rex Server.rex/Contents/MacOS/rex",
        ],
        _ => &[],
    }
}

/// The copy of `name` inside its app, if this platform has apps that carry one and it is
/// installed.
pub fn bundled(name: &str) -> Option<PathBuf> {
    bundled_candidates(name)
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

/// Where a tool is looked for: the folders of a `PATH`, then the user's own bin folder, then
/// the copy inside its app. A test makes its own, so it never finds the person's tools.
#[derive(Clone, Debug)]
pub struct Search {
    /// A `PATH`'s value.
    pub path: Option<OsString>,
    /// `$XDG_BIN_HOME`, else `~/.local/bin`, which a desktop launch often leaves off `PATH`.
    pub user_bin: PathBuf,
}

impl Search {
    /// The search this process's environment describes.
    pub fn from_env() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            user_bin: std::env::var_os("XDG_BIN_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| super::dirs::home().join(".local/bin")),
        }
    }

    /// Whether a folder of the `PATH` holds `name`.
    pub fn on_path(&self, name: &str) -> bool {
        self.path
            .as_ref()
            .is_some_and(|path| std::env::split_paths(path).any(|dir| dir.join(name).is_file()))
    }

    /// Where `name` is when the `PATH` does not have it: the user's bin folder, then its app.
    pub fn off_path(&self, name: &str) -> Option<PathBuf> {
        Some(self.user_bin.join(name))
            .filter(|path| path.is_file())
            .or_else(|| bundled(name))
    }

    /// Whether `name` is installed anywhere this search looks.
    pub fn finds(&self, name: &str) -> bool {
        self.on_path(name) || self.off_path(name).is_some()
    }
}

/// Whether a folder of `PATH` holds `name`.
pub fn on_path(name: &str) -> bool {
    Search::from_env().on_path(name)
}

/// Where `name` is when `PATH` does not have it: the user's own bin folder
/// (`$XDG_BIN_HOME`, else `~/.local/bin`), which a desktop launch often leaves off `PATH`,
/// then the copy inside its app.
pub fn off_path(name: &str) -> Option<PathBuf> {
    Search::from_env().off_path(name)
}

/// What to run for `name`: the name itself when `PATH` has it, else [`off_path`], else the
/// name again, so that a missing tool fails as "not found" under its own name.
pub fn program(name: &str) -> PathBuf {
    if on_path(name) {
        return Path::new(name).into();
    }
    off_path(name).unwrap_or_else(|| Path::new(name).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_is_its_own_name_unless_only_an_app_has_it() {
        // No app carries this, anywhere.
        assert_eq!(
            program("rex-rune-no-such-tool"),
            Path::new("rex-rune-no-such-tool")
        );
        assert!(bundled("sh").is_none());
        for name in ["tailscale", "rex"] {
            let program = program(name);
            if on_path(name) {
                assert_eq!(program, Path::new(name));
            } else if let Some(found) = off_path(name) {
                assert!(found.is_absolute() && found.is_file());
                assert_eq!(program, found);
            } else {
                assert_eq!(program, Path::new(name));
            }
        }
        if !cfg!(target_os = "macos") {
            assert!(bundled("tailscale").is_none() && bundled("rex").is_none());
        }
    }
}
