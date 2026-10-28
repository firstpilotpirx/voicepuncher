//! The thread that streams the microphone to the service and reads its events.

use super::{
    heard, link, Arc, AudioBuffer16k, CapturedAudio, Control, Duration, HeardAudio, Instant, Link,
    LinkTimings, Mutex, Ordering, Outage, PipelineInput, Receiver, Result, Sender, SpottingClient,
    SpottingEvent, SpottingService, StreamResampler, TryRecvError, VoiceCommand, AWAY_WAIT,
    NEXT_SESSION,
};

/// What the streaming worker owns.
pub(super) struct Stream {
    pub(crate) service: SpottingService,
    pub(crate) session: u64,
    pub(crate) link: Link,
    /// How the attempts after a loss are paced and how far back the replay reaches.
    pub(crate) timings: LinkTimings,
    /// The commands listened for now, sent again to a new connection.
    pub(crate) commands: Vec<VoiceCommand>,
    /// The end of the last command reported: a replay never reaches back to it.
    pub(crate) last_command_end: Option<Instant>,
    pub(crate) resampler: StreamResampler,
    pub(crate) audio: Box<dyn CapturedAudio>,
    pub(crate) control: Receiver<Control>,
    pub(crate) inputs: Sender<PipelineInput>,
    pub(crate) heard: Arc<Mutex<HeardAudio>>,
}

/// Runs until told to stop; a lost service is reported once, a long outage once, and its
/// return once.
pub(super) fn stream_to_service(mut stream: Stream) {
    loop {
        match stream.control.try_recv() {
            Ok(Control::ListenFor(commands)) => stream.listen_for(commands),
            Ok(Control::Stop) | Err(TryRecvError::Disconnected) => return stream.close(),
            Err(TryRecvError::Empty) => {}
        }
        stream.stream_audio();
        match &stream.link {
            Link::Up(_) => stream.exchange(),
            Link::Away(_) => stream.reach_for_service(),
        }
    }
}

impl Stream {
    fn listen_for(&mut self, commands: Vec<VoiceCommand>) {
        log::info!(
            "command spotting: session={} listening for {commands:?}",
            self.session
        );
        self.commands = commands;
        if let Link::Up(client) = &mut self.link {
            if let Err(e) = client.listen_for(&self.commands) {
                self.lose(&e);
            }
        }
    }

    /// Holds every sample captured, and sends it while the service is there.
    fn stream_audio(&mut self) {
        let captured = self.audio.take();
        let samples = self.resampler.push(&captured);
        if samples.is_empty() {
            return;
        }
        if let Ok(mut heard) = self.heard.lock() {
            heard.push(&samples, Instant::now());
        }
        if let Link::Up(client) = &mut self.link {
            if let Err(e) = client.send_audio(&samples) {
                self.lose(&e);
            }
        }
    }

    /// Checks that the service answers and passes its events on.
    fn exchange(&mut self) {
        let Link::Up(client) = &mut self.link else {
            return;
        };
        let events = client.keep_alive().and_then(|()| client.read_events());
        match events {
            Ok(events) => {
                for mut event in events {
                    self.note_command_end(&event);
                    self.to_capture_times(&mut event);
                    let _ = self.inputs.send(PipelineInput::Spotting(event));
                }
            }
            Err(e) => self.lose(&e),
        }
    }

    /// The report times of `event`, dated by stream position, as the moments that audio was
    /// captured: they fall behind the clock once audio was lost. Its own times stay, as the
    /// lead-in is taken by them.
    fn to_capture_times(&self, event: &mut SpottingEvent) {
        let (SpottingEvent::PhraseSpotted { timing: Some(report), .. }
        | SpottingEvent::CommandsSaid { timing: Some(report), .. }) = event
        else {
            return;
        };
        let Ok(heard) = self.heard.lock() else {
            return;
        };
        report.words_ended = heard.captured_at(report.words_ended);
        if let Some(confirmation) = report.confirmation.as_mut() {
            confirmation.asked_at = heard.captured_at(confirmation.asked_at);
        }
    }

