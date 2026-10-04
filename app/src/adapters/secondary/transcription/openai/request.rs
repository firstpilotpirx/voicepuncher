use std::io::Cursor;

use anyhow::{Context, Result};

use crate::config::TranscriptionCfg;
use crate::core::domain::AudioBuffer16k;

pub const MULTIPART_BOUNDARY: &str = "voicepuncher-7d1f3a9c5e2b";
const WAV_FILENAME: &str = "audio.wav";
const WAV_CONTENT_TYPE: &str = "audio/wav";
const RESPONSE_FORMAT: &str = "json";
const BITS_PER_SAMPLE: u16 = 16;
const MONO: u16 = 1;

pub fn content_type() -> String {
    format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}")
}

/// Encodes the buffer as a 16 kHz mono 16-bit PCM WAV file in memory.
pub fn encode_wav(audio: &AudioBuffer16k) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: MONO,
        sample_rate: AudioBuffer16k::SAMPLE_RATE_HZ,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec).context("start WAV encoding")?;
    for &sample in audio.samples() {
        writer.write_sample(sample).context("encode WAV sample")?;
    }
    writer.finalize().context("finish WAV encoding")?;
    Ok(cursor.into_inner())
}

/// Builds the OpenAI Audio Transcriptions multipart body: text fields, then the WAV file.
pub fn multipart_body(cfg: &TranscriptionCfg, wav: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(wav.len() + 1024);
    push_field(&mut body, "model", &cfg.model);
    if cfg.language != TranscriptionCfg::AUTO_LANGUAGE {
        push_field(&mut body, "language", &cfg.language);
    }
    if !cfg.prompt.is_empty() {
        push_field(&mut body, "prompt", &cfg.prompt);
    }
    push_field(&mut body, "temperature", &cfg.temperature.to_string());
    push_field(&mut body, "response_format", RESPONSE_FORMAT);
    body.extend_from_slice(
        format!(
            "--{MULTIPART_BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; \
             filename=\"{WAV_FILENAME}\"\r\nContent-Type: {WAV_CONTENT_TYPE}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    body
}

fn push_field(body: &mut Vec<u8>, name: &str, value: &str) {
    body.extend_from_slice(
        format!(
            "--{MULTIPART_BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        )
        .as_bytes(),
    );
}

/// Joins the service's segments back into one line. whisper-server separates segments
/// with `\n`, sometimes inside a word, and starts a segment with a space at word
/// boundaries, so dropping the newline restores both cases.
pub fn normalize_transcript(text: &str) -> String {
    text.replace('\n', "").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_text(cfg: &TranscriptionCfg) -> String {
        String::from_utf8_lossy(&multipart_body(cfg, b"RIFF")).into_owned()
    }

    #[test]
    fn should_encode_wav_header_for_16khz_mono_pcm16() {
        // Given
        let audio = AudioBuffer16k::new(vec![0, 1, -1, i16::MAX]);
        // When
        let wav = encode_wav(&audio).unwrap();
        // Then
        let spec = hound::WavReader::new(Cursor::new(&wav)).unwrap().spec();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(wav.len(), 44 + 4 * 2);
    }

    #[test]
    fn should_include_model_temperature_format_and_file_parts() {
        // Given
        let cfg = TranscriptionCfg {
            model: "whisper-1".into(),
            ..TranscriptionCfg::default()
        };
        // When
        let body = body_text(&cfg);
        // Then
        assert!(body.contains("name=\"model\"\r\n\r\nwhisper-1\r\n"));
        assert!(body.contains("name=\"temperature\"\r\n\r\n0\r\n"));
        assert!(body.contains("name=\"response_format\"\r\n\r\njson\r\n"));
        assert!(body.contains(
            "name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF\r\n"
        ));
        assert!(body.ends_with(&format!("--{MULTIPART_BOUNDARY}--\r\n")));
    }

    #[test]
    fn should_omit_language_for_auto_and_empty_prompt() {
        // Given
        let cfg = TranscriptionCfg::default();
        // When
        let body = body_text(&cfg);
        // Then
        assert!(!body.contains("name=\"language\""));
        assert!(!body.contains("name=\"prompt\""));
    }

    #[test]
    fn should_send_fixed_language_and_prompt() {
        // Given
        let cfg = TranscriptionCfg {
            language: "en".into(),
            prompt: "Docker, Kubernetes".into(),
            ..TranscriptionCfg::default()
        };
        // When
        let body = body_text(&cfg);
        // Then
        assert!(body.contains("name=\"language\"\r\n\r\nen\r\n"));
        assert!(body.contains("name=\"prompt\"\r\n\r\nDocker, Kubernetes\r\n"));
    }

    #[test]
    fn should_join_word_split_across_segments() {
        assert_eq!(normalize_transcript("наход\nившаяся"), "находившаяся");
    }

    #[test]
    fn should_keep_space_between_words_across_segments() {
        assert_eq!(
            normalize_transcript(" обозначения\n периодов\n"),
            "обозначения периодов"
        );
    }
}
