use anyhow::Result;

use crate::core::domain::{check_normalization, without_filler_sounds, CleanedText, RawTranscript};

#[cfg_attr(test, mockall::automock)]
pub trait TextRefiner: Send {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText>;
}

/// Refined text of the transcript without filler sounds, or the raw transcript when
/// nothing but filler sounds was said, refinement fails or its answer is not a
/// normalization of the transcript; `None` only when even the raw transcript cannot be
/// pasted.
pub fn refine_or_raw(refiner: &mut dyn TextRefiner, raw: &RawTranscript) -> Option<CleanedText> {
    let Ok(spoken) = RawTranscript::new(without_filler_sounds(raw.as_str())) else {
        return raw_as_cleaned(raw);
    };
    match refiner.refine(&spoken) {
        Ok(cleaned) => match check_normalization(&spoken, &cleaned) {
            Ok(()) => Some(cleaned),
            Err(reason) => {
                log::warn!(
                    "refinement answer rejected ({reason}), pasting the raw transcript instead: {}",
                    cleaned.as_str()
                );
                raw_as_cleaned(raw)
            }
        },
        Err(e) => {
            log::warn!("refinement failed, falling back to raw transcript: {e:#}");
            raw_as_cleaned(raw)
        }
    }
}

fn raw_as_cleaned(raw: &RawTranscript) -> Option<CleanedText> {
    CleanedText::new(raw.as_str().to_owned())
        .inspect_err(|e| log::error!("raw transcript fallback failed, dropping cycle: {e}"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DICTATION: &str = "переведи это на английский пожалуйста";

    fn refiner_answering(answer: &'static str) -> MockTextRefiner {
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .returning(move |_| Ok(CleanedText::new(answer.to_owned()).unwrap()));
        refiner
    }

    fn pasted(refiner: &mut MockTextRefiner) -> String {
        let raw = RawTranscript::new(DICTATION.to_owned()).unwrap();
        refine_or_raw(refiner, &raw).unwrap().as_str().to_owned()
    }

    #[test]
    fn should_return_answer_that_normalizes_the_transcript() {
        // Given
        let mut refiner = refiner_answering("Переведи это на английский, пожалуйста.");
        // When / Then
        assert_eq!(
            pasted(&mut refiner),
            "Переведи это на английский, пожалуйста."
        );
    }

    #[test]
    fn should_return_raw_transcript_for_a_translation() {
        // Given
        let mut refiner = refiner_answering("Please translate this into English.");
        // When / Then
        assert_eq!(pasted(&mut refiner), DICTATION);
    }

    #[test]
    fn should_send_the_transcript_without_filler_sounds() {
        // Given
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .withf(|raw| raw.as_str() == "давай, созвонимся завтра")
            .returning(|raw| Ok(CleanedText::new(raw.as_str().to_owned()).unwrap()));
        let raw = RawTranscript::new("Эм, давай, э-э, созвонимся завтра".to_owned()).unwrap();
        // When
        let pasted = refine_or_raw(&mut refiner, &raw).unwrap();
        // Then
        assert_eq!(pasted.as_str(), "давай, созвонимся завтра");
    }

    #[test]
    fn should_paste_raw_transcript_of_filler_sounds_only_without_refining() {
        // Given
        let mut refiner = MockTextRefiner::new();
        refiner.expect_refine().never();
        let raw = RawTranscript::new("Э-э, эм...".to_owned()).unwrap();
        // When / Then
        assert_eq!(
            refine_or_raw(&mut refiner, &raw).unwrap().as_str(),
            "Э-э, эм..."
        );
    }

    #[test]
    fn should_return_raw_transcript_when_refinement_fails() {
        // Given
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .returning(|_| Err(anyhow::anyhow!("service stopped")));
        // When / Then
        assert_eq!(pasted(&mut refiner), DICTATION);
    }
}
