//! Locks that outlive a panic: one failed holder must not take every later user down.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// `mutex`'s value, also after a panic in another holder.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
