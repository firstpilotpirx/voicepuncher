//! Reloading the configuration: a new pipeline once the old one is idle.

use super::{
    log_rejection, mpsc, tell, Command, Config, Next, PipelineActivity, Receiver, ReloadListener,
    ReloadReport, RuntimeState, Supervisor, Wait, IDLE_POLL_INTERVAL,
};

impl Supervisor {
    pub(super) fn reload(
        &mut self,
        config: Config,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let models_missing = self.require_services(&config);
        if self.pipeline.is_none() {
            return self.keep_for_later(config);
        }
        if models_missing {
            return self.wait_for_models(config, listener, commands, pending);
        }
        self.change(RuntimeState::Restarting);
        if self.wait_for_services(commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        let next = match Next::assemble(self.assembler.as_mut(), &config) {
            Ok(next) => next,
            Err(e) => return self.reject(listener, &e),
        };
        if self.wait_for_idle(listener, commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        self.end_pipeline(config);
        let running = match self.launch(next) {
            Ok(running) => running,
            Err(e) => return self.reject(listener, &e),
        };
        self.router.route_to(running.inputs.clone());
        self.pipeline = Some(running);
        self.change(self.ready());
        tell(listener, ReloadReport::Applied);
        Wait::Idle
    }

    /// Whether `config` needs a model that is missing; when none is, the services it needs are
    /// required from now on.
    fn require_services(&mut self, config: &Config) -> bool {
        let models_missing = self
            .gates
            .models
            .as_ref()
            .is_some_and(|models| !models.missing(config).is_empty());
        if !models_missing {
            if let Some(services) = &mut self.gates.services {
                services.require(config);
            }
            self.gates.services_required = true;
        }
        models_missing
    }

    /// Tells `listener` the configuration was rejected for `e`.
    fn reject(&self, listener: &mut Option<ReloadListener>, e: &anyhow::Error) -> Wait {
        log_rejection(e);
        tell(listener, ReloadReport::Rejected(format!("{e:#}")));
        self.change(self.ready());
        Wait::Idle
    }

    /// Without a pipeline, the configuration waits for the next start.
    pub(super) fn keep_for_later(&mut self, config: Config) -> Wait {
        self.end_pipeline(config);
        log::info!("runtime: configuration kept for when the pipeline starts");
        Wait::Idle
    }

    /// Ends the pipeline, if one runs, and keeps `config` as the configuration of the next
    /// one, its hotkey bound.
    pub(super) fn end_pipeline(&mut self, config: Config) {
        if let Some(old) = self.pipeline.take() {
            old.shut_down();
        }
        self.binding.rebind(config.hotkey.key);
        self.config = config;
    }

    /// Waits until no dictation is in progress and detaches the router from the pipeline;
    /// a stop arriving meanwhile ends the wait, a reload waits its turn.
    pub(super) fn wait_for_idle(
        &self,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let mut told = false;
        loop {
            match commands.try_recv() {
                Ok(Command::Stop) | Err(mpsc::TryRecvError::Disconnected) => return Wait::Stopped,
                Ok(reload) => pending.push_back(reload),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            let idle = self.activity.wait_idle(IDLE_POLL_INTERVAL) == PipelineActivity::Idle;
            if idle && self.detached_idle() {
                return Wait::Idle;
            }
            if !idle && !told {
                log::info!("runtime: restart waits for the dictation in progress");
                tell(listener, ReloadReport::WaitsForDictation);
                told = true;
            }
        }
    }

    /// Detaches the router and whether the pipeline is still idle after that: a signal
    /// forwarded just before the detach may still be on its way to the pipeline, so it gets
    /// the time to start its dictation, which the wait then keeps, the router attached again.
    pub(super) fn detached_idle(&self) -> bool {
        self.router.detach();
        std::thread::sleep(IDLE_POLL_INTERVAL);
        if self.activity.current() == PipelineActivity::Idle {
            return true;
        }
        if let Some(running) = &self.pipeline {
            self.router.route_to(running.inputs.clone());
        }
        false
    }
}
