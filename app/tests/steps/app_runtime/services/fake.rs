//! The stand-in for the local services: what the runtime requires, sees and stops.

use super::{
    push, Arc, BTreeMap, Config, Event, Events, LocalServices, Mutex, PathBuf, ServiceKind,
    ServiceState, ServiceStatus, ServicesProgress, COMMANDS, COMMAND_SPOTTING, DICTATION,
    LOOKS_TO_ANSWER, SECOND_COPY,
};

#[derive(Default)]
struct Inner {
    statuses: BTreeMap<u16, ServiceStatus>,
    needed: Vec<u16>,
    /// Services required from now on answer by themselves after a few looks.
    answer_by_themselves: bool,
    looks: BTreeMap<u16, usize>,
}

impl Inner {
    /// One more look at every starting service; one looked at often enough answers.
    fn look_at_starting(&mut self) {
        let starting: Vec<u16> = self
            .statuses
            .iter()
            .filter(|(_, status)| matches!(status.state, ServiceState::Starting { .. }))
            .map(|(port, _)| *port)
            .collect();
        for port in starting {
            let looks = self.looks.entry(port).or_default();
            *looks += 1;
            if *looks >= LOOKS_TO_ANSWER {
                self.statuses.get_mut(&port).unwrap().state = ServiceState::Ready;
            }
        }
    }
}

/// Shared by the runtime, which requires and stops the services, and the steps.
#[derive(Clone)]
pub(crate) struct FakeServices {
    inner: Arc<Mutex<Inner>>,
    events: Events,
}

impl FakeServices {
    pub(crate) fn new(events: &Events, answer_by_themselves: bool) -> Self {
        let inner = Inner { answer_by_themselves, ..Inner::default() };
        Self { inner: Arc::new(Mutex::new(inner)), events: Arc::clone(events) }
    }

    pub(super) fn set(&self, port: u16, state: ServiceState) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(status) = inner.statuses.get_mut(&port) {
            status.state = state;
        }
    }

    pub(super) fn required(&self, port: u16) -> bool {
        self.inner.lock().unwrap().statuses.contains_key(&port)
    }

    pub(crate) fn any_required(&self) -> bool {
        !self.inner.lock().unwrap().statuses.is_empty()
    }

    pub(super) fn states(&self) -> Vec<ServiceState> {
        let inner = self.inner.lock().unwrap();
        inner
            .statuses
            .values()
            .map(|status| status.state.clone())
            .collect()
    }
}

fn status(port: u16) -> ServiceStatus {
    let kind = match port {
        DICTATION | SECOND_COPY => ServiceKind::Dictation,
        COMMANDS => ServiceKind::Commands,
        _ => ServiceKind::CommandSpotting,
    };
    ServiceStatus {
        kind,
        label: kind.name().into(),
        port,
        state: ServiceState::Starting { attempt: 1 },
        owned: true,
        log_file: PathBuf::from(format!("/logs/{port}.log")),
    }
}

impl LocalServices for FakeServices {
    fn require(&mut self, config: &Config) {
        let mut needed = vec![DICTATION, SECOND_COPY];
        if config.voice_control.enabled {
            needed.extend([COMMANDS, COMMAND_SPOTTING]);
        }
        let mut inner = self.inner.lock().unwrap();
        for port in &needed {
            inner.statuses.entry(*port).or_insert_with(|| status(*port));
        }
        inner.needed = needed;
    }

    fn progress(&self) -> ServicesProgress {
        let mut inner = self.inner.lock().unwrap();
        if inner.answer_by_themselves {
            inner.look_at_starting();
        }
        let statuses: Vec<ServiceStatus> = inner.statuses.values().cloned().collect();
        ServicesProgress::of(&inner.needed, &statuses)
    }

    fn stop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        for status in inner.statuses.values_mut() {
            status.state = ServiceState::Stopped;
        }
        push(&self.events, Event::ServicesStopped);
    }
}
