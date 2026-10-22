use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::*;

const SERVICE: &str = "http://127.0.0.1:8081/v1";
const INTERVAL: Duration = Duration::from_millis(10);
const SHORT_WAIT: Duration = Duration::from_millis(30);
const LONG_WAIT: Duration = Duration::from_secs(2);
const REJOIN_WITHIN: Duration = Duration::from_secs(1);
/// A probe interval no test outlasts: the test probes by hand.
const NEVER_ON_ITS_OWN: Duration = Duration::from_secs(3600);

#[derive(Clone, Default)]
struct FakeProbe(Arc<Mutex<HashSet<u16>>>);

impl FakeProbe {
    fn up(&self, port: u16) {
        self.0.lock().unwrap().insert(port);
    }
}

impl Probe for FakeProbe {
    fn answers(&self, port: u16) -> bool {
        self.0.lock().unwrap().contains(&port)
    }
}

fn pool_with(up: &[u16], offset: usize) -> (CopyPool, FakeProbe) {
    pool_probed_every(up, offset, INTERVAL)
}

fn pool_probed_every(up: &[u16], offset: usize, interval: Duration) -> (CopyPool, FakeProbe) {
    let probe = FakeProbe::default();
    up.iter().for_each(|port| probe.up(*port));
    let address = ServiceAddress::parse(SERVICE).unwrap();
    (
        CopyPool::discover(address, probe.clone(), offset, interval),
        probe,
    )
}

fn ports(pool: &CopyPool) -> Vec<u16> {
    pool.snapshot().iter().map(|copy| copy.port).collect()
}

fn hung(pool: &CopyPool, port: u16) -> bool {
    pool.snapshot()
        .iter()
        .any(|copy| copy.port == port && copy.hung)
}

fn wait_until(within: Duration, condition: impl Fn() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < within {
        if condition() {
            return true;
        }
        thread::sleep(INTERVAL);
    }
    condition()
}

#[test]
fn should_find_the_copies_on_the_ports_above_the_service_port() {
    let (pool, _) = pool_with(&[18082, 18083, 18084], 0);
    assert_eq!(ports(&pool), [18082, 18083, 18084]);
}

#[test]
fn should_take_the_service_port_alone_when_it_answers() {
    let (pool, _) = pool_with(&[8081, 18082], 0);
    assert_eq!(ports(&pool), [8081]);
}

#[test]
fn should_find_no_copy_when_nothing_answers() {
    let (pool, _) = pool_with(&[], 0);
    assert!(!pool.has_copies());
    assert!(matches!(
        pool.lease(SHORT_WAIT),
        Err(PoolError::NoCopies { .. })
    ));
}

#[test]
fn should_drop_a_copy_at_its_first_refused_connection_and_take_it_back() {
    let (pool, _) = pool_probed_every(&[18082, 18083], 0, NEVER_ON_ITS_OWN);
    let lease = pool.lease(SHORT_WAIT).unwrap();
    assert_eq!(lease.port(), 18082);
    lease.refused();
    assert_eq!(ports(&pool), [18083]);
    assert_eq!(pool.lease(SHORT_WAIT).unwrap().port(), 18083);
    pool.inner.probe_round();
    assert_eq!(ports(&pool), [18082, 18083]);
}

#[test]
fn should_add_a_copy_that_starts_later() {
    let (pool, probe) = pool_with(&[18082], 0);
    probe.up(18083);
    assert!(wait_until(REJOIN_WITHIN, || ports(&pool) == [18082, 18083]));
}

#[test]
fn should_wait_while_every_copy_is_busy_and_give_up_holding_none() {
    let (pool, _) = pool_with(&[18082], 0);
    let held = pool.lease(SHORT_WAIT).unwrap();
    assert!(matches!(
        pool.lease(SHORT_WAIT),
        Err(PoolError::NoFreeCopy { .. })
    ));
    held.finished();
    assert_eq!(pool.lease(SHORT_WAIT).unwrap().port(), 18082);
}

