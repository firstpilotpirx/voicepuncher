//! Synthesized test speech under recording conditions, and the seam metrics that judge a
//! transcript of it: phrases repeated at a chunk seam, and spoken words that went missing.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const SAMPLE_RATE_HZ: u32 = 16_000;

const TEXT_RU: &str =
    "Сегодня утром я проверил очередь задач и понял, что половина из них устарела. \
Мы договорились с командой, что будем пересматривать список каждую пятницу после обеда. \
Потом я открыл логи сервера и увидел, что база данных отвечает медленнее обычного. \
Оказалось, что индекс по полю даты создания так и не построили после миграции. \
Я добавил индекс, перезапустил сервис и время ответа упало с двух секунд до сорока миллисекунд. \
Вечером мы обсуждали, как разбить большой модуль оплаты на несколько маленьких частей. \
Каждая часть должна отвечать только за одну вещь и иметь понятный интерфейс. \
Ещё нужно написать тесты на возврат денег, потому что там уже дважды находили ошибки. \
В конце дня я обновил документацию и отправил короткий отчёт руководителю проекта.";

const TEXT_EN: &str = "This morning I went through the task queue and realised that half of it was out of date. \
We agreed with the team to review the list every Friday afternoon. \
Then I opened the server logs and saw that the database was answering slower than usual. \
It turned out that the index on the creation date column was never built after the migration. \
I added the index, restarted the service, and the response time dropped from two seconds to forty milliseconds. \
In the evening we discussed how to split the large payment module into several small parts. \
Each part should be responsible for one thing only and have a clear interface. \
We also need tests for refunds, because bugs were found there twice already. \
At the end of the day I updated the documentation and sent a short report to the project lead.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Ru, Lang::En];

    pub fn text(self) -> &'static str {
        match self {
            Lang::Ru => TEXT_RU,
            Lang::En => TEXT_EN,
        }
    }

    fn voice(self) -> &'static str {
        match self {
            Lang::Ru => "Milena",
            Lang::En => "Eddy",
        }
    }
}

/// How the speech is spoken or how the microphone hears it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Normal,
    Slow,
    Fast,
    SentencePauses,
    WordPauses,
    NoiseLow,
    NoiseHigh,
    PhoneBand,
    Quiet,
    Clipped,
    Dropouts,
}

impl Condition {
    pub const ALL: [Condition; 11] = [
        Condition::Normal,
        Condition::Slow,
        Condition::Fast,
        Condition::SentencePauses,
        Condition::WordPauses,
        Condition::NoiseLow,
        Condition::NoiseHigh,
        Condition::PhoneBand,
        Condition::Quiet,
        Condition::Clipped,
        Condition::Dropouts,
    ];

    fn words_per_minute(self) -> u32 {
        match self {
            Condition::Slow => 140,
            Condition::Fast => 260,
            Condition::WordPauses => 170,
            _ => 190,
        }
    }

    /// `say` input: the text with silence markers for the pause conditions.
    fn say_input(self, text: &str) -> String {
        match self {
            Condition::SentencePauses => text.replace(". ", ". [[slnc 1500]] "),
            Condition::WordPauses => text
                .split_whitespace()
                .map(|w| format!("{w} [[slnc 350]]"))
                .collect::<Vec<_>>()
                .join(" "),
            _ => text.to_owned(),
        }
    }

    fn degrade(self, samples: &mut Vec<i16>) {
        match self {
            Condition::NoiseLow => add_noise(samples, 0.01),
            Condition::NoiseHigh => add_noise(samples, 0.05),
            Condition::PhoneBand => phone_band(samples),
            Condition::Quiet => gain(samples, 0.15),
            Condition::Clipped => gain(samples, 6.0),
            Condition::Dropouts => dropouts(samples),
            _ => {}
        }
    }
}

/// Speech of `lang`'s paragraph under `condition`, 16 kHz mono, after `lead_in_secs` of silence.
/// The lead-in moves the chunk seams onto other words.
pub fn synthesize(lang: Lang, condition: Condition, lead_in_secs: f32) -> Result<Vec<i16>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("speech.wav");
    let status = Command::new("say")
        .args(["-v", lang.voice(), "-r"])
        .arg(condition.words_per_minute().to_string())
        .args(["--file-format=WAVE", "--data-format=LEI16@16000", "-o"])
        .arg(&path)
        .arg(condition.say_input(lang.text()))
        .status()
        .context("failed to run macOS 'say'")?;
    if !status.success() {
        bail!("'say' failed to synthesize speech ({status})");
    }
    let speech = read_wav(&path)?;
    let mut samples = vec![0i16; (lead_in_secs * SAMPLE_RATE_HZ as f32) as usize];
    samples.extend(speech);
    condition.degrade(&mut samples);
    Ok(samples)
}

