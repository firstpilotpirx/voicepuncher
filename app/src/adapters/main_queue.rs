//! Work handed to the main queue, where `AppKit` draws: the overlay and the status item; and
//! values that live on the main thread, reached from any thread through a handle.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use objc2::MainThreadMarker;

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
    // SAFETY: `dispatch_time` only reads its arguments.
    let when = unsafe { dispatch_time(DISPATCH_TIME_NOW, delay_ms * NANOS_PER_MILLI) };
    // SAFETY: as in `dispatch_to_main`.
    unsafe { dispatch_after_f(when, main_queue(), into_context(work), trampoline) };
}

/// Runs `work` on the main queue, with the marker of the main thread it runs on.
pub(crate) fn on_main<F: FnOnce(MainThreadMarker) + Send + 'static>(work: F) {
    dispatch_to_main(move || work(main_queue_marker()));
}

/// Runs `work` on the main queue `delay_ms` from now, with the marker of the main thread.
pub(crate) fn on_main_after<F: FnOnce(MainThreadMarker) + Send + 'static>(delay_ms: i64, work: F) {
    dispatch_to_main_after(delay_ms, move || work(main_queue_marker()));
}

fn main_queue_marker() -> MainThreadMarker {
    // SAFETY: only main-queue work calls this, and the main queue runs on the main thread.
    unsafe { MainThreadMarker::new_unchecked() }
}

unsafe extern "C" fn trampoline(context: *mut c_void) {
    // SAFETY: `context` came from `into_context` and is run exactly once.
    let work: Box<MainQueueWork> = unsafe { Box::from_raw(context.cast()) };
    (*work)();
}

thread_local! {
    /// The values of every `MainOnly`, by key; only the main thread's map is ever filled.
    static MAIN_ONLY: RefCell<HashMap<u64, Rc<dyn Any>>> = RefCell::new(HashMap::new());
}

static NEXT_KEY: AtomicU64 = AtomicU64::new(0);

fn store<T: 'static>(value: T) -> u64 {
    let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
    let value: Rc<dyn Any> = Rc::new(value);
    MAIN_ONLY.with(|values| values.borrow_mut().insert(key, value));
    key
}

fn lend<T: 'static>(key: u64) -> Option<Rc<T>> {
    let value = MAIN_ONLY.with(|values| values.borrow().get(&key).cloned())?;
    value.downcast::<T>().ok()
}

fn remove(key: u64) {
    let removed = MAIN_ONLY.try_with(|values| values.borrow_mut().remove(&key));
    // WHY: dropped outside the borrow, as the value's own drop may store or remove values
    drop(removed);
}

/// A value that lives on the main thread, reached from any thread through this handle: only
/// code holding a `MainThreadMarker` borrows it, so its `AppKit` objects never leave the main
/// thread. Clones share the value; the last one dropped removes it on the main thread.
pub(crate) struct MainOnly<T: 'static> {
    key: Arc<MainKey>,
    value: PhantomData<fn() -> T>,
}

impl<T: 'static> MainOnly<T> {
    pub(crate) fn new(_mtm: MainThreadMarker, value: T) -> Self {
        Self { key: Arc::new(MainKey(store(value))), value: PhantomData }
    }

    /// The value; `None` once the main thread's values are gone, as at the end of the process.
    pub(crate) fn get(&self, _mtm: MainThreadMarker) -> Option<Rc<T>> {
        lend(self.key.0)
    }

    /// The value when called on the main thread, else `None`: for probes of tests.
    pub(crate) fn on_this_thread(&self) -> Option<Rc<T>> {
        self.get(MainThreadMarker::new()?)
    }
}

impl<T: 'static> Clone for MainOnly<T> {
    fn clone(&self) -> Self {
        Self { key: Arc::clone(&self.key), value: PhantomData }
    }
}

struct MainKey(u64);

impl Drop for MainKey {
    fn drop(&mut self) {
        let key = self.0;
        if MainThreadMarker::new().is_some() {
            remove(key);
        } else {
            dispatch_to_main(move || remove(key));
        }
    }
}

/// Fails to compile when `$t` is `Send`: with it, both impls of the probe trait apply and the
/// call is ambiguous. `const _: () = assert_not_send!(T);` keeps `T` on its thread for good.
#[doc(hidden)]
#[macro_export]
macro_rules! assert_not_send {
    ($t:ty) => {{
        trait AmbiguousIfSend<A> {
            fn some_item() {}
        }
        impl<T: ?Sized> AmbiguousIfSend<()> for T {}
        impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}
        let _ = <$t as AmbiguousIfSend<_>>::some_item;
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_and_sync<T: Send + Sync>() {}

    #[test]
    fn should_lend_a_stored_value_until_it_is_removed() {
        let key = store(String::from("overlay"));
        assert_eq!(
            lend::<String>(key).as_deref().map(String::as_str),
            Some("overlay")
        );
        remove(key);
        assert!(
            lend::<String>(key).is_none(),
            "a removed value is still lent"
        );
    }

    #[test]
    fn should_lend_nothing_for_a_value_of_another_type() {
        let key = store(7_u32);
        assert!(lend::<String>(key).is_none());
        remove(key);
    }

    #[test]
    fn should_cross_threads_as_a_handle_while_its_value_cannot() {
        is_send_and_sync::<MainOnly<std::rc::Rc<()>>>();
        crate::assert_not_send!(std::rc::Rc<()>);
    }
}
