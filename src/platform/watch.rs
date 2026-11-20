//! Following a file as it changes on disk.
//!
//! [`file`] watches one path and calls `wake`, on a thread of its own, once the file has been
//! quiet for [`SETTLE`] after a change: written and closed, renamed into place, created or
//! deleted. A burst of changes is one wake. Following stops when the [`Watch`] drops.
//!
//! Only Linux can (inotify, on the file's folder, so a file that is replaced or does not exist
//! yet is still followed). Everywhere else [`file`] says it is not available, and a feature
//! that follows a file, such as the theme source, is left out rather than offered and broken.
//! There is no polling stand-in.
use std::{io, path::Path, time::Duration};

#[cfg(target_os = "linux")]
#[path = "watch/inotify.rs"]
mod imp;
#[cfg(not(target_os = "linux"))]
#[path = "watch/none.rs"]
mod imp;

/// Whether this platform can follow a file as it changes.
pub const AVAILABLE: bool = imp::AVAILABLE;

/// How long a file must be quiet after its last change before `wake` is called.
pub const SETTLE: Duration = Duration::from_millis(100);

/// Following one file; dropping it stops following, and `wake` is never called again.
pub struct Watch(#[allow(dead_code)] imp::Watch);

/// Follow the file at `path` (its folder must exist; the file need not), calling `wake` on the
/// watcher's own thread [`SETTLE`] after the last change to it. Changes to other files in the
/// same folder are not changes to it. Where the platform cannot follow a file, the error's kind
/// is [`io::ErrorKind::Unsupported`].
pub fn file(path: &Path, wake: impl FnMut() + Send + 'static) -> io::Result<Watch> {
    imp::file(path, Box::new(wake)).map(Watch)
}

#[cfg(all(test, target_os = "linux"))]
#[path = "watch/inotify_tests.rs"]
mod tests;

/// The other platforms' stand-in, built on Linux too so its tests run here as well.
#[cfg(all(test, target_os = "linux"))]
#[path = "watch/none.rs"]
mod stand_in;

#[cfg(all(test, not(target_os = "linux")))]
mod tests {
    #[test]
    fn following_a_file_is_not_available_here() {
        const { assert!(!super::AVAILABLE) };
        let error = super::file(std::path::Path::new("theme-source"), || {})
            .err()
            .expect("no watcher");
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    }
}
