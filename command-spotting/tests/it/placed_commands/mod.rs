//! Commands said without a pause around them, heard by the real keyword spotter, voice
//! activity detector and speech recognition service for commands, on speech synthesized
//! with macOS `say`. Needs the models (`needs_kws_models`), and the tests on the real speech
//! recognition service for commands need it too (`needs_command_stt`), so the engine confirms
//! as the command spotting service does.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::common;
use crate::scripted_hearing;
use crate::stream_clock;

mod synced;
mod under_load;

use scripted_hearing::{ScriptedHearing, ScriptedWord};
use stream_clock::{samples, Clocked, StreamClock};
use voicepuncher_command_spotting::confirm::{Hear, Heard, TimedWord, UtteranceTranscriber};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::{Placement, ServerEvent};
use voicepuncher_test_steps::required_env;

mod clocked;
use clocked::*;
mod feeding;
use feeding::*;
mod hearings;
use hearings::*;

const STANDBY: [&str; 3] = ["start dictation", "press enter", "clear all text"];
mod needs_kws_models;
