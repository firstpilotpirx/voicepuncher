//! The application macOS attributes the permissions of a bare binary to: the nearest
//! ancestor process that is an application, such as the terminal `cargo run` runs in.

use std::process::Command;

/// How far up the process tree the search goes.
const MAX_ANCESTORS: usize = 16;
const APP_EXECUTABLE_MARKER: &str = "/Contents/MacOS/";
const APP_SUFFIX: &str = ".app";
const UNKNOWN_APP: &str = "unknown";

/// The nearest ancestor application's name, else the parent process's name.
pub fn parent_application() -> String {
    let parent = std::os::unix::process::parent_id();
    let mut pid = parent;
    for _ in 0..MAX_ANCESTORS {
        let Some((ppid, command)) = process_entry(pid) else {
            break;
        };
        if let Some(app) = application_name(&command) {
            return app;
        }
        if ppid <= 1 {
            break;
        }
        pid = ppid;
    }
    process_entry(parent).map_or_else(
        || UNKNOWN_APP.into(),
        |(_, command)| executable_name(&command),
    )
}

/// The parent pid and the executable path of `pid`.
fn process_entry(pid: u32) -> Option<(u32, String)> {
    let output = Command::new("ps")
        .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let (ppid, command) = line.split_once(char::is_whitespace)?;
    Some((ppid.trim().parse().ok()?, command.trim().to_string()))
}

/// `iTerm` for `/Applications/iTerm.app/Contents/MacOS/iTerm2`.
fn application_name(command: &str) -> Option<String> {
    let (bundle_path, _) = command.split_once(APP_EXECUTABLE_MARKER)?;
    let bundle = bundle_path.rsplit('/').next()?;
    bundle.strip_suffix(APP_SUFFIX).map(String::from)
}

fn executable_name(command: &str) -> String {
    command
        .rsplit('/')
        .next()
        .unwrap_or(command)
        .trim_start_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_application_of_an_app_executable() {
        assert_eq!(
            application_name("/Applications/iTerm.app/Contents/MacOS/iTerm2").as_deref(),
            Some("iTerm")
        );
    }

    #[test]
    fn should_name_no_application_for_a_plain_executable() {
        assert_eq!(application_name("/bin/zsh"), None);
    }

    #[test]
    fn should_name_a_login_shell_without_its_dash() {
        assert_eq!(executable_name("-zsh"), "zsh");
    }
}
