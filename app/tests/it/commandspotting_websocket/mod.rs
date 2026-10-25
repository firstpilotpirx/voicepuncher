//! The command spotting adapter's client against the real command spotting service, on
//! speech synthesized with macOS `say`. `just e2e-smoke` starts the service and sets
//! `VOICEPUNCHER_COMMAND_SPOTTING_URL`; without it the recipe leaves the service tests (`needs_kws`) out.

use std::net::TcpStream;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use voicepuncher::adapters::secondary::commandspotting::websocket::client::probe;
use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand, VoiceCommandRecognizer};
use voicepuncher_e2e_test::voice_commands::{spoken_with_pauses, spotting_events};
use voicepuncher_test_steps::required_env;

mod probe;
mod sessions;
use sessions::*;

mod needs_kws {
    use super::*;

    #[test]
    fn should_answer_a_probe() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        probe(&url).unwrap();
    }

    #[test]
    fn should_report_a_spoken_phrase_with_its_speech() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
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
                _ => "another event",
            })
            .collect();
        // The keyword spotter and the speech recognition of the utterance may both report it,
        // each before or shortly after the end of the speech: a quiet pause may close the
        // utterance before the spotter has decoded its last word.
        assert_eq!(kinds.first(), Some(&"speech started"), "{kinds:?}");
        assert_eq!(
            kinds.iter().filter(|k| **k == "speech ended").count(),
            1,
            "{kinds:?}"
        );
        assert!(kinds.contains(&"press enter"), "{kinds:?}");
        assert!(
            kinds
                .iter()
                .all(|k| ["speech started", "press enter", "speech ended"].contains(k)),
            "{kinds:?}"
        );
    }

    #[test]
    fn should_be_refused_a_phrase_the_service_does_not_know() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // Given
        let mut socket = raw_socket(&url);
        // When
        for message in [HELLO, r#"{"type":"select","phrases":["open the door"]}"#] {
            socket.send(Message::text(message)).unwrap();
        }
        // Then
        let deadline = Instant::now() + REFUSAL_LIMIT;
        let refusal = loop {
            assert!(Instant::now() < deadline, "{NOT_REFUSED}");
            match socket.read() {
                Ok(Message::Close(frame)) => {
                    break frame.map(|f| f.reason.to_string()).unwrap_or_default()
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    panic!("{NOT_REFUSED}")
                }
                Err(e) => panic!("the connection failed before the refusal: {e}"),
            }
        };
        assert!(refusal.contains("open the door"), "{refusal}");
    }

    #[test]
    fn should_recognize_each_command_streamed_from_a_48_khz_microphone() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // When
        let recognized = each_at_once(&VoiceCommand::ALL, |command| {
            recognized_from_microphone(&url, command.phrase(), listened_for(command))
        });
        // Then
        for (command, recognized) in VoiceCommand::ALL.into_iter().zip(recognized) {
            assert_eq!(recognized, [command], "spoken: {:?}", command.phrase());
        }
    }

    #[test]
    fn should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
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
    fn should_take_a_command_phrase_that_ends_the_utterance() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        let cases = [
            ("now start dictation", VoiceCommand::StartDictation),
            ("I think we should press enter", VoiceCommand::PressEnter),
            ("and then clear all text", VoiceCommand::ClearAllText),
        ];
        // Given listening, when a sentence ends with a command
        let heard = each_at_once(&cases, |(sentence, _)| heard_after_a_pause(&url, sentence));
        // Then
        for ((sentence, command), heard) in cases.into_iter().zip(heard) {
            assert_eq!(heard, Some(command), "said: {sentence:?}");
        }
    }

    #[test]
    fn should_recognize_stop_dictation_and_press_enter_said_with_a_short_pause_in_real_time() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // Given a dictation runs
        let mut listening = Listening::start(&url, &VoiceCommand::DICTATING);
        // When the user says both commands with a short pause, as the app switches phrase sets
        let speech = std::thread::scope(|scope| {
            let audio = listening.audio.clone();
            let speaker = scope.spawn(move || {
                say_in_real_time(
                    &audio,
                    &at_microphone_rate(&spoken_with_pauses("stop dictation").unwrap()),
                );
                say_in_real_time(&audio, &microphone_silence(SHORT_PAUSE_SECS));
                say_in_real_time(
                    &audio,
                    &at_microphone_rate(&spoken_with_pauses("press enter").unwrap()),
                );
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
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // When three apps send a command at the same time
        let recognized: Vec<Vec<VoiceCommand>> = std::thread::scope(|scope| {
            // every listener starts before the first is joined, so they listen at once
            let handles: Vec<_> = [
                VoiceCommand::PressEnter,
                VoiceCommand::ClearAllText,
                VoiceCommand::StartDictation,
            ]
            .into_iter()
            .map(|command| (command.phrase(), url.clone()))
            .map(|(phrase, url)| {
                scope
                    .spawn(move || recognized_from_microphone(&url, phrase, &VoiceCommand::STANDBY))
            })
            .collect();
            let mut recognized = Vec::with_capacity(handles.len());
            for handle in handles {
                recognized.push(handle.join().unwrap());
            }
            recognized
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

    #[test]
    fn should_keep_a_live_service_that_hears_nothing_for_a_while() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // Given
        let voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::OpenStream { stream, audio: _audio, inputs: inputs_rx } =
            voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::open_stream(&url, &VoiceCommand::STANDBY).unwrap();
        // When nothing is captured for longer than the silence limit
        let input = inputs_rx.recv_timeout(QUIET_SESSION);
        // Then
        assert!(input.is_err(), "unexpected input {input:?}");
        stream.stop();
    }

    #[test]
    fn should_start_a_dictation_with_start_opening_the_utterance() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        let sentences = [
            "start dictation again",
            "start dictation for the next message",
        ];
        // Given listening, when a sentence opens with "start dictation"
        let heard = each_at_once(&sentences, |sentence| heard_after_a_pause(&url, sentence));
        // Then
        for (sentence, heard) in sentences.into_iter().zip(heard) {
            assert_eq!(
                heard,
                Some(VoiceCommand::StartDictation),
                "said: {sentence:?}"
            );
        }
    }

    #[test]
    fn should_ignore_a_command_phrase_with_more_speech_after_it() {
        let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
        // A single short word right after a command ("press enter now") is within the margin
        // quiet speech needs and still passes: a known limit of the pause rule.
        let sentences = [
            "press enter when you are ready",
            "clear all text in the morning please",
        ];
        // Given listening, when a command phrase is followed by more speech
        let heard = each_at_once(&sentences, |sentence| heard_after_a_pause(&url, sentence));
        // Then
        for (sentence, heard) in sentences.into_iter().zip(heard) {
            assert_eq!(heard, None, "said: {sentence:?}");
        }
    }
}
