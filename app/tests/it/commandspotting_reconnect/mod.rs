//! The command spotting adapter against stub services on a loopback port that close the
//! connection, go away and come back: the adapter reports the loss once, an outage past the
//! replay once, reaches for the service until it answers, reports its return and sends it the
//! audio it missed.
//! A service that accepts but never completes the handshake fails an attempt in seconds.

use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

use voicepuncher::adapters::secondary::commandspotting::websocket::{
    AudioStream, LinkTimings, MicrophoneAudio,
};
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand};

mod service;
use service::*;
mod stream;
use stream::*;
mod outages;
use outages::*;
mod copies;
