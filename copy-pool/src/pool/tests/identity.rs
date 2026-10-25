//! A port joins only once the program on it says it is the service: another program that
//! accepts connections on a port of the service never gets a request.

use std::collections::HashMap;

use super::*;
use crate::probe::{ForeignProgram, Identity};

/// A probe whose ports accept connections as `up` says and identify as `identities` says
/// (the service when unlisted), counting the identity questions per port.
#[derive(Clone, Default)]
struct IdentityProbe {
    up: Arc<Mutex<HashSet<u16>>>,
    identities: Arc<Mutex<HashMap<u16, Identity>>>,
    asked: Arc<Mutex<Vec<u16>>>,
}

impl IdentityProbe {
    fn with(up: &[u16], identities: &[(u16, Identity)]) -> Self {
        let probe = Self::default();
        probe.up.lock().unwrap().extend(up);
        probe
            .identities
            .lock()
            .unwrap()
            .extend(identities.iter().cloned());
        probe
    }

    fn set_up(&self, port: u16, up: bool) {
        let mut ports = self.up.lock().unwrap();
        if up {
            ports.insert(port);
        } else {
            ports.remove(&port);
        }
    }

    fn identify_as(&self, port: u16, identity: Identity) {
        self.identities.lock().unwrap().insert(port, identity);
    }

    fn asked(&self, port: u16) -> usize {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .filter(|asked| **asked == port)
            .count()
    }
}

impl Probe for IdentityProbe {
    fn answers(&self, port: u16) -> bool {
        self.up.lock().unwrap().contains(&port)
    }

    fn identify(&self, port: u16) -> Identity {
        self.asked.lock().unwrap().push(port);
        self.identities
            .lock()
            .unwrap()
            .get(&port)
            .cloned()
            .unwrap_or(Identity::Service)
    }
}

fn go_program(port: u16) -> Identity {
    Identity::Foreign(ForeignProgram { port, name: Some("app".into()), pid: Some(28_344) })
}

fn pool_identified_by(probe: &IdentityProbe) -> CopyPool {
    let address = ServiceAddress::parse(SERVICE).unwrap();
    CopyPool::discover(address, probe.clone(), 0, NEVER_ON_ITS_OWN)
}

#[test]
fn should_take_the_copies_when_another_program_holds_the_service_port() {
    let probe = IdentityProbe::with(&[8081, 18082, 18083], &[(8081, go_program(8081))]);
    let pool = pool_identified_by(&probe);
    assert_eq!(ports(&pool), [18082, 18083]);
}

#[test]
fn should_pass_over_another_program_on_a_copy_port() {
    let probe = IdentityProbe::with(&[18082, 18083, 18084], &[(18084, go_program(18084))]);
    let pool = pool_identified_by(&probe);
    assert_eq!(ports(&pool), [18082, 18083]);
}

#[test]
fn should_ask_a_port_once_while_it_keeps_accepting_connections() {
    let probe = IdentityProbe::with(&[18082], &[]);
    let pool = pool_identified_by(&probe);
    assert_eq!(ports(&pool), [18082]);
    pool.inner.probe_round();
    pool.inner.probe_round();
    assert_eq!(probe.asked(18082), 1);
}

#[test]
fn should_ask_an_unanswered_port_again_and_take_it_once_it_says_it_is_the_service() {
    let probe = IdentityProbe::with(&[18082], &[(18082, Identity::Unknown)]);
    let pool = pool_identified_by(&probe);
    assert!(!pool.has_copies());
    probe.identify_as(18082, Identity::Service);
    pool.inner.probe_round();
    assert_eq!(ports(&pool), [18082]);
}

#[test]
fn should_ask_a_port_again_after_it_stopped_accepting_connections() {
    let probe = IdentityProbe::with(&[8081, 18082], &[(8081, go_program(8081))]);
    let pool = pool_identified_by(&probe);
    assert_eq!(ports(&pool), [18082]);
    probe.set_up(8081, false);
    pool.inner.probe_round();
    probe.identify_as(8081, Identity::Service);
    probe.set_up(8081, true);
    pool.inner.probe_round();
    assert_eq!(probe.asked(8081), 2);
    assert_eq!(ports(&pool), [8081]);
}

#[test]
fn should_name_the_program_on_the_service_port_when_no_copy_runs() {
    let probe = IdentityProbe::with(&[8081], &[(8081, go_program(8081))]);
    let pool = pool_identified_by(&probe);
    let Err(PoolError::NoCopies { foreign, .. }) = pool.lease(SHORT_WAIT) else {
        panic!("a lease without copies");
    };
    assert_eq!(
        foreign,
        Some(ForeignProgram { port: 8081, name: Some("app".into()), pid: Some(28_344) })
    );
}
