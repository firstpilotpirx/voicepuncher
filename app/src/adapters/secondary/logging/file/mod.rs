//! The application's log file: a size-rotating writer and the folder it lives in.

mod header;
mod panic_hook;
mod rotating;
mod tee;

use std::path::{Path, PathBuf};

pub use header::launch_header;
pub use panic_hook::install_panic_hook;
pub use rotating::RotatingFileWriter;
pub use tee::{terminal_stderr, TeeWriter};

/// The logs folder inside the user's home, shared with the services' logs.
const LOGS_FOLDER_IN_HOME: &str = ".voicepuncher/logs";
/// The application's own log file inside the logs folder.
pub const LOG_FILE_NAME: &str = "voicepuncher.log";
/// The size a log file may reach before it rotates.
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
/// How many rotated files are kept beside the current one.
pub const KEPT_BACKUPS: usize = 5;

/// The logs folder for a home folder; nothing is created.
pub fn logs_dir(home: &Path) -> PathBuf {
    home.join(LOGS_FOLDER_IN_HOME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_place_logs_folder_inside_home() {
        assert_eq!(
            logs_dir(Path::new("/Users/me")),
            PathBuf::from("/Users/me/.voicepuncher/logs")
        );
    }

    #[test]
    fn should_rotate_at_ten_megabytes_keeping_five_backups() {
        assert_eq!(MAX_LOG_BYTES, 10 * 1024 * 1024);
        assert_eq!(KEPT_BACKUPS, 5);
    }
}
