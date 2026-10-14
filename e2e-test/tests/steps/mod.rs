pub mod file_transcription;
pub mod refinement_service_stub;
pub mod speech_recognition;
mod text_refinement;
pub mod voice_control;

use std::panic::{self, AssertUnwindSafe};

use futures::channel::oneshot;

use crate::E2eWorld;

/// Runs a blocking `step` on a thread of its own with the scenario's world, so the scenarios
/// the harness runs at once overlap: cucumber polls them all on one thread, and a step that
/// blocks it holds up every other scenario. A panic in `step` fails the step as before.
pub async fn off_thread<F>(world: &mut E2eWorld, step: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut E2eWorld) -> anyhow::Result<()> + Send + 'static,
{
    let mut own = std::mem::take(world);
    let (done, finished) = oneshot::channel();
    std::thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| step(&mut own)));
        let _ = done.send((own, result));
    });
    let (own, result) = finished.await.expect("the step's thread always answers");
    *world = own;
    result.unwrap_or_else(|panicked| panic::resume_unwind(panicked))
}
