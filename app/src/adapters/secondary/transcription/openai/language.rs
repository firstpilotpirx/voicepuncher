use serde::Deserialize;

use crate::config::TranscriptionCfg;

#[derive(Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    /// The language the service detected; absent from services that do not name it.
    #[serde(default)]
    pub language: Option<String>,
}

/// The user's language to request the transcript in again, when the service detected a
/// language the user does not speak; `None` keeps the transcript.
pub fn retry_language<'a>(
    response: &TranscriptionResponse,
    cfg: &'a TranscriptionCfg,
) -> Option<&'a str> {
    if cfg.language != TranscriptionCfg::AUTO_LANGUAGE {
        return None;
    }
    let detected = response.language.as_deref()?.to_lowercase();
    let spoken_by_user = cfg.languages.iter().any(|code| {
        *code == detected || TranscriptionCfg::language_name(code) == Some(detected.as_str())
    });
    if spoken_by_user {
        return None;
    }
    cfg.languages.first().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detected(language: Option<&str>) -> TranscriptionResponse {
        TranscriptionResponse { text: "text".into(), language: language.map(str::to_owned) }
    }

    #[test]
    fn should_request_again_in_the_first_user_language_after_a_foreign_one() {
        let cfg = TranscriptionCfg::default();
        assert_eq!(retry_language(&detected(Some("korean")), &cfg), Some("ru"));
    }

    #[test]
    fn should_keep_a_language_named_by_code_or_by_name() {
        let cfg = TranscriptionCfg::default();
        assert_eq!(retry_language(&detected(Some("English")), &cfg), None);
        assert_eq!(retry_language(&detected(Some("ru")), &cfg), None);
    }

    #[test]
    fn should_keep_a_transcript_whose_language_the_service_does_not_name() {
        assert_eq!(
            retry_language(&detected(None), &TranscriptionCfg::default()),
            None
        );
    }
}
