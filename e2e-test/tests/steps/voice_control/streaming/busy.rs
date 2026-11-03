//! Every core of the machine kept busy by spinning threads of the test process, as a build
//! or another program would keep it, while a scenario streams in real time.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use cucumber::given;

use super::run;
use crate::E2eWorld;

/// One spinning thread per available core, stopped when dropped.
#[derive(Debug)]
pub struct BusyCores {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl BusyCores {
    pub(crate) fn start() -> Self {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        let stop = Arc::new(AtomicBool::new(false));
        let threads = (0..cores)
            .map(|_| {
                let stop = Arc::clone(&stop);
                std::thread::spawn(move || spin_until(&stop))
            })
            .collect();
        Self { stop, threads }
    }
}

fn spin_until(stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        std::hint::spin_loop();
    }
}

impl Drop for BusyCores {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            // A spinning thread only loops: one that panicked has nothing left to stop
            let _ = thread.join();
        }
    }
}

#[given("every core of the machine is busy with other work")]
fn every_core_busy(world: &mut E2eWorld) {
    run(world).busy = Some(BusyCores::start());
}
