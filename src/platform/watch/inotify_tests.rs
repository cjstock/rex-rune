//! The inotify watcher against a real folder of the test's own.
use super::{SETTLE, Watch, file};
use crate::download::tests::Temp;
use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
    time::{Duration, Instant},
};

/// Long enough for a wake to arrive on a loaded machine.
const SOON: Duration = Duration::from_secs(3);
/// Long enough that a second wake would have come.
const QUIET: Duration = Duration::from_millis(400);

struct Followed {
    _folder: Temp,
    path: std::path::PathBuf,
    watch: Option<Watch>,
    wakes: Receiver<Instant>,
}

fn followed(label: &str) -> Followed {
    let folder = Temp::new(label);
    let path = folder.0.join("theme-source");
    let (sender, wakes) = channel();
    let watch = file(&path, move || {
        let _ = sender.send(Instant::now());
    })
    .unwrap();
    Followed {
        _folder: folder,
        path,
        watch: Some(watch),
        wakes,
    }
}

impl Followed {
    /// Exactly one wake, and when it came.
    fn one_wake(&self, what: &str) -> Instant {
        let at = self
            .wakes
            .recv_timeout(SOON)
            .unwrap_or_else(|_| panic!("{what}: a wake"));
        self.no_wake(what);
        at
    }

    fn no_wake(&self, what: &str) {
        assert_eq!(
            self.wakes.recv_timeout(QUIET),
            Err(RecvTimeoutError::Timeout),
            "{what}: no other wake"
        );
    }
}

#[test]
fn the_platform_can_follow_a_file() {
    const { assert!(super::AVAILABLE) };
}

#[test]
fn elsewhere_following_a_file_is_not_available() {
    use super::stand_in;
    const { assert!(!stand_in::AVAILABLE) };
    let error = stand_in::file(std::path::Path::new("theme-source"), Box::new(|| {}))
        .err()
        .expect("no watcher");
    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
}

#[test]
fn creating_writing_replacing_and_deleting_the_file_each_wake_once() {
    let f = followed("watch-each");
    f.no_wake("nothing changed yet");

    // Created and still open: the creation alone.
    let open = std::fs::File::create(&f.path).unwrap();
    f.one_wake("created");
    // Closed after writing.
    drop(open);
    f.one_wake("closed after writing");

    // Written in place, as matugen does.
    std::fs::write(&f.path, "background = #000000\n").unwrap();
    f.one_wake("written in place");

    // Written beside it and renamed into place, as a post hook's `mv` does.
    let beside = f.path.with_file_name(".theme-source.new");
    std::fs::write(&beside, "background = #ffffff\n").unwrap();
    std::fs::rename(&beside, &f.path).unwrap();
    f.one_wake("renamed into place");

    std::fs::remove_file(&f.path).unwrap();
    f.one_wake("deleted");
}

#[test]
fn other_files_in_the_folder_are_not_the_file() {
    let f = followed("watch-others");
    let other = f.path.with_file_name("theme-source.bak");
    std::fs::write(&other, "x").unwrap();
    std::fs::rename(&other, f.path.with_file_name("elsewhere")).unwrap();
    std::fs::write(f.path.with_file_name("theme-source-2"), "x").unwrap();
    f.no_wake("other files");
}

#[test]
fn a_burst_of_changes_is_one_wake_after_the_last_settles() {
    let f = followed("watch-burst");
    let mut last = Instant::now();
    for i in 0..8 {
        std::fs::write(&f.path, format!("background = #00000{i}\n")).unwrap();
        last = Instant::now();
        std::thread::sleep(SETTLE / 4);
    }
    let woke = f.one_wake("a burst");
    assert!(
        woke >= last + SETTLE,
        "the wake waits {SETTLE:?} after the last change: {:?}",
        woke - last
    );
}

#[test]
fn dropping_the_watch_stops_following() {
    let mut f = followed("watch-drop");
    std::fs::write(&f.path, "a").unwrap();
    f.one_wake("while following");
    f.watch = None;
    std::fs::write(&f.path, "b").unwrap();
    assert_eq!(
        f.wakes.recv_timeout(SOON),
        Err(RecvTimeoutError::Disconnected),
        "no wake, and the watcher let go of it"
    );
}

#[test]
fn a_folder_that_does_not_exist_cannot_be_followed() {
    let folder = Temp::new("watch-missing");
    let path = folder.0.join("missing").join("theme-source");
    let error = file(&path, || {}).err().expect("no watcher");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
