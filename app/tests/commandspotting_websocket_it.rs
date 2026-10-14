//! The command spotting adapter's client against the real command spotting service, on
//! speech synthesized with macOS `say`. `just e2e-smoke` starts the service and sets
//! `VOICEPUNCHER_COMMAND_SPOTTING_URL`; without it the service tests are skipped.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use tungstenite::Message;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::probe;
use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand, VoiceCommandRecognizer};
use voicepuncher_e2e_test::voice_commands::{
    recognized_commands, spoken_with_pauses, spotting_events,
};

/// A typical microphone: 48 kHz, delivered in CoreAudio-sized buffers.
const MIC_RATE_HZ: u32 = 48_000;
const MIC_BUFFER: usize = 512;
const SPEECH_RATE_HZ: u32 = 16_000;
/// How long the service may stay quiet before every event is taken to have arrived.
const QUIET: Duration = Duration::from_secs(2);

/// A loopback port nothing listens on.
const NOTHING_LISTENING: &str = "ws://127.0.0.1:1";

fn service_url() -> Option<String> {
    let url = std::env::var("VOICEPUNCHER_COMMAND_SPOTTING_URL").ok();
    if url.is_none() {
        eprintln!("GATED: VOICEPUNCHER_COMMAND_SPOTTING_URL not set, skipping");
    }
    url
}

#[test]
fn should_fail_a_probe_naming_the_service_when_nothing_answers() {
    // When
    let error = format!("{:#}", probe(NOTHING_LISTENING).unwrap_err());
    // Then
    assert!(
        error.contains("command spotting service not reachable"),
        "{error}"
    );
    assert!(error.contains("just start-model-servers"), "{error}");
}

#[test]
fn should_answer_a_probe() {
    let Some(url) = service_url() else { return };
    probe(&url).unwrap();
}

#[test]
fn should_report_a_spoken_phrase_between_the_bounds_of_its_speech() {
    let Some(url) = service_url() else { return };
    // Given
    let audio = spoken_with_pauses("press enter").unwrap();
    // When
    let events = spotting_events(&url, &audio, &VoiceCommand::STANDBY).unwrap();
    // Then
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| match e {
            SpottingEvent::SpeechStarted(_) => "speech started",
            SpottingEvent::PhraseSpotted { phrase, .. } => phrase.as_str(),
            SpottingEvent::SpeechEnded(_) => "speech ended",
            SpottingEvent::CommandsSaid { .. } => "commands said",
        })
        .collect();
    // The keyword spotter and the speech recognition of the utterance may both report it.
    let mut kinds = kinds;
    kinds.dedup();
    let speech_end = kinds.iter().position(|k| *k == "speech ended").unwrap();
    assert_eq!(
        kinds[..speech_end + 1],
        ["speech started", "press enter", "speech ended"]
    );
    assert!(
        kinds[speech_end + 1..].iter().all(|k| *k == "press enter"),
        "{kinds:?}"
    );
}

#[test]
fn should_be_refused_a_phrase_the_service_does_not_know() {
    let Some(url) = service_url() else { return };
    // Given
    let (mut socket, _) = tungstenite::connect(url.as_str()).unwrap();
    // When
    socket
        .send(Message::text(r#"{"phrases": ["open the door"]}"#))
        .unwrap();
    // Then
    let refusal = loop {
        match socket.read().unwrap() {
            Message::Close(frame) => break frame.map(|f| f.reason.to_string()).unwrap_or_default(),
            _ => continue,
        }
    };
    assert!(refusal.contains("open the door"), "{refusal}");
}

/// 16 kHz speech as a 48 kHz microphone would capture it.
fn at_microphone_rate(speech: &[f32]) -> Vec<f32> {
    let factor = (MIC_RATE_HZ / SPEECH_RATE_HZ) as usize;
    speech
        .windows(2)
        .flat_map(|pair| {
            (0..factor).map(move |i| pair[0] + (pair[1] - pair[0]) * i as f32 / factor as f32)
        })
        .collect()
}

/// The phrases the app listens for in the state where `command` is accepted.
fn listened_for(command: VoiceCommand) -> &'static [VoiceCommand] {
    if VoiceCommand::DICTATING.contains(&command) {
        &VoiceCommand::DICTATING
    } else {
        &VoiceCommand::STANDBY
    }
}

/// What the app recognizes when `words` reach it from a 48 kHz microphone.
fn recognized_from_microphone(
    url: &str,
    words: &str,
    commands: &[VoiceCommand],
) -> Vec<VoiceCommand> {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        url,
        commands,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )
    .unwrap();
    for buffer in at_microphone_rate(&spoken_with_pauses(words).unwrap()).chunks(MIC_BUFFER) {
        audio_tx.send(buffer.to_vec()).unwrap();
    }
    let mut events = Vec::new();
    while let Ok(input) = inputs_rx.recv_timeout(QUIET) {
        match input {
            PipelineInput::Spotting(event) => events.push(event),
            other => panic!("unexpected input {other:?}"),
        }
    }
    stream.stop();
    recognized_commands(events)
}

