//! Following a file as it changes on disk.
//!
//! Only Linux can (inotify); everywhere else, a feature that follows a file, such as the
//! theme source, is left out rather than offered and broken. The watcher itself is not here
//! yet: until it is, a followed file is read when it is chosen and at launch.

/// Whether this platform can follow a file as it changes.
pub const AVAILABLE: bool = cfg!(target_os = "linux");
