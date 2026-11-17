//! The user's fast "enter" glued to the last word of an utterance, which the running decode
//! missed (command history of 2026-11-16..17, copied into the voice samples' `glued-enters`):
//! the second look at the end of the speech hears each. Streamed as the streaming bench
//! streams a clip, replaying the service's clock; the time of the report is the session
//! tests' (`second_look`).

use std::path::Path;

use voicepuncher_command_spotting::bench::{glued_enter_clips, run_clip, Pace};
use voicepuncher_test_steps::required_env;

use super::engine;

mod needs_streaming_models {
    use super::*;

    mod needs_voice_samples {
        use super::*;

        #[test]
        fn should_report_every_fast_enter_glued_to_the_end_of_the_users_speech() {
            let root = required_env("VOICEPUNCHER_VOICE_SAMPLES");
            let clips = glued_enter_clips(Path::new(&root)).expect("the glued-enters recordings");
            assert!(clips.len() >= 4, "{} recordings", clips.len());
            let engine = engine();
            for clip in &clips {
                // When
                let outcome = run_clip(&engine, clip, Pace::Fast).expect("the clip streams");
                // Then
                assert_eq!(outcome.heard, ["enter"], "{}", clip.id);
            }
        }
    }
}