#[test]
fn should_recognize_each_command_streamed_from_a_48_khz_microphone() {
    let Some(url) = service_url() else { return };
    for command in VoiceCommand::ALL {
        // When
        let recognized = recognized_from_microphone(&url, command.phrase(), listened_for(command));
        // Then
        assert_eq!(recognized, [command], "spoken: {:?}", command.phrase());
    }
}

const LONG_LISTENING_SECS: f32 = 30.0;
const BETWEEN_COMMANDS_SECS: f32 = 5.0;
/// Longest the service may take to report a command after its audio was sent.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

fn microphone_silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; (secs * MIC_RATE_HZ as f32) as usize]
}

/// One listening session as the app runs it: a single connection, audio sent buffer by
/// buffer, the recognizer fed every event.
struct Listening {
    audio: mpsc::Sender<Vec<f32>>,
    inputs: mpsc::Receiver<PipelineInput>,
    recognizer: VoiceCommandRecognizer,
    stream: AudioStream,
}

impl Listening {
    fn start(url: &str, commands: &[VoiceCommand]) -> Self {
        let (audio, audio_rx) = mpsc::channel();
        let (inputs_tx, inputs) = mpsc::channel();
        let stream = AudioStream::start(
            url,
            commands,
            Instant::now(),
            MIC_RATE_HZ,
            audio_rx,
            inputs_tx,
        )
        .unwrap();
        Self {
            audio,
            inputs,
            recognizer: VoiceCommandRecognizer::new(),
            stream,
        }
    }

    fn hear(&self, samples: Vec<f32>) {
        for buffer in samples.chunks(MIC_BUFFER) {
            self.audio.send(buffer.to_vec()).unwrap();
        }
    }

    fn say_after(&self, pause_secs: f32, words: &str) {
        self.hear(microphone_silence(pause_secs));
        self.hear(at_microphone_rate(&spoken_with_pauses(words).unwrap()));
    }

    /// The next command the app recognizes, as the pipeline would get it.
    fn next_command(&mut self) -> Option<VoiceCommand> {
        let deadline = Instant::now() + COMMAND_TIMEOUT;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.inputs.recv_timeout(left) {
                Ok(PipelineInput::Spotting(event)) => {
                    if let Some(spoken) = self.recognizer.feed(event).first() {
                        return Some(spoken.command);
                    }
                }
                Ok(other) => panic!("unexpected input {other:?}"),
                Err(_) => return None,
            }
        }
        None
    }
}

#[test]
fn should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them() {
    let Some(url) = service_url() else { return };
    // Given listening for a long time with nothing said
    let mut listening = Listening::start(&url, &VoiceCommand::STANDBY);
    // When the user starts a dictation, stops it and sends the message
    listening.say_after(LONG_LISTENING_SECS, "start dictation");
    let started = listening.next_command();
    listening.stream.listen_for(&VoiceCommand::DICTATING);
    listening.say_after(BETWEEN_COMMANDS_SECS, "stop dictation");
    let stopped = listening.next_command();
    listening.stream.listen_for(&VoiceCommand::STANDBY);
    listening.say_after(BETWEEN_COMMANDS_SECS, "press enter");
    let entered = listening.next_command();
    listening.stream.stop();
    // Then
    assert_eq!(
        [started, stopped, entered],
        [
            Some(VoiceCommand::StartDictation),
            Some(VoiceCommand::StopDictation),
            Some(VoiceCommand::PressEnter)
        ]
    );
}

#[test]
fn should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening() {
    let Some(url) = service_url() else { return };
    // Given
    let mut listening = Listening::start(&url, &VoiceCommand::STANDBY);
    // When
    listening.say_after(LONG_LISTENING_SECS, "I will press enter when I am done");
    // Then
    assert_eq!(listening.next_command(), None);
    listening.stream.stop();
}

#[test]
fn should_take_a_command_phrase_that_ends_the_utterance() {
    let Some(url) = service_url() else { return };
    for (sentence, command) in [
        ("now start dictation", VoiceCommand::StartDictation),
        ("I think we should press enter", VoiceCommand::PressEnter),
        ("and then clear all text", VoiceCommand::ClearAllText),
    ] {
        // Given
        let mut listening = Listening::start(&url, &VoiceCommand::STANDBY);
        // When
        listening.say_after(BETWEEN_COMMANDS_SECS, sentence);
        // Then
        assert_eq!(
            listening.next_command(),
            Some(command),
            "said: {sentence:?}"
        );
        listening.stream.stop();
    }
}

/// The pause a quick user leaves between two commands.
const SHORT_PAUSE_SECS: f32 = 0.8;