#[test]
fn should_never_lease_one_copy_twice() {
    let (pool, _) = pool_with(&[18082, 18083], 0);
    let first = pool.lease(SHORT_WAIT).unwrap();
    let second = pool.lease(SHORT_WAIT).unwrap();
    assert_ne!(first.port(), second.port());
    assert!(pool.lease(SHORT_WAIT).is_err());
}

#[test]
fn should_serve_waiting_requests_first_come_first_served() {
    let (pool, _) = pool_with(&[18082], 0);
    let held = pool.lease(SHORT_WAIT).unwrap();
    let order = Arc::new(Mutex::new(Vec::new()));
    let waiter = |name: &'static str| {
        let (pool, order) = (pool.clone(), order.clone());
        thread::spawn(move || {
            let lease = pool.lease(LONG_WAIT).unwrap();
            order.lock().unwrap().push(name);
            thread::sleep(SHORT_WAIT);
            lease.finished();
        })
    };
    let first = waiter("first");
    thread::sleep(SHORT_WAIT);
    let second = waiter("second");
    thread::sleep(SHORT_WAIT);
    held.finished();
    first.join().unwrap();
    second.join().unwrap();
    assert_eq!(*order.lock().unwrap(), ["first", "second"]);
}

#[test]
fn should_keep_the_place_of_a_request_whose_wait_ran_out() {
    let (pool, _) = pool_with(&[18082], 0);
    let held = pool.lease(SHORT_WAIT).unwrap();
    let mut first = pool.place();
    assert!(matches!(
        first.lease_avoiding(SHORT_WAIT, &[]),
        Err(PoolError::NoFreeCopy { .. })
    ));
    held.finished();
    assert!(
        matches!(pool.lease(SHORT_WAIT), Err(PoolError::NoFreeCopy { .. })),
        "a later request took the copy the first one waits for"
    );
    assert_eq!(first.lease_avoiding(SHORT_WAIT, &[]).unwrap().port(), 18082);
}

#[test]
fn should_give_up_the_place_of_a_dropped_request() {
    let (pool, _) = pool_with(&[18082], 0);
    let held = pool.lease(SHORT_WAIT).unwrap();
    let mut dropped = pool.place();
    assert!(dropped.lease_avoiding(SHORT_WAIT, &[]).is_err());
    drop(dropped);
    held.finished();
    assert_eq!(pool.lease(SHORT_WAIT).unwrap().port(), 18082);
}

#[test]
fn should_give_the_answer_back_when_it_comes_in_time() {
    let (pool, _) = pool_with(&[18082], 0);
    let lease = pool.lease(SHORT_WAIT).unwrap();
    let ran = lease.run(LONG_WAIT, |url| (url.to_owned(), Finish::Answered));
    assert!(matches!(ran, Ran::Answered(url) if url == "http://127.0.0.1:18082/v1"));
}

#[test]
fn should_hold_the_copy_of_an_abandoned_request_until_it_is_answered() {
    let (pool, _) = pool_with(&[18082], 0);
    let lease = pool.lease(SHORT_WAIT).unwrap();
    let ran = lease.run(SHORT_WAIT, |_| {
        thread::sleep(Duration::from_millis(300));
        ((), Finish::Answered)
    });
    assert!(matches!(ran, Ran::GaveUp));
    assert!(matches!(
        pool.lease(SHORT_WAIT),
        Err(PoolError::NoFreeCopy { .. })
    ));
    assert_eq!(pool.lease(LONG_WAIT).unwrap().port(), 18082);
}

#[test]
fn should_report_a_copy_hung_past_twice_its_wait_until_it_answers() {
    let (pool, _) = pool_with(&[18082, 18083], 0);
    let lease = pool.lease(SHORT_WAIT).unwrap();
    let ran = lease.run(Duration::from_millis(10), |_| {
        thread::sleep(Duration::from_millis(300));
        ((), Finish::Answered)
    });
    assert!(matches!(ran, Ran::GaveUp));
    assert!(wait_until(LONG_WAIT, || hung(&pool, 18082)));
    assert_eq!(pool.lease(SHORT_WAIT).unwrap().port(), 18083);
    assert!(wait_until(LONG_WAIT, || !hung(&pool, 18082)));
}

