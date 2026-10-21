//! Work handed to the main queue, where AppKit draws: the overlay and the status item.

use std::ffi::c_void;

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(
        queue: *const c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
    fn dispatch_after_f(
        when: u64,
        queue: *const c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
    fn dispatch_time(when: u64, delta: i64) -> u64;
}

const DISPATCH_TIME_NOW: u64 = 0;
const NANOS_PER_MILLI: i64 = 1_000_000;

type MainQueueWork = Box<dyn FnOnce() + Send>;

fn main_queue() -> *const c_void {
    &raw const _dispatch_main_q
}

fn into_context<F: FnOnce() + Send + 'static>(work: F) -> *mut c_void {
    let boxed: Box<MainQueueWork> = Box::new(Box::new(work));
    Box::into_raw(boxed).cast()
}

/// Runs `work` on the main queue as soon as it gets there.
pub(crate) fn dispatch_to_main<F: FnOnce() + Send + 'static>(work: F) {
    // SAFETY: the context is a leaked `Box<MainQueueWork>` that `trampoline` takes back once.
    unsafe { dispatch_async_f(main_queue(), into_context(work), trampoline) }
}

/// Runs `work` on the main queue `delay_ms` from now.
pub(crate) fn dispatch_to_main_after<F: FnOnce() + Send + 'static>(delay_ms: i64, work: F) {
    // SAFETY: as in `dispatch_to_main`; `dispatch_time` only reads its arguments.
    unsafe {
        let when = dispatch_time(DISPATCH_TIME_NOW, delay_ms * NANOS_PER_MILLI);
        dispatch_after_f(when, main_queue(), into_context(work), trampoline);
    }
}

unsafe extern "C" fn trampoline(context: *mut c_void) {
    // SAFETY: `context` came from `into_context` and is run exactly once.
    let work: Box<MainQueueWork> = unsafe { Box::from_raw(context.cast()) };
    (*work)();
}