#[test]
fn should_recognize_stop_dictation_and_press_enter_said_with_a_short_pause_in_real_time() {
    let Some(url) = service_url() else { return };
    // Given a dictation runs
    let mut listening = Listening::start(&url, &VoiceCommand::DICTATING);
    // When the user says both commands with a short pause, as the app switches phrase sets
    let speech = std::thread::scope(|scope| {
        let audio = listening.audio.clone();
        let speaker = scope.spawn(move || {
            let me = |samples: Vec<f32>| {
                let buffer_time = Duration::from_secs_f32(MIC_BUFFER as f32 / MIC_RATE_HZ as f32);
                for buffer in samples.chunks(MIC_BUFFER) {
                    audio.send(buffer.to_vec()).unwrap();
                    std::thread::sleep(buffer_time);
                }
            };
            me(at_microphone_rate(
                &spoken_with_pauses("stop dictation").unwrap(),
            ));
            me(microphone_silence(SHORT_PAUSE_SECS));
            me(at_microphone_rate(
                &spoken_with_pauses("press enter").unwrap(),
            ));
        });
        let stopped = listening.next_command();
        listening.stream.listen_for(&VoiceCommand::STANDBY);
        let entered = listening.next_command();
        speaker.join().unwrap();
        [stopped, entered]
    });
    listening.stream.stop();
    // Then
    assert_eq!(
        speech,
        [
            Some(VoiceCommand::StopDictation),
            Some(VoiceCommand::PressEnter)
        ]
    );
}

#[test]
fn should_serve_several_apps_listening_at_once() {
    let Some(url) = service_url() else { return };
    // When three apps send a command at the same time
    let recognized: Vec<Vec<VoiceCommand>> = std::thread::scope(|scope| {
        let handles: Vec<_> = [
            VoiceCommand::PressEnter,
            VoiceCommand::ClearAllText,
            VoiceCommand::StartDictation,
        ]
        .into_iter()
        .map(|command| {
            let url = url.clone();
            scope.spawn(move || {
                recognized_from_microphone(&url, command.phrase(), &VoiceCommand::STANDBY)
            })
        })
        .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // Then each gets its own command
    assert_eq!(
        recognized,
        [
            vec![VoiceCommand::PressEnter],
            vec![VoiceCommand::ClearAllText],
            vec![VoiceCommand::StartDictation]
        ]
    );
}

/// A service that accepts the connection, then hangs: it never reads or answers again.
fn hanging_service() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let socket = tungstenite::accept(stream).unwrap();
        std::thread::sleep(Duration::from_secs(600));
        drop(socket);
    });
    url
}

/// Longer than the app waits for a silent service before it gives up on it.
const HANG_NOTICED_WITHIN: Duration = Duration::from_secs(20);

#[test]
fn should_report_the_service_lost_when_it_stops_answering() {
    // Given the app streams to a service that hangs
    let url = hanging_service();
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        &url,
        &VoiceCommand::STANDBY,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )
    .unwrap();
    // When the microphone keeps capturing
    let deadline = Instant::now() + HANG_NOTICED_WITHIN;
    let mut lost = false;
    while !lost && Instant::now() < deadline {
        audio_tx.send(microphone_silence(0.1)).unwrap();
        lost = matches!(
            inputs_rx.recv_timeout(Duration::from_millis(100)),
            Ok(PipelineInput::SpottingLost)
        );
    }
    // Then
    assert!(
        lost,
        "the hanging service was not noticed within {HANG_NOTICED_WITHIN:?}"
    );
    stream.stop();
}

/// Longer than the silence the app allows the service before it gives up on it.
const QUIET_SESSION: Duration = Duration::from_secs(15);

#[test]
fn should_keep_a_live_service_that_hears_nothing_for_a_while() {
    let Some(url) = service_url() else { return };
    // Given
    let (_audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        &url,
        &VoiceCommand::STANDBY,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )
    .unwrap();
    // When nothing is captured for longer than the silence limit
    let input = inputs_rx.recv_timeout(QUIET_SESSION);
    // Then
    assert!(input.is_err(), "unexpected input {input:?}");
    stream.stop();
}

#[test]
fn should_start_a_dictation_with_start_opening_the_utterance() {
    let Some(url) = service_url() else { return };
    for sentence in [
        "start dictation again",
        "start dictation for the next message",
    ] {
        // Given
        let mut listening = Listening::start(&url, &VoiceCommand::STANDBY);
        // When
        listening.say_after(BETWEEN_COMMANDS_SECS, sentence);
        // Then
        assert_eq!(
            listening.next_command(),
            Some(VoiceCommand::StartDictation),
            "said: {sentence:?}"
        );
        listening.stream.stop();
    }
}

#[test]
fn should_ignore_a_command_phrase_with_more_speech_after_it() {
    let Some(url) = service_url() else { return };
    // A single short word right after a command ("press enter now") is within the margin
    // quiet speech needs and still passes: a known limit of the pause rule.
    for sentence in [
        "press enter when you are ready",
        "clear all text in the morning please",
    ] {
        // Given
        let mut listening = Listening::start(&url, &VoiceCommand::STANDBY);
        // When
        listening.say_after(BETWEEN_COMMANDS_SECS, sentence);
        // Then
        assert_eq!(listening.next_command(), None, "said: {sentence:?}");
        listening.stream.stop();
    }
}
