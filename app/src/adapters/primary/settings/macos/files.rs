//! Folders the window names: written with `~` in the file, shown in Finder on request.

use std::path::{Path, PathBuf};

use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSString;

/// How the configuration file names the home folder.
const HOME_PREFIX: &str = "~/";

/// `folder` as the file writes it, with `~/` resolved to the home folder.
pub fn resolve(folder: &str) -> PathBuf {
    match folder.strip_prefix(HOME_PREFIX) {
        Some(rest) => home().join(rest),
        None => PathBuf::from(folder),
    }
}

/// `path` as the file writes it: `~/…` inside the home folder.
pub fn collapse_home(path: &Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rest) => format!("{HOME_PREFIX}{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// Shows `folder` in Finder, creating it first.
pub fn reveal(folder: &Path) {
    if let Err(e) = std::fs::create_dir_all(folder) {
        log::warn!("settings: cannot create {}: {e}", folder.display());
        return;
    }
    let path = NSString::from_str(&folder.to_string_lossy());
    if !NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(Some(&path), &path) {
        log::warn!("settings: {} not shown in Finder", folder.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_write_a_folder_in_the_home_folder_with_a_tilde() {
        // Given
        let folder = home().join(".voicepuncher/archive");
        // When
        let written = collapse_home(&folder);
        // Then
        assert_eq!(written, "~/.voicepuncher/archive");
        assert_eq!(resolve(&written), folder);
    }

    #[test]
    fn should_keep_a_folder_outside_the_home_folder_as_it_is() {
        // Given / When
        let written = collapse_home(Path::new("/Volumes/Archive"));
        // Then
        assert_eq!(written, "/Volumes/Archive");
        assert_eq!(resolve(&written), PathBuf::from("/Volumes/Archive"));
    }
}
