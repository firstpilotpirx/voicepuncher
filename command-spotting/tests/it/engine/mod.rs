//! The real keyword spotting model and voice activity detector on speech synthesized with
//! macOS `say`. Needs the models the justfile finds (`needs_kws_models`).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use crate::common;
use crate::kws_models::kws_models;
use voicepuncher_test_steps::required_env;

use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::engine::{Engine, EngineCfg, Session, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::ServerEvent;

mod speech;
pub(crate) use speech::*;
mod hearings;
pub(crate) use hearings::reported_phrases;
use hearings::*;
mod turns;
use turns::*;

const PAUSE_SECS: f32 = 1.0;
/// As long as a user keeps listening on before saying a command.
const LONG_SILENCE_SECS: f32 = 25.0;
/// How late the reported phrase may start after its speech starts, and how far its end may be
/// from the end of its speech: the app's pause rule margins. An early start is allowed: in
/// noise the spotter may stretch a phrase back.
const LEAD_TOLERANCE_SECS: f32 = 0.3;
const TAIL_TOLERANCE_SECS: f32 = 0.5;
/// Silence streamed before the phrase set changes.
const LEAD_IN_SECS: f32 = 2.0;
const FRAME_SAMPLES: usize = 1_600;
const MIN_SILENCE_SECS: f32 = 0.3;
mod needs_kws_models;