    fn note_command_end(&mut self, event: &SpottingEvent) {
        let end = match event {
            SpottingEvent::PhraseSpotted { end, .. } | SpottingEvent::CommandsSaid { end, .. } => {
                *end
            }
            _ => return,
        };
        self.last_command_end = Some(self.last_command_end.map_or(end, |last| last.max(end)));
    }

    fn lose(&mut self, error: &anyhow::Error) {
        log::error!("command spotting: session={} lost: {error:#}", self.session);
        let lost_at = Instant::now();
        if let Link::Up(client) = std::mem::replace(
            &mut self.link,
            Link::Away(Outage::since(lost_at, self.timings)),
        ) {
            client.close();
        }
        let _ = self.inputs.send(PipelineInput::SpottingLost);
    }

    /// Tries the service again when the pause after the last attempt is over; tells the
    /// pipeline once when the outage has lasted too long for the replay to hide it.
    fn reach_for_service(&mut self) {
        let Link::Away(outage) = &mut self.link else {
            return;
        };
        let now = Instant::now();
        if outage.became_long(now) {
            let _ = self.inputs.send(PipelineInput::SpottingStillAway);
        }
        if !outage.due(now) {
            std::thread::sleep(AWAY_WAIT);
            return;
        }
        // The loss dated by stream position, as the command ends and the replay are.
        let lost_at = outage.lost_at;
        let replay_from = self
            .timings
            .replay_start(heard::dated(&self.heard, lost_at), self.last_command_end);
        match self.reconnect(replay_from) {
            Ok((client, replayed)) => self.back(client, lost_at, replayed),
            Err(e) => self.attempt_failed(now, &e),
        }
    }

    /// Streams on over `client`, the connection made again after the loss at `lost_at`.
    fn back(&mut self, client: SpottingClient, lost_at: Instant, replayed: Duration) {
        self.session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        log::info!(
            "command spotting: session={} back after {:.1}s away, {:.1}s of audio sent again",
            self.session,
            lost_at.elapsed().as_secs_f32(),
            replayed.as_secs_f32()
        );
        self.link = Link::Up(Box::new(client));
        let _ = self.inputs.send(PipelineInput::SpottingBack);
    }

    /// Counts a failed attempt to reach the service at `now` and logs it when due.
    fn attempt_failed(&mut self, now: Instant, e: &anyhow::Error) {
        let Link::Away(outage) = &mut self.link else {
            return;
        };
        let pause = outage.failed(now);
        if link::attempt_logged(outage.failed_attempts) {
            log::info!(
                "command spotting: session={} attempt {} to reach {} failed, next in {}ms: {e:#}",
                self.session,
                outage.failed_attempts,
                self.service.url,
                pause.as_millis()
            );
        }
    }

    /// A new connection, told the current commands and sent the audio held from `replay_from`.
    fn reconnect(&self, replay_from: Instant) -> Result<(SpottingClient, Duration)> {
        let (audio_start, samples) = self.heard.lock().map_or_else(
            |_| (Instant::now(), Vec::new()),
            |heard| heard.replay_from(replay_from),
        );
        let refused = |port| self.service.refused.report(port);
        let mut client =
            SpottingClient::connect_reporting(&self.service.url, audio_start, &refused)?
                .confirming(self.service.confirm_commands);
        client.listen_for(&self.commands)?;
        if !samples.is_empty() {
            client.send_audio(&samples)?;
        }
        let replayed =
            Duration::from_secs_f32(samples.len() as f32 / AudioBuffer16k::SAMPLE_RATE_HZ as f32);
        Ok((client, replayed))
    }

    fn close(self) {
        if let Link::Up(client) = self.link {
            client.close();
        }
        log::info!("command spotting: session={} closed", self.session);
    }
}
