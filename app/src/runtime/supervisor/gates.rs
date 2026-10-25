//! The gates before a pipeline starts: permissions, models and services.

use super::{
    mpsc, tell, Command, Config, Context, Next, PttSignal, Receiver, ReloadListener, ReloadReport,
    Result, RuntimeState, Supervisor, Verdict, Wait, SERVICES_POLL,
};

impl Supervisor {
    /// Waiting for models when a file the configuration needs is missing; otherwise the
    /// services are told what it needs, once.
    pub(super) fn models_missing(&mut self) -> Option<RuntimeState> {
        let missing = match &self.gates.models {
            Some(models) => models.missing(&self.config),
            None => Vec::new(),
        };
        if let Some(models) = self.gates.models.as_ref().filter(|_| !missing.is_empty()) {
            self.gates.services_required = false;
            let folder = models.folder(&self.config);
            return Some(RuntimeState::WaitingForModels { folder, missing });
        }
        if !self.gates.services_required {
            if let Some(services) = &mut self.gates.services {
                services.require(&self.config);
            }
            self.gates.services_required = true;
        }
        None
    }

    /// What keeps the pipeline from starting: a missing permission, then a missing model
    /// file, then a needed service that failed or does not answer yet; none when it may start.
    pub(crate) fn gate(&mut self) -> Option<RuntimeState> {
        let models = self.models_missing();
        if let Some(watched) = self.gates.permissions.as_ref().filter(|w| !w.all_granted()) {
            return Some(watched.waiting());
        }
        if models.is_some() {
            return models;
        }
        let progress = self.gates.services.as_ref()?.progress();
        if let Some(reason) = progress.failed {
            return Some(RuntimeState::Failed(reason));
        }
        (!progress.all_ready()).then_some(RuntimeState::StartingServices {
            ready: progress.ready,
            total: progress.total,
        })
    }

    /// Starts a pipeline from the configuration, and the hotkey listener with the first one.
    pub(crate) fn open(&mut self) -> Result<()> {
        let next = Next::assemble(self.assembler.as_mut(), &self.config)?;
        let running = self.launch(next)?;
        self.router.route_to(running.inputs.clone());
        self.pipeline = Some(running);
        if let Some(listener) = self.listener.take() {
            let (hotkey_tx, hotkey_rx) = mpsc::channel::<PttSignal>();
            listener
                .spawn(self.binding.clone(), hotkey_tx)
                .context("failed to spawn hotkey listener")?;
            self.router
                .spawn_forwarding(hotkey_rx)
                .context("failed to spawn the hotkey router")?;
        }
        Ok(())
    }

    /// Closes the pipeline when a permission is revoked, and starts it once every permission
    /// is granted and every needed service answers.
    pub(super) fn check_gates(&mut self) {
        let pipeline_runs = self.pipeline.is_some();
        if let Some(watched) = &mut self.gates.permissions {
            match watched.check(pipeline_runs) {
                Verdict::StillWaiting(state) => self.change_to_new(state),
                Verdict::Close(state) => {
                    self.close(&state);
                    self.change(state);
                }
                Verdict::Unchanged | Verdict::Open => {}
            }
        }
        if self.pipeline.is_some() || self.health.failed {
            return;
        }
        match self.gate() {
            Some(state) => self.change_to_new(state),
            None => self.start_pipeline(),
        }
    }

    /// Ends the pipeline for a revoked permission: a recording in progress is cancelled.
    pub(super) fn close(&mut self, waiting: &RuntimeState) {
        self.router.detach();
        let Some(running) = self.pipeline.take() else {
            return;
        };
        match waiting {
            RuntimeState::WaitingForPermissions { revoked, .. } if !revoked.is_empty() => {
                running.revoke(revoked[0]);
            }
            _ => running.shut_down(),
        }
    }

    /// Waits until every service the new configuration needs answers; with one failed, the
    /// running pipeline keeps its settings until a later reload.
    pub(super) fn wait_for_services(
        &self,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let Some(services) = &self.gates.services else {
            return Wait::Idle;
        };
        loop {
            let progress = services.progress();
            if let Some(reason) = &progress.failed {
                log::warn!("runtime: a needed service failed: {reason}");
                return Wait::Idle;
            }
            if progress.all_ready() {
                return Wait::Idle;
            }
            self.change_to_new(RuntimeState::StartingServices {
                ready: progress.ready,
                total: progress.total,
            });
            match commands.recv_timeout(SERVICES_POLL) {
                Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Wait::Stopped
                }
                Ok(reload) => pending.push_back(reload),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// Ends the pipeline once no dictation is in progress and waits for the models `config`
    /// needs; the services are told what it needs once they are present.
    pub(super) fn wait_for_models(
        &mut self,
        config: Config,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        self.change(RuntimeState::Restarting);
        if self.wait_for_idle(listener, commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        self.end_pipeline(config);
        if let Some(state) = self.models_missing() {
            self.change(state);
        }
        tell(listener, ReloadReport::Applied);
        Wait::Idle
    }
}
