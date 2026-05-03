use std::path::PathBuf;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::core::application::ports::audio_recorder::AudioRecorder;
use crate::core::domain::AudioBuffer16k;

const RINGBUF_CAPACITY: usize = 1_920_000;

pub struct CpalAudioRecorder {
    debug_save_wav: Option<PathBuf>,
    active: Option<ActiveRecording>,
}

struct ActiveRecording {
    stream: Stream,
    native_sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
    consumer: ringbuf::HeapCons<i16>,
}

impl CpalAudioRecorder {
    pub fn new(debug_save_wav: Option<PathBuf>) -> Self {
        Self {
            debug_save_wav,
            active: None,
        }
    }
}

impl AudioRecorder for CpalAudioRecorder {
    fn start(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default input device available"))?;

        let supported_config = device.default_input_config()?;
        let native_sample_rate = supported_config.sample_rate();
        let channels = supported_config.channels();

        let stream_config = StreamConfig {
            channels,
            sample_rate: native_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let rb = HeapRb::<i16>::new(RINGBUF_CAPACITY);
        let (mut producer, consumer) = rb.split();

        let channels_usize = channels as usize;
        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono_frames = if channels_usize == 1 {
                    data.iter().map(|&s| float_to_i16(s)).collect::<Vec<i16>>()
                } else {
                    data.chunks_exact(channels_usize)
                        .map(|frame| {
                            let avg = frame.iter().copied().sum::<f32>() / channels_usize as f32;
                            float_to_i16(avg)
                        })
                        .collect::<Vec<i16>>()
                };
                for sample in mono_frames {
                    let _ = producer.try_push(sample);
                }
            },
            |err| {
                log::error!("cpal input stream error: {err}");
            },
            None,
        )?;

        stream.play()?;

        self.active = Some(ActiveRecording {
            stream,
            native_sample_rate,
            channels,
            consumer,
        });

        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let mut recording = self
            .active
            .take()
            .ok_or_else(|| anyhow!("recorder was not started"))?;

        drop(recording.stream);

        let mut raw_samples = Vec::with_capacity(RINGBUF_CAPACITY);
        while let Some(s) = recording.consumer.try_pop() {
            raw_samples.push(s);
        }

        let resampled = if recording.native_sample_rate == AudioBuffer16k::SAMPLE_RATE_HZ {
            raw_samples
        } else {
            resample_to_16k(raw_samples, recording.native_sample_rate)?
        };

        if let Some(ref path) = self.debug_save_wav {
            save_wav(path, &resampled)?;
        }

        Ok(AudioBuffer16k::new(resampled))
    }
}

fn float_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 32_767.0) as i16
}

fn resample_to_16k(mono_i16: Vec<i16>, native_rate: u32) -> Result<Vec<i16>> {
    let ratio = AudioBuffer16k::SAMPLE_RATE_HZ as f64 / native_rate as f64;
    let chunk_size = 1024_usize;
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(ratio, 1.0, params, chunk_size, 1)
        .map_err(|e| anyhow!("resampler construction failed: {e}"))?;

    let input_f32: Vec<f32> = mono_i16.iter().map(|&s| s as f32 / 32_767.0).collect();

    let total_frames = input_f32.len();
    let mut output_samples: Vec<f32> =
        Vec::with_capacity(((total_frames as f64) * ratio).ceil() as usize + chunk_size);

    let mut pos = 0usize;
    while pos < total_frames {
        let end = (pos + chunk_size).min(total_frames);
        let chunk = &input_f32[pos..end];

        let padded: Vec<f32> = if chunk.len() < chunk_size {
            let mut p = chunk.to_vec();
            p.resize(chunk_size, 0.0);
            p
        } else {
            chunk.to_vec()
        };

        let input_channels = vec![padded];
        let output = resampler
            .process(&input_channels, None)
            .map_err(|e| anyhow!("resampler process failed: {e}"))?;

        output_samples.extend_from_slice(&output[0]);
        pos += chunk_size;
    }

    let result = output_samples.into_iter().map(float_to_i16).collect();

    Ok(result)
}

fn save_wav(path: &PathBuf, samples: &[i16]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: AudioBuffer16k::SAMPLE_RATE_HZ,
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
