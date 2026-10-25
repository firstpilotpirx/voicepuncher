//! Integration tests for `OpenAiTranscriber` against an in-test HTTP stub.
//! Two tests are gated on `VOICEPUNCHER_TRANSCRIPTION_URL` (+ `VOICEPUNCHER_BENCH_CORPUS`)
//! and transcribe a corpus WAV, clean and under high noise, through a real speech
//! recognition service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_copy_pool::ServiceAddress;
use voicepuncher_test_steps::loopback::serve_each;
use voicepuncher_test_steps::required_env;

mod stubs;
use stubs::*;
mod answers;
mod retries;
use retries::*;
mod languages;
mod real_service;
use languages::*;
mod copies;
use copies::*;
mod copy_recovery;
mod foreign_program;
use copy_recovery::*;