#[test]
fn should_start_with_the_copy_of_its_offset() {
    let (first, _) = pool_with(&[18082, 18083, 18084], 0);
    let (second, _) = pool_with(&[18082, 18083, 18084], 1);
    assert_eq!(first.lease(SHORT_WAIT).unwrap().port(), 18082);
    assert_eq!(second.lease(SHORT_WAIT).unwrap().port(), 18083);
}

#[test]
fn should_share_one_pool_per_service_address() {
    let one = CopyPool::shared("http://127.0.0.1:1/v1").unwrap();
    let again = CopyPool::shared("http://127.0.0.1:1/v1").unwrap();
    let other = CopyPool::shared("http://127.0.0.1:2/v1").unwrap();
    assert!(Arc::ptr_eq(&one.inner, &again.inner));
    assert!(!Arc::ptr_eq(&one.inner, &other.inner));
}

/// Takes `delay` to tell whether a port answers; every port answers.
struct SlowProbe {
    delay: Duration,
}

impl Probe for SlowProbe {
    fn answers(&self, _: u16) -> bool {
        thread::sleep(self.delay);
        true
    }
}

#[test]
fn should_return_before_a_slow_first_probe_ends_and_lease_once_it_did() {
    // Given
    let address = ServiceAddress::parse(SERVICE).unwrap();
    let probe = SlowProbe { delay: Duration::from_millis(300) };
    // When
    let started = Instant::now();
    let pool = CopyPool::discover(address, probe, 0, NEVER_ON_ITS_OWN);
    let created_in = started.elapsed();
    let lease = pool.lease(LONG_WAIT);
    // Then
    assert!(created_in < Duration::from_millis(100), "{created_in:?}");
    assert_eq!(lease.unwrap().port(), 8081);
}

#[test]
fn should_lease_a_copy_not_tried_yet_before_a_tried_one() {
    let (pool, _) = pool_with(&[18082, 18083], 0);
    let lease = pool.lease_avoiding(SHORT_WAIT, &[18082]).unwrap();
    assert_eq!(lease.port(), 18083);
    lease.finished();
    let again = pool.lease_avoiding(SHORT_WAIT, &[18082, 18083]).unwrap();
    assert_eq!(again.port(), 18082);
}

#[test]
fn should_refuse_a_lease_at_once_when_no_copy_takes_requests() {
    let (pool, _) = pool_with(&[], 0);
    let started = Instant::now();
    assert!(matches!(
        pool.lease(LONG_WAIT),
        Err(PoolError::NoCopies { .. })
    ));
    assert!(
        started.elapsed() < LONG_WAIT / 2,
        "waited {:?}",
        started.elapsed()
    );
}

#[test]
fn should_wait_for_a_copy_to_come_back_when_none_takes_requests() {
    // Given
    let (pool, probe) = pool_with(&[], 0);
    let comes_back = thread::spawn(move || {
        thread::sleep(SHORT_WAIT);
        probe.up(18083);
    });
    // When
    let started = Instant::now();
    let back = pool.await_copy(LONG_WAIT);
    // Then
    comes_back.join().unwrap();
    assert!(back);
    assert!(started.elapsed() < LONG_WAIT);
    assert_eq!(ports(&pool), [18083]);
}

#[test]
fn should_give_up_waiting_for_a_copy_that_never_comes_back() {
    // Given
    let (pool, _) = pool_with(&[], 0);
    // When
    let started = Instant::now();
    let back = pool.await_copy(SHORT_WAIT);
    // Then
    assert!(!back);
    assert!(started.elapsed() >= SHORT_WAIT);
}

#[test]
fn should_not_wait_while_a_copy_takes_requests() {
    let (pool, _) = pool_with(&[18082], 0);
    let started = Instant::now();
    assert!(pool.await_copy(LONG_WAIT));
    assert!(started.elapsed() < SHORT_WAIT);
}
