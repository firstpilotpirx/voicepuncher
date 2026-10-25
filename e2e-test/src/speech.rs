//! Synthesized test speech under recording conditions, and the seam metrics that judge a
//! transcript of it: phrases repeated at a chunk seam, and spoken words that went missing.

use anyhow::Result;

use crate::speech_cache::{synthesized, SayRequest};
pub use voicepuncher_test_steps::speech_cache::{read_wav, write_wav};

mod transcript_checks;
use crate::conversions::{saturating_i16, saturating_usize};
pub use transcript_checks::*;

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
    pub const ALL: [Self; 2] = [Self::Ru, Self::En];

    pub fn text(self) -> &'static str {
        match self {
            Self::Ru => TEXT_RU,
            Self::En => TEXT_EN,
        }
    }

    fn voice(self) -> &'static str {
        match self {
            Self::Ru => "Milena",
            Self::En => "Eddy",
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
    pub const ALL: [Self; 11] = [
        Self::Normal,
        Self::Slow,
        Self::Fast,
        Self::SentencePauses,
        Self::WordPauses,
        Self::NoiseLow,
        Self::NoiseHigh,
        Self::PhoneBand,
        Self::Quiet,
        Self::Clipped,
        Self::Dropouts,
    ];

    fn words_per_minute(self) -> u32 {
        match self {
            Self::Slow => 140,
            Self::Fast => 260,
            Self::WordPauses => 170,
            _ => 190,
        }
    }

    /// `say` input: the text with silence markers for the pause conditions.
    fn say_input(self, text: &str) -> String {
        match self {
            Self::SentencePauses => text.replace(". ", ". [[slnc 1500]] "),
            Self::WordPauses => text
                .split_whitespace()
                .map(|w| format!("{w} [[slnc 350]]"))
                .collect::<Vec<_>>()
                .join(" "),
            _ => text.to_owned(),
        }
    }

    fn degrade(self, samples: &mut Vec<i16>) {
        match self {
            Self::NoiseLow => add_noise(samples, 0.01),
            Self::NoiseHigh => add_noise(samples, 0.05),
            Self::PhoneBand => phone_band(samples),
            Self::Quiet => gain(samples, 0.15),
            Self::Clipped => gain(samples, 6.0),
            Self::Dropouts => dropouts(samples),
            _ => {}
        }
    }
}

/// Speech of `lang`'s paragraph under `condition`, 16 kHz mono, after `lead_in_secs` of silence.
/// The lead-in moves the chunk seams onto other words.
pub fn synthesize(lang: Lang, condition: Condition, lead_in_secs: f32) -> Result<Vec<i16>> {
    let speech = say(lang, condition)?;
    Ok(after_lead_in(&speech, condition, lead_in_secs).0)
}

/// [`synthesize`] after each of `lead_ins_secs`, together with the same speech before
/// `condition` degraded it: where the clean copy is silent, the speech pauses. `say` runs once.
pub fn synthesize_with_clean(
    lang: Lang,
    condition: Condition,
    lead_ins_secs: &[f32],
) -> Result<Vec<(Vec<i16>, Vec<i16>)>> {
    let speech = say(lang, condition)?;
    Ok(lead_ins_secs
        .iter()
        .map(|&lead_in| after_lead_in(&speech, condition, lead_in))
        .collect())
}

/// `speech` after `lead_in_secs` of silence, degraded by `condition`, and the clean copy.
fn after_lead_in(speech: &[i16], condition: Condition, lead_in_secs: f32) -> (Vec<i16>, Vec<i16>) {
    let mut samples = vec![0i16; saturating_usize(lead_in_secs * SAMPLE_RATE_HZ as f32)];
    samples.extend_from_slice(speech);
    let clean = samples.clone();
    condition.degrade(&mut samples);
    (samples, clean)
}

/// `lang`'s paragraph spoken by macOS `say` as `condition` asks, 16 kHz mono.
fn say(lang: Lang, condition: Condition) -> Result<Vec<i16>> {
    synthesized(&SayRequest {
        voice: Some(lang.voice()),
        words_per_minute: Some(condition.words_per_minute()),
        text: &condition.say_input(lang.text()),
    })
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
    saturating_i16(v.round().clamp(f32::from(i16::MIN), f32::from(i16::MAX)))
}

/// Low-passed white noise (a pink-ish hum) at `amplitude` of full scale.
fn add_noise(samples: &mut [i16], amplitude: f32) {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut smooth = 0.0f32;
    for s in samples.iter_mut() {
        let white = rng.next_unit() * 2.0 - 1.0;
        smooth = 0.8 * smooth + 0.2 * white;
        let noise = smooth * 2.5 * amplitude * f32::from(i16::MAX);
        *s = clamp_i16(f32::from(*s) + noise);
    }
}

fn gain(samples: &mut [i16], factor: f32) {
    for s in samples.iter_mut() {
        *s = clamp_i16(f32::from(*s) * factor);
    }
}

/// 120 ms of silence every 0.6–1.7 s, like a microphone that drops out.
fn dropouts(samples: &mut [i16]) {
    let rate = SAMPLE_RATE_HZ as f32;
    let len = saturating_usize(0.12 * rate);
    let mut rng = Rng(7);
    let mut at = saturating_usize(rate * (0.5 + rng.next_unit() * 1.2));
    while at < samples.len() {
        let end = (at + len).min(samples.len());
        samples[at..end].fill(0);
        at += saturating_usize(rate * (0.6 + rng.next_unit() * 1.1));
    }
}

/// Telephone line: 300–3400 Hz band, sampled at 8 kHz and back.
fn phone_band(samples: &mut Vec<i16>) {
    let mut x: Vec<f32> = samples.iter().map(|&s| f32::from(s)).collect();
    for _ in 0..2 {
        biquad(&mut x, &Biquad::high_pass(300.0));
        biquad(&mut x, &Biquad::low_pass(3400.0));
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
            (f32::midpoint(1.0, cos), -(1.0 + cos))
        } else {
            ((1.0 - cos) / 2.0, 1.0 - cos)
        };
        Self { b: [b0 / a0, b1 / a0, b0 / a0], a: [-2.0 * cos / a0, (1.0 - alpha) / a0] }
    }

    fn high_pass(freq: f32) -> Self {
        Self::coefficients(freq, true)
    }

    fn low_pass(freq: f32) -> Self {
        Self::coefficients(freq, false)
    }
}

fn biquad(x: &mut [f32], f: &Biquad) {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    for v in x.iter_mut() {
        let x0 = *v;
        let y0 = f.b[0] * x0 + f.b[1] * x1 + f.b[2] * x2 - f.a[0] * y1 - f.a[1] * y2;
        (x2, x1, y2, y1) = (x1, x0, y1, y0);
        *v = y0;
    }
}

#[cfg(test)]
mod tests;
