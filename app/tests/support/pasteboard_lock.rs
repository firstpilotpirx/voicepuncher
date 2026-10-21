//! The machine-wide pasteboard lock: macOS keeps one general pasteboard per machine, so tests that write it
//! hold this lock while they do, and runs from parallel checkouts take turns instead of overwriting each
//! other's clipboard. A file lock, not the model machine lock, so `just test` never waits for a model run.

use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

/// Holds the pasteboard lock until dropped; the kernel frees it when the holder's process dies, also when
/// nextest kills a hung test.
pub struct PasteboardLock {
    _file: File,
}

impl PasteboardLock {
    /// Takes the lock at its default path, waiting while another run holds it.
    pub fn take() -> Self {
        Self::take_at(&default_path())
    }

    /// Takes the lock at `path`, waiting while another holder has it.
    pub fn take_at(path: &Path) -> Self {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("pasteboard lock directory must be creatable");
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .expect("pasteboard lock file must open");
        if !try_lock(&file, libc::LOCK_NB) {
            eprintln!("waiting for the pasteboard lock {}", path.display());
            assert!(try_lock(&file, 0), "pasteboard lock must be taken");
        }
        Self { _file: file }
    }
}

/// An exclusive `flock` on `file`, with `LOCK_NB` returning false at once when another holder has it.
fn try_lock(file: &File, flags: libc::c_int) -> bool {
    loop {
        // SAFETY: the descriptor belongs to `file`, open for the whole call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | flags) } == 0 {
            return true;
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EWOULDBLOCK) if flags & libc::LOCK_NB != 0 => return false,
            _ => panic!("pasteboard lock flock failed: {error}"),
        }
    }
}

fn default_path() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME must be set");
    PathBuf::from(home).join(".cache/voicepuncher/pasteboard.lock")
}
