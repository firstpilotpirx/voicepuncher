use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

mod confirming;
mod running;
mod spotting;

mod needs_voice_samples {
    use super::*;

    #[test]
    fn should_keep_the_users_two_commands_apart_across_a_short_pause_with_a_click() {
        let engine = engine();
        let audio = users_two_start_dictations();
        // When
        let events = spotted_in(&engine, &audio);
        // Then
        assert_eq!(speech_segments(&events), 2, "{events:?}");
    }

    #[test]
    fn should_end_the_users_quiet_command_at_a_short_pause_before_more_speech() {
        let engine = engine();
        let command = users_quiet_restart();
        for pause_secs in [0.4, 0.5] {
            // When
            let (ends, sentence_start, events) =
                ends_before_sentence(&engine, &command, pause_secs);
            // Then
            assert!(
                    ends.first().is_some_and(|end| *end < sentence_start),
                    "after a {pause_secs} s pause the quiet command's utterance did not end before the sentence at {sentence_start:.2}s: {events:?}"
                );
        }
    }

    #[test]
    fn should_confirm_a_command_followed_by_a_short_pause_without_the_speech_after_it() {
        let _shared = engine();
        // Given
        let hear = Arc::new(KeepsClips::default());
        let engine = own_engine(None).hearing(hear.clone());
        let command = users_quiet_restart();
        // The detector holds the quiet voice's speech about 0.55 s past its last sound: a clip
        // running past that end takes in the sentence's first sound.
        let (audio, sentence_start) = command_pause_sentence(&command, 0.6);
        let at = saturating_usize(sentence_start * SAMPLE_RATE as f32);
        let sentence_onset = audio[at..at + MARK_SAMPLES].to_vec();
        let command_end = command[command.len() - MARK_SAMPLES..].to_vec();
        let mut session = listening(&engine, &ALL_PHRASES);
        // When
        fed(&mut session, &audio);
        // Then
        let clips = hear.0.lock().unwrap();
        let mixed = clips
            .iter()
            .filter(|clip| holds(clip, &command_end) && holds(clip, &sentence_onset))
            .count();
        let command_clips = clips
            .iter()
            .filter(|clip| holds(clip, &command_end))
            .count();
        assert!(
                command_clips > 0 && mixed == 0,
                "{mixed} of {command_clips} clips holding the command also hold the sentence's first sound"
            );
    }
}
