//! Bounded-concurrency map for the E2E harness.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Maps `items` through `f` on at most `max_concurrent` threads at a time and
/// returns the results in input order.
#[expect(clippy::excessive_nesting, reason = "size baseline")]
pub fn map_concurrently<T, R, F>(items: &[T], max_concurrent: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let next = AtomicUsize::new(0);
    let mut indexed: Vec<(usize, R)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..max_concurrent.max(1))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::SeqCst);
                        let Some(item) = items.get(i) else {
                            return done;
                        };
                        done.push((i, f(item)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("a worker panicked"))
            .collect()
    });
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn should_return_results_in_input_order() {
        let items: Vec<u64> = (0..10).collect();
        let results = map_concurrently(&items, 2, |&i| {
            std::thread::sleep(Duration::from_millis(10 * (10 - i)));
            i * 10
        });
        assert_eq!(results, (0..10).map(|i| i * 10).collect::<Vec<_>>());
    }

    #[test]
    fn should_run_at_most_n_items_at_a_time() {
        let running = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let items: Vec<u32> = (0..12).collect();
        map_concurrently(&items, 2, |_| {
            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(20));
            running.fetch_sub(1, Ordering::SeqCst);
        });
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }
}
