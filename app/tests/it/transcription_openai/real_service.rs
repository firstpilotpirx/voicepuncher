//! Against the real speech recognition service.

use super::*;

/// The real service's URL and the corpus's first recording with its manifest entry.
pub(super) fn real_service_and_first_corpus_sample() -> (String, serde_json::Value, AudioBuffer16k)
{
    let base_url = required_env("VOICEPUNCHER_TRANSCRIPTION_URL");
    let corpus = required_env("VOICEPUNCHER_BENCH_CORPUS");
    let manifest = std::fs::read_to_string(PathBuf::from(&corpus).join("manifest.jsonl")).unwrap();
    let first: serde_json::Value = serde_json::from_str(manifest.lines().next().unwrap()).unwrap();
    let wav = PathBuf::from(&corpus).join(first["path"].as_str().unwrap());
    let audio = FileSystemAudioRecorder::load(&wav).unwrap();
    (base_url, first, audio)
}

pub(super) fn transcribe_through_real_service(base_url: &str, audio: &AudioBuffer16k) -> String {
    let cfg = TranscriptionCfg { base_url: base_url.to_owned(), ..TranscriptionCfg::default() };
    let transcript = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .transcribe(audio)
        .unwrap();
    transcript
        .expect("speech should not be discarded")
        .as_str()
        .to_owned()
}

/// The peak of the white noise laid over speech: loud enough that the service is unsure of
/// windows, which made it fall back to random sampling, yet quiet enough that the speech is
/// still kept (a peak of 6000 buried it: the transcript was discarded as no speech).
pub(super) const HIGH_NOISE_PEAK: i32 = 1_500;

pub(super) const NOISE_SEED: u32 = 0x5eed_1234;

/// Lays reproducible white noise (a fixed-seed linear congruential generator) over the speech.
pub(super) fn under_high_noise(speech: &AudioBuffer16k) -> AudioBuffer16k {
    const LCG_MULTIPLIER: u32 = 1_664_525;
    const LCG_INCREMENT: u32 = 1_013_904_223;
    let mut state = NOISE_SEED;
    let noisy = speech
        .samples()
        .iter()
        .map(|&sample| {
            state = state
                .wrapping_mul(LCG_MULTIPLIER)
                .wrapping_add(LCG_INCREMENT);
            let noise =
                i32::try_from(state >> 16).unwrap() % (2 * HIGH_NOISE_PEAK + 1) - HIGH_NOISE_PEAK;
            i16::try_from(
                (i32::from(sample) + noise).clamp(i32::from(i16::MIN), i32::from(i16::MAX)),
            )
            .unwrap()
        })
        .collect();
    AudioBuffer16k::new(noisy)
}

pub(super) mod needs_stt {
    use super::*;

    mod needs_corpus {
        use super::*;

        #[test]
        fn should_transcribe_corpus_sample_through_real_service() {
            // Given a running service and the corpus
            let (base_url, first, audio) = real_service_and_first_corpus_sample();
            // When
            let text = transcribe_through_real_service(&base_url, &audio).to_lowercase();
            // Then
            let reference = first["reference"].as_str().unwrap();
            let last_word = reference.split_whitespace().last().unwrap();
            assert!(
                text.contains(last_word),
                "expected '{last_word}' in '{text}'"
            );
        }

        #[test]
        fn should_transcribe_the_same_noisy_recording_to_the_same_text_twice() {
            // Given a running service and the corpus
            let (base_url, _, speech) = real_service_and_first_corpus_sample();
            let noisy = under_high_noise(&speech);
            // When
            let first = transcribe_through_real_service(&base_url, &noisy);
            let second = transcribe_through_real_service(&base_url, &noisy);
            // Then
            assert_eq!(first, second);
        }
    }
}
