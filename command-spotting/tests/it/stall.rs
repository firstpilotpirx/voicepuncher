//! A confirmation that gets no answer from the speech recognition service, on the real
//! keyword spotting model and voice activity detector, which the justfile finds
//! (`needs_kws_models`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::common;
use crate::kws_models::kws_models;

use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::ServerEvent;

const DEADLINE: Duration = Duration::from_secs(3);
const FRAME_SAMPLES: usize = 1_600;
/// How soon after its speech a failed confirmation is reported.
const REPORTED_WITHIN: Duration = Duration::from_secs(10);

fn engine() -> Engine {
    let (model_dir, vad_model) = kws_models();
    let cfg = EngineCfg {
        model_dir,
        vad_model,
        threads: 2,
        min_silence_secs: 0.3,
        save_speech_dir: None,
        keywords: None,
        transcription_url: None,
        max_command_secs: 3.0,
        confirmation_deadline: DEADLINE,
    };
    Engine::load(&cfg).expect("models load")
}

/// A stalled speech recognition service: every confirmation waits its deadline out.
struct Stalled;

impl Hear for Stalled {
    fn hear(&self, _: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        std::thread::sleep(deadline);
        anyhow::bail!("speech recognition copy sent no answer in {deadline:?}")
    }
}

mod needs_kws_models {
    use super::*;

    #[test]
    fn should_report_a_segment_whose_confirmation_got_no_answer_as_failed() {
        let engine = engine();
        // Given
        let engine = engine.hearing(Arc::new(Stalled));
        let mut session = engine.session().unwrap();
        session.listen_for(&["press enter".to_owned()]).unwrap();
        let pause = vec![0.0f32; SAMPLE_RATE as usize];
        let audio: Vec<f32> = pause
            .iter()
            .copied()
            .chain(common::spoken_by(None, "press enter"))
            .chain(pause.iter().copied())
            .collect();
        // When
        let mut events: Vec<ServerEvent> = audio
            .chunks(FRAME_SAMPLES)
            .flat_map(|frame| session.accept(frame))
            .collect();
        let fed = Instant::now();
        events.extend(session.settle());
        // Then
        let failed: Vec<&ServerEvent> = events
            .iter()
            .filter(|e| matches!(e, ServerEvent::ConfirmationFailed { .. }))
            .collect();
        assert_eq!(failed.len(), 1, "{events:?}");
        assert!(fed.elapsed() < REPORTED_WITHIN, "took {:?}", fed.elapsed());
        let end = events.iter().find_map(|e| match e {
            ServerEvent::SpeechEnd { t } => Some(*t),
            _ => None,
        });
        let failed_end = failed.iter().find_map(|e| match e {
            ServerEvent::ConfirmationFailed { end, .. } => Some(*end),
            _ => None,
        });
        assert_eq!(failed_end, end, "{events:?}");
    }
}
