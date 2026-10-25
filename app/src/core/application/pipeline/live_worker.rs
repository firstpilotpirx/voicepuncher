//! The thread that transcribes a dictation's chunks while it records.

use std::sync::mpsc;
use std::time::Instant;

use crate::core::application::audio_recorder::LiveChunk;

use crate::core::domain::FailingPart;

use super::failure_notices::recognition_part;
use super::live_transcript::LiveTranscript;
use super::{lock_transcriber, LiveSessionResult, SharedTranscriber};

/// Transcribes every chunk of `chunk_rx` as it comes, shows the text so far in
/// `live_transcript`, and sends the texts and the count of lost chunks once the channel ends.
pub(super) fn run_live_worker(
    transcriber: &SharedTranscriber,
    chunk_rx: &mpsc::Receiver<LiveChunk>,
    results_tx: &mpsc::SyncSender<LiveSessionResult>,
    live_transcript: &LiveTranscript,
) {
    let mut texts = Vec::new();
    let mut lost_chunks = 0usize;
    let mut lost_to = None;
    let mut chunk_num = 0usize;
    while let Ok(chunk) = chunk_rx.recv() {
        chunk_num += 1;
        match transcribe_chunk(transcriber, &chunk, chunk_num) {
            Ok(Some(text)) => {
                texts.push(text);
                live_transcript.show(&texts);
            }
            Ok(None) => {}
            Err(ChunkLost(part)) => {
                lost_chunks += 1;
                lost_to = Some(part);
            }
        }
    }
    log::info!("live worker: {chunk_num} chunks done");
    if results_tx
        .send(LiveSessionResult { texts, lost_chunks, lost_to })
        .is_err()
    {
        log::debug!("live worker: the result of {chunk_num} chunks was received by nobody");
    }
}

/// Speech recognition failed on a chunk; the rest of the dictation is kept.
struct ChunkLost(FailingPart);

/// The text of `chunk`, none when it held no speech.
fn transcribe_chunk(
    transcriber: &SharedTranscriber,
    chunk: &LiveChunk,
    chunk_num: usize,
) -> Result<Option<String>, ChunkLost> {
    let t_queued_ms = Instant::now()
        .saturating_duration_since(chunk.enqueued_at)
        .as_millis();
    let duration_secs = chunk.audio.duration_ms() as f32 / 1000.0;
    let t0 = Instant::now();
    let answer = lock_transcriber(transcriber).transcribe(&chunk.audio);
    match answer {
        Ok(Some(raw)) => {
            log::info!(
                "chunk {chunk_num} | {duration_secs:.1}s audio | queued {t_queued_ms}ms | transcribed in {:.2}s",
                t0.elapsed().as_secs_f32(),
            );
            Ok(Some(raw.as_str().to_owned()))
        }
        Ok(None) => {
            log::info!(
                "chunk {chunk_num} | {duration_secs:.1}s audio | queued {t_queued_ms}ms | hallucination dropped in {:.2}s",
                t0.elapsed().as_secs_f32(),
            );
            Ok(None)
        }
        Err(e) => {
            log::error!(
                "chunk {chunk_num} lost | {duration_secs:.1}s audio not transcribed, the rest of the dictation is kept: {e:#}"
            );
            Err(ChunkLost(recognition_part(&e)))
        }
    }
}