pub fn read_wav(path: &Path) -> Result<Vec<i16>> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE_HZ || spec.channels != 1 || spec.bits_per_sample != 16 {
        bail!("{} is not 16 kHz mono 16-bit", path.display());
    }
    reader
        .samples::<i16>()
        .collect::<Result<_, _>>()
        .context("read WAV samples")
}

pub fn write_wav(path: &Path, samples: &[i16]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE_HZ,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

/// Deterministic xorshift, so every run hears the same noise and dropouts.
struct Rng(u64);

impl Rng {
    fn next_unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn clamp_i16(v: f32) -> i16 {
    v.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Low-passed white noise (a pink-ish hum) at `amplitude` of full scale.
fn add_noise(samples: &mut [i16], amplitude: f32) {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut smooth = 0.0f32;
    for s in samples.iter_mut() {
        let white = rng.next_unit() * 2.0 - 1.0;
        smooth = 0.8 * smooth + 0.2 * white;
        let noise = smooth * 2.5 * amplitude * i16::MAX as f32;
        *s = clamp_i16(*s as f32 + noise);
    }
}

fn gain(samples: &mut [i16], factor: f32) {
    for s in samples.iter_mut() {
        *s = clamp_i16(*s as f32 * factor);
    }
}

/// 120 ms of silence every 0.6–1.7 s, like a microphone that drops out.
fn dropouts(samples: &mut [i16]) {
    let rate = SAMPLE_RATE_HZ as f32;
    let len = (0.12 * rate) as usize;
    let mut rng = Rng(7);
    let mut at = (rate * (0.5 + rng.next_unit() * 1.2)) as usize;
    while at < samples.len() {
        let end = (at + len).min(samples.len());
        samples[at..end].fill(0);
        at += (rate * (0.6 + rng.next_unit() * 1.1)) as usize;
    }
}

/// Telephone line: 300–3400 Hz band, sampled at 8 kHz and back.
fn phone_band(samples: &mut Vec<i16>) {
    let mut x: Vec<f32> = samples.iter().map(|&s| s as f32).collect();
    for _ in 0..2 {
        biquad(&mut x, Biquad::high_pass(300.0));
        biquad(&mut x, Biquad::low_pass(3400.0));
    }
    for pair in x.chunks_mut(2) {
        let held = pair[0];
        pair.iter_mut().for_each(|v| *v = held);
    }
    *samples = x.into_iter().map(clamp_i16).collect();
}

struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
}

impl Biquad {
    const Q: f32 = std::f32::consts::FRAC_1_SQRT_2;

    fn coefficients(freq: f32, high: bool) -> Self {
        let w = 2.0 * std::f32::consts::PI * freq / SAMPLE_RATE_HZ as f32;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * Self::Q);
        let a0 = 1.0 + alpha;
        let (b0, b1) = if high {
            ((1.0 + cos) / 2.0, -(1.0 + cos))
        } else {
            ((1.0 - cos) / 2.0, 1.0 - cos)
        };
        Self {
            b: [b0 / a0, b1 / a0, b0 / a0],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
        }
    }

    fn high_pass(freq: f32) -> Self {
        Self::coefficients(freq, true)
    }

    fn low_pass(freq: f32) -> Self {
        Self::coefficients(freq, false)
    }
}

fn biquad(x: &mut [f32], f: Biquad) {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    for v in x.iter_mut() {
        let x0 = *v;
        let y0 = f.b[0] * x0 + f.b[1] * x1 + f.b[2] * x2 - f.a[0] * y1 - f.a[1] * y2;
        (x2, x1, y2, y1) = (x1, x0, y1, y0);
        *v = y0;
    }
}

/// Words lowercased, `ё` read as `е`, every non-alphanumeric character dropped.
pub fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.to_lowercase()
                .replace('ё', "е")
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word pairs that occur more often in the transcript than in the spoken text — the trace
/// a chunk seam leaves when it keeps the overlap twice.
pub fn repeated_phrases(spoken: &str, transcript: &str) -> Vec<String> {
    use std::collections::HashMap;
    fn pairs(words: &[String]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for w in words.windows(2) {
            *counts.entry(format!("{} {}", w[0], w[1])).or_insert(0) += 1;
        }
        counts
    }
    let spoken = pairs(&normalized_words(spoken));
    let mut repeated: Vec<String> = pairs(&normalized_words(transcript))
        .into_iter()
        .filter(|(pair, n)| *n > 1 && *n > spoken.get(pair).copied().unwrap_or(0))
        .map(|(pair, _)| pair)
        .collect();
    repeated.sort();
    repeated
}

/// Spoken words of the longest phrase the transcript dropped: in every gap between two
/// words both texts share in order, the spoken words beyond those the transcript puts in
/// their place. A misheard phrase fills its gap and counts nothing.
pub fn dropped_phrase_len(spoken: &str, transcript: &str) -> usize {
    let s = normalized_words(spoken);
    let t = normalized_words(transcript);
    // lcs[i][j]: longest common subsequence of s[i..] and t[j..]
    let mut lcs = vec![vec![0usize; t.len() + 1]; s.len() + 1];
    for i in (0..s.len()).rev() {
        for j in (0..t.len()).rev() {
            lcs[i][j] = if s[i] == t[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let (mut gap_start_s, mut gap_start_t) = (0, 0);
    let mut longest = 0;
    while i < s.len() && j < t.len() {
        if s[i] == t[j] {
            longest = longest.max((i - gap_start_s).saturating_sub(j - gap_start_t));
            i += 1;
            j += 1;
            (gap_start_s, gap_start_t) = (i, j);
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    longest.max((s.len() - gap_start_s).saturating_sub(t.len() - gap_start_t))
}

/// Share of spoken words that the transcript does not contain in order (longest common
/// subsequence of normalized words).
pub fn missing_word_share(spoken: &str, transcript: &str) -> f64 {
    let s = normalized_words(spoken);
    let t = normalized_words(transcript);
    if s.is_empty() {
        return 0.0;
    }
    let mut row = vec![0usize; t.len() + 1];
    for sw in &s {
        let mut diagonal = 0;
        for (j, tw) in t.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if sw == tw {
                diagonal + 1
            } else {
                above.max(row[j])
            };
            diagonal = above;
        }
    }
    (s.len() - row[t.len()]) as f64 / s.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_find_a_phrase_repeated_at_a_seam() {
        let spoken = "у которой миллион граней и мы на эти грани можем";
        let transcript = "у которой миллион граней. Миллион граней, и мы на эти грани можем";
        assert_eq!(repeated_phrases(spoken, transcript), vec!["миллион граней"]);
    }

    #[test]
    fn should_ignore_a_phrase_the_speech_itself_repeats() {
        let spoken = "да да конечно да да";
        assert!(repeated_phrases(spoken, "Да, да, конечно. Да, да.").is_empty());
    }

    #[test]
    fn should_report_no_missing_words_for_the_same_text() {
        assert_eq!(missing_word_share("Hello, World!", "hello world"), 0.0);
    }

    #[test]
    fn should_count_dropped_words() {
        let share = missing_word_share("one two three four", "one four");
        assert!((share - 0.5).abs() < 1e-9, "{share}");
    }

    #[test]
    fn should_count_a_dropped_phrase() {
        assert_eq!(dropped_phrase_len("a b c d e f g h", "a b g h"), 4);
    }

    #[test]
    fn should_not_count_a_misheard_phrase() {
        let spoken = "индекс по полю даты создания так и не построили";
        let transcript = "индекс попа людат и создание так и не построили";
        assert_eq!(dropped_phrase_len(spoken, transcript), 0);
    }

    #[test]
    fn should_count_a_phrase_replaced_by_a_shorter_one() {
        let spoken = "at the end of a day I updated a report to the lead";
        let transcript = "at the Jordan reports to the lead";
        assert_eq!(dropped_phrase_len(spoken, transcript), 6);
    }

    #[test]
    fn should_count_words_dropped_at_the_end() {
        assert_eq!(dropped_phrase_len("one two three four five", "one two"), 3);
    }

    #[test]
    fn should_read_yo_as_ye() {
        assert_eq!(normalized_words("Ещё отчёт"), vec!["еще", "отчет"]);
    }

    #[test]
    fn should_synthesize_speech_longer_than_the_chunk_threshold_in_every_condition() {
        std::thread::scope(|scope| {
            for lang in Lang::ALL {
                for condition in Condition::ALL {
                    scope.spawn(move || {
                        let samples = synthesize(lang, condition, 0.0).unwrap();
                        let secs = samples.len() as f32 / SAMPLE_RATE_HZ as f32;
                        assert!(secs > 30.0, "{lang:?} {condition:?}: only {secs:.1} s");
                    });
                }
            }
        });
    }
}
