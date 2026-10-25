//! Steps of a dictation started by voice: its timing, the speech after it and a failed confirmation.

use super::{
    chain, doubled, doubles, one_command, pipeline_log, run, Event, VoiceRun, COMMAND_LENGTH,
    OTHER_SENTENCE, PHRASE_LEAD, REACTION, SEGMENT_TAIL, SENTENCE, SETTLE,
};

mod failed_confirmation;
mod speech_after_start;
mod start_timing;
