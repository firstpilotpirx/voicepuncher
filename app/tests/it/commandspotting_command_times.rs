//! The command spotting adapter dates a command's start by the moment its audio was
//! captured, also when the audio it streams runs behind the clock: the dictation is cut
//! there, so no word said before the command is lost.

use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

use voicepuncher::adapters::secondary::commandspotting::websocket::{AudioStream, MicrophoneAudio};
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::conversions::saturating_usize;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand};

use crate::fake_websocket::{nothing_yet, serve_one};

const MIC_RATE_HZ: u32 = 48_000;
const MIC_BUFFER_SECS: f32 = 0.1;
const SAMPLE_RATE_HZ: f32 = 16_000.0;

/// How far the stream's first sample is dated before the moment it was really captured.
const STREAM_BEHIND: Duration = Duration::from_millis(800);

/// How far the command's start may fall from the capture of its first sample: a block's
/// length, the resampler's delay and the stream thread's read.
const START_TOLERANCE: Duration = Duration::from_millis(300);

/// The stream heard this much audio within this, however busy the machine.
const REPORTED_WITHIN: Duration = Duration::from_secs(15);

/// A service that reports "press enter" from `start` seconds of the stream on, once it has
/// received the audio up to a quarter second past it.
fn service_reporting_enter_at(start: f32) -> String {
    serve_one(move |mut socket| {
        let mut received = 0_usize;
        let needed = saturating_usize((start + 0.25) * SAMPLE_RATE_HZ);
        while received < needed {
            match socket.read() {
                Ok(Message::Binary(bytes)) => received += bytes.len() / 4,
                Ok(_) => {}
                Err(e) if nothing_yet(&e) => {}
                Err(_) => return,
            }
        }
        report_enter(&mut socket, start);
        keep_reading(&mut socket);
    })
}

/// Reads on until the client goes, so the connection stays open.
fn keep_reading(socket: &mut WebSocket<TcpStream>) {
    loop {
        match socket.read() {
            Err(e) if !nothing_yet(&e) => return,
            _ => {}
        }
    }
}

fn report_enter(socket: &mut WebSocket<TcpStream>, start: f32) {
    let json = format!(
        r#"{{"event":"command","commands":["press enter"],"start":{start},"end":{},"placement":"tail"}}"#,
        start + 0.25
    );
    socket.send(Message::text(json)).unwrap();
}

/// A stream whose first sample is dated `STREAM_BEHIND` before it is fed, as a microphone that
/// started late or lost audio leaves it.
fn stream_behind_the_clock(url: &str) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        url,
        &VoiceCommand::DICTATING,
        MicrophoneAudio {
            audio: audio_rx,
            native_rate: MIC_RATE_HZ,
            captured_from: Instant::now() - STREAM_BEHIND,
        },
        inputs_tx,
    )
    .unwrap();
    (stream, audio_tx, inputs_rx)
}

/// Feeds the microphone in real time until the command arrives; its start.
fn command_start(audio: &Sender<Vec<f32>>, inputs: &Receiver<PipelineInput>) -> Option<Instant> {
    let deadline = Instant::now() + REPORTED_WITHIN;
    let buffer = vec![0.1_f32; saturating_usize(MIC_BUFFER_SECS * MIC_RATE_HZ as f32)];
    while Instant::now() < deadline {
        audio.send(buffer.clone()).unwrap();
        let next_buffer = Instant::now() + Duration::from_secs_f32(MIC_BUFFER_SECS);
        while let Ok(input) =
            inputs.recv_timeout(next_buffer.saturating_duration_since(Instant::now()))
        {
            if let PipelineInput::Spotting(SpottingEvent::CommandsSaid { start, .. }) = input {
                return Some(start);
            }
        }
    }
    None
}

/// How far the reported start of a command at `start` seconds is from the capture of its
/// first sample, on a stream behind the clock.
fn start_error(start: f32) -> Duration {
    let url = service_reporting_enter_at(start);
    let (_stream, audio, inputs) = stream_behind_the_clock(&url);
    let fed_from = Instant::now();
    let reported = command_start(&audio, &inputs).expect("the command is reported");
    let captured = fed_from + Duration::from_secs_f32(start);
    reported.max(captured) - reported.min(captured)
}

#[test]
fn should_date_a_short_phrase_command_start_by_its_capture_on_a_stream_behind_the_clock() {
    let error = start_error(1.78);

    assert!(
        error <= START_TOLERANCE,
        "the command's start is {error:?} from the capture of its first sample"
    );
}

#[test]
fn should_date_a_command_start_after_a_long_dictation_by_its_capture_on_a_stream_behind_the_clock()
{
    let error = start_error(3.97);

    assert!(
        error <= START_TOLERANCE,
        "the command's start is {error:?} from the capture of its first sample"
    );
}
