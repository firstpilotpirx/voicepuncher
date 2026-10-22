//! The request a confirmation is sent as: the utterance as 16-bit WAV in a multipart form
//! with the decoding fields of the speech recognition service.

use std::io::Cursor;

use anyhow::Result;

pub(super) const BOUNDARY: &str = "voicepuncher-command-spotting-3f9a";
const LANGUAGE: &str = "en";
// No prompt: with the command phrases as the prompt (whisper-server runs with `-mc 0`, which
// drops a prompt unless the request sets its own `max_context`), Russian speech such as
// "четыре", "пять" or "очисти" was heard as "clear all text." or "press enter.", the user's
// recorded speech gave four wrong commands instead of one, and the slowest confirmations of
// Russian speech took 1.7 s instead of 0.2 s (restore-recognition-prompt, design.md § Results).

/// The temperature step of a retry when a decoding fails (stuck repeating, or too unsure):
/// 0 turns the retries off. The service's default 0.2 retries at 0.2, 0.4, … 1.0, and
/// speech in another language that gets stuck repeating in English used those up for
/// seconds; a retry at 0.55 still took up to 3 s, since it got stuck as well.
const TEMPERATURE_STEP: &str = "0";

pub(super) fn wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
    for sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)?;
    }
    writer.finalize()?;
    Ok(cursor.into_inner())
}

pub(super) fn multipart_body(wav: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(wav.len() + 512);
    for (name, value) in [
        ("language", LANGUAGE),
        ("temperature", "0"),
        ("temperature_inc", TEMPERATURE_STEP),
        ("response_format", "verbose_json"),
        ("timestamp_granularities[]", "word"),
    ] {
        body.extend_from_slice(
            format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"utterance.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_put_the_fields_before_the_audio_in_the_request() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        let language = body.find("name=\"language\"").unwrap();
        let file = body.find("name=\"file\"").unwrap();
        assert!(language < file);
        assert!(body.contains("name=\"language\"\r\n\r\nen\r\n"));
        assert!(body.contains("name=\"temperature\"\r\n\r\n0\r\n"));
        assert!(body.contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
        assert!(body.contains("name=\"timestamp_granularities[]\"\r\n\r\nword\r\n"));
        assert!(body.ends_with(&format!("--{BOUNDARY}--\r\n")));
    }

    #[test]
    fn should_ask_for_no_retry_of_a_confirmation() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        assert!(body.contains("name=\"temperature\"\r\n\r\n0\r\n"));
        assert!(body.contains("name=\"temperature_inc\"\r\n\r\n0\r\n"));
    }

    #[test]
    fn should_send_no_prompt_with_a_confirmation() {
        // When
        let body = String::from_utf8_lossy(&multipart_body(b"RIFF")).into_owned();
        // Then
        assert!(!body.contains("name=\"prompt\""));
        assert!(!body.contains("name=\"max_context\""));
        assert!(!body.contains("name=\"carry_initial_prompt\""));
    }
}
