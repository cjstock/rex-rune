//! Following a file with inotify: a watch on its folder, filtered to its name, so writing it in
//! place, renaming a new one over it, creating it and deleting it are all seen. One thread per
//! [`Watch`] waits on the inotify descriptor and on an eventfd that dropping the watch writes.
use super::SETTLE;
use std::{
    ffi::CString,
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::Path,
    sync::Arc,
    time::Instant,
};

pub const AVAILABLE: bool = true;

/// The events that change the file: closed after writing, renamed into place, created and
/// deleted.
const CHANGES: u32 = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_CREATE | libc::IN_DELETE;

/// An `inotify_event`'s fixed part: watch, mask, cookie and the name's length.
const HEADER: usize = std::mem::size_of::<libc::inotify_event>();

pub struct Watch {
    stop: Arc<OwnedFd>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        let one = 1u64;
        // SAFETY: writes eight bytes from a live u64 to an eventfd this watch owns.
        unsafe { libc::write(self.stop.as_raw_fd(), (&raw const one).cast(), 8) };
    }
}

pub fn file(path: &Path, wake: Box<dyn FnMut() + Send>) -> io::Result<Watch> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?
        .as_bytes()
        .to_vec();
    let folder = match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    };
    let folder = CString::new(folder.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a NUL in the path"))?;
    let inotify = owned(unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) })?;
    // SAFETY: a valid descriptor and a NUL-terminated path.
    let watched = unsafe {
        libc::inotify_add_watch(
            inotify.as_raw_fd(),
            folder.as_ptr(),
            CHANGES | libc::IN_ONLYDIR,
        )
    };
    if watched < 0 {
        return Err(io::Error::last_os_error());
    }
    let stop = Arc::new(owned(unsafe {
        libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK)
    })?);
    let stopped = stop.clone();
    std::thread::Builder::new()
        .name("rex-watch".into())
        .spawn(move || follow(inotify, stopped, name, wake))?;
    Ok(Watch { stop })
}

/// A descriptor a call returned, or its error.
fn owned(fd: libc::c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a descriptor just returned to this process and owned by nothing else.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// The watcher's thread: until the watch drops or the folder goes, wake [`SETTLE`] after the
/// last change to `name`.
fn follow(inotify: OwnedFd, stop: Arc<OwnedFd>, name: Vec<u8>, mut wake: Box<dyn FnMut() + Send>) {
    let mut due: Option<Instant> = None;
    // u64s, so the buffer is aligned for the events the kernel writes into it.
    let mut buffer = [0u64; 512];
    loop {
        let timeout = due.map_or(-1, |due| {
            let left = due.saturating_duration_since(Instant::now());
            left.as_micros().div_ceil(1000).min(i32::MAX as u128) as i32
        });
        let mut fds = [
            libc::pollfd {
                fd: inotify.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: stop.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: two initialized pollfds.
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, timeout) } < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return;
        }
        if fds[1].revents != 0 {
            return;
        }
        let mut gone = fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0;
        if fds[0].revents & libc::POLLIN != 0 {
            loop {
                // SAFETY: reads into a buffer of exactly this many bytes.
                let read = unsafe {
                    libc::read(
                        inotify.as_raw_fd(),
                        buffer.as_mut_ptr().cast(),
                        std::mem::size_of_val(&buffer),
                    )
                };
                let Ok(read) = usize::try_from(read) else {
                    break;
                };
                if read == 0 {
                    break;
                }
                // SAFETY: the bytes the kernel just wrote, within the buffer.
                let bytes =
                    unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), read) };
                let seen = changes(bytes, &name);
                if seen.changed {
                    due = Some(Instant::now() + SETTLE);
                }
                gone |= seen.gone;
            }
        }
        if due.is_some_and(|due| Instant::now() >= due) || (gone && due.is_some()) {
            due = None;
            wake();
        }
        if gone {
            return;
        }
    }
}

#[derive(Default)]
struct Seen {
    /// The file changed, or events were lost and it may have.
    changed: bool,
    /// The folder's watch ended: it was deleted or moved off its file system.
    gone: bool,
}

/// What a read of inotify events says about the file called `name`.
fn changes(mut bytes: &[u8], name: &[u8]) -> Seen {
    let mut seen = Seen::default();
    while bytes.len() >= HEADER {
        // SAFETY: at least a header's bytes; read unaligned, since names pad events unevenly.
        let event: libc::inotify_event = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast()) };
        let end = (HEADER + event.len as usize).min(bytes.len());
        let named = &bytes[HEADER..end];
        let named = &named[..named.iter().position(|&b| b == 0).unwrap_or(named.len())];
        if event.mask & libc::IN_Q_OVERFLOW != 0 || (event.mask & CHANGES != 0 && named == name) {
            seen.changed = true;
        }
        if event.mask & libc::IN_IGNORED != 0 {
            seen.gone = true;
        }
        bytes = &bytes[end..];
    }
    seen
}
