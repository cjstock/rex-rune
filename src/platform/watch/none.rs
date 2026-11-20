//! No way to follow a file on this platform: [`file`] says so.
use std::{io, path::Path};

pub const AVAILABLE: bool = false;

pub enum Watch {}

pub fn file(_path: &Path, _wake: Box<dyn FnMut() + Send>) -> io::Result<Watch> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "following a file is not available on this platform",
    ))
}
