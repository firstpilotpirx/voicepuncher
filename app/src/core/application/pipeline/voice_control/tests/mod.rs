use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::core::application::audio_recorder::LiveChunk;
use crate::core::application::live_transcript_view::HiddenLiveTranscript;
use crate::core::application::pipeline::{PipelineConfig, PipelineParts};
use crate::core::application::prelude::*;
use crate::core::application::{ChunkingCfg, PipelineInput};
use crate::core::domain::{
    AudioBuffer16k, PttSignal, RawTranscript, RecordingMode, SpottingEvent, VoiceCommand,
};

mod stand_ins;
use stand_ins::*;
mod listening;
mod outages;
mod starts;
