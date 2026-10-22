//! Probes under load: a copy busy decoding answers a probe late, and one late answer must
//! not drop it.

use super::*;

/// The two copies found: on the two ports above the service port.
const UP: [u16; 2] = [18082, 18083];

/// A probe that both copies answer, except the `misses` next ones of `missing`, counting the
/// probes of each port.
struct MissingProbe {
    missing: u16,
    misses: Mutex<usize>,
    probes: Mutex<Vec<u16>>,
}

impl MissingProbe {
    fn missing(port: u16, misses: usize) -> Arc<Self> {
        Arc::new(Self { missing: port, misses: Mutex::new(misses), probes: Mutex::new(Vec::new()) })
    }

    fn probes_of(&self, port: u16) -> usize {
        self.probes
            .lock()
            .unwrap()
            .iter()
            .filter(|probed| **probed == port)
            .count()
    }
}

impl Probe for Arc<MissingProbe> {
    fn answers(&self, port: u16) -> bool {
        self.probes.lock().unwrap().push(port);
        if port != self.missing {
            return UP.contains(&port);
        }
        let mut misses = self.misses.lock().unwrap();
        if *misses == 0 {
            return true;
        }
        *misses -= 1;
        false
    }
}

fn pool_probed_by(probe: &Arc<MissingProbe>) -> CopyPool {
    let address = ServiceAddress::parse(SERVICE).unwrap();
    CopyPool::discover(address, Arc::clone(probe), 0, NEVER_ON_ITS_OWN)
}

#[test]
fn should_keep_a_copy_missed_by_one_slow_probe_and_probe_it_once_more() {
    // Given two copies found, one of which answers its next probe late
    let probe = MissingProbe::missing(18083, 0);
    let pool = pool_probed_by(&probe);
    assert_eq!(ports(&pool), UP); // the first probe round is over
    *probe.misses.lock().unwrap() = 1;
    // When a probe round misses it once
    pool.inner.probe_round();
    // Then it stays, probed a second time in the same round
    assert_eq!(ports(&pool), UP);
    assert_eq!(probe.probes_of(18083), 3);
    assert_eq!(probe.probes_of(18082), 2);
}

#[test]
fn should_drop_a_copy_missed_by_two_probes_in_a_row() {
    // Given two copies found, one of which stops answering
    let probe = MissingProbe::missing(18083, 0);
    let pool = pool_probed_by(&probe);
    assert_eq!(ports(&pool), UP);
    *probe.misses.lock().unwrap() = 2;
    // When a probe round misses it twice
    pool.inner.probe_round();
    // Then it leaves
    assert_eq!(ports(&pool), [18082]);
}
