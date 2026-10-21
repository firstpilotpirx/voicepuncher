use std::path::Path;

use objc2_foundation::NSProcessInfo;

/// The first log line of a session: which build, process and config produced the log.
pub fn launch_header(config_path: &Path) -> String {
    format!(
        "voicepuncher {} starting (pid {}, config {}, macOS {}, {})",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        config_path.display(),
        macos_version(),
        std::env::consts::ARCH
    )
}

fn macos_version() -> String {
    let version = NSProcessInfo::processInfo().operatingSystemVersion();
    format!(
        "{}.{}.{}",
        version.majorVersion, version.minorVersion, version.patchVersion
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_version_pid_config_macos_and_architecture() {
        let header = launch_header(Path::new("/Users/me/.voicepuncher/config.toml"));

        let expected_start = format!(
            "voicepuncher {} starting (pid {}, config /Users/me/.voicepuncher/config.toml, macOS ",
            env!("CARGO_PKG_VERSION"),
            std::process::id()
        );
        assert!(header.starts_with(&expected_start), "{header}");
        assert!(
            header.ends_with(&format!(", {})", std::env::consts::ARCH)),
            "{header}"
        );
        let macos = &header[expected_start.len()..header.rfind(", ").unwrap()];
        assert_eq!(macos.split('.').count(), 3, "{header}");
        assert!(
            macos.split('.').all(|part| part.parse::<u32>().is_ok()),
            "{header}"
        );
    }
}
