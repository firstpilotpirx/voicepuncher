use crate::core::application::PermissionProbe;
use anyhow::anyhow;

pub struct Preflight;

impl Preflight {
    pub fn run(probe: &dyn PermissionProbe) -> anyhow::Result<()> {
        let parent_name = probe
            .parent_process_name()
            .unwrap_or_else(|_| "unknown".to_string());

        probe.check_microphone().map_err(|_| {
            anyhow!("permission denied for microphone; grant Microphone to {parent_name}")
        })?;

        probe.check_accessibility().map_err(|_| {
            anyhow!("permission denied for accessibility; grant Accessibility to {parent_name}")
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::permission_probe::MockPermissionProbe;
    use anyhow::anyhow;

    #[test]
    fn should_return_ok_when_all_checks_pass() {
        // Given
        let mut mock = MockPermissionProbe::new();
        mock.expect_parent_process_name()
            .returning(|| Ok("iTerm2".to_string()));
        mock.expect_check_microphone().returning(|| Ok(()));
        mock.expect_check_accessibility().returning(|| Ok(()));

        // When
        let result = Preflight::run(&mock);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_return_err_with_parent_name_when_mic_denied() {
        // Given
        let mut mock = MockPermissionProbe::new();
        mock.expect_parent_process_name()
            .returning(|| Ok("iTerm2".to_string()));
        mock.expect_check_microphone()
            .returning(|| Err(anyhow!("denied")));

        // When
        let result = Preflight::run(&mock);

        // Then
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("microphone"),
            "expected 'microphone' in: {msg}"
        );
        assert!(msg.contains("iTerm2"), "expected 'iTerm2' in: {msg}");
    }

    #[test]
    fn should_return_err_with_parent_name_when_accessibility_denied() {
        // Given
        let mut mock = MockPermissionProbe::new();
        mock.expect_parent_process_name()
            .returning(|| Ok("iTerm2".to_string()));
        mock.expect_check_microphone().returning(|| Ok(()));
        mock.expect_check_accessibility()
            .returning(|| Err(anyhow!("denied")));

        // When
        let result = Preflight::run(&mock);

        // Then
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("accessibility"),
            "expected 'accessibility' in: {msg}"
        );
        assert!(msg.contains("iTerm2"), "expected 'iTerm2' in: {msg}");
    }

    #[test]
    fn should_use_unknown_when_parent_process_name_fails() {
        // Given
        let mut mock = MockPermissionProbe::new();
        mock.expect_parent_process_name()
            .returning(|| Err(anyhow!("no ppid")));
        mock.expect_check_microphone()
            .returning(|| Err(anyhow!("denied")));

        // When
        let result = Preflight::run(&mock);

        // Then
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown"), "expected 'unknown' in: {msg}");
    }
}
