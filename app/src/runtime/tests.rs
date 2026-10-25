use super::*;
use crate::core::application::pipeline::{PipelineConfig, PipelineParts};
use crate::core::application::prelude::*;
use crate::core::domain::{HotKey, RecordingMode};
use std::time::Instant;

/// Longest the runtime thread may take to reach a state.
const REACTION: Duration = Duration::from_secs(2);

struct IdleAssembler;

impl PipelineAssembler for IdleAssembler {
    fn assemble(&mut self, _cfg: &Config, _in: Sender<PipelineInput>) -> Result<Pipeline> {
        Ok(Pipeline::new(
            PipelineParts {
                recorder: Box::new(MockAudioRecorder::new()),
                transcriber: Box::new(MockTranscriber::new()),
                paster: Box::new(MockTextPaster::new()),
                sound_player: Box::new(MockSoundPlayer::new()),
                visual_indicator: Box::new(MockVisualIndicator::new()),
            },
            PipelineConfig::default(),
        ))
    }
}

struct Recorded(Arc<Mutex<Vec<RuntimeState>>>);

impl RuntimeObserver for Recorded {
    fn state_changed(&self, state: &RuntimeState) {
        self.0.lock().unwrap().push(state.clone());
    }
}

fn listener() -> MockHotkeyListener {
    let mut listener = MockHotkeyListener::new();
    listener
        .expect_spawn()
        .returning(|_, _| Ok(std::thread::spawn(|| {})));
    listener
}

fn with_hotkey(hotkey: HotKey) -> Config {
    let mut config = Config::default();
    config.hotkey.key = hotkey;
    config
}

fn ready(hotkey: HotKey) -> RuntimeState {
    RuntimeState::Ready { hotkey, recording_mode: RecordingMode::Toggle, voice_control: false }
}

#[test]
fn should_tell_an_observer_every_state_of_a_start_a_reload_and_a_stop_in_order() {
    // Given
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()));
    runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
    // When
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    runtime.reload(with_hotkey(HotKey::F19)).unwrap();
    wait_for_state(&seen, &ready(HotKey::F19));
    runtime.stop();
    // Then
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            RuntimeState::Starting,
            ready(HotKey::F13),
            RuntimeState::Restarting,
            ready(HotKey::F19),
            RuntimeState::Stopped,
        ]
    );
}

struct FailingReassembly(usize);

impl PipelineAssembler for FailingReassembly {
    fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline> {
        self.0 += 1;
        if self.0 > 1 {
            bail!("speech recognition service is not reachable");
        }
        IdleAssembler.assemble(config, inputs)
    }
}

fn reports_of(runtime: &Runtime, config: Config) -> (Result<()>, Arc<Mutex<Vec<ReloadReport>>>) {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let told = Arc::clone(&reports);
    let result = runtime
        .reloader()
        .unwrap()
        .reload_reporting(config, Box::new(move |r| told.lock().unwrap().push(r)));
    (result, reports)
}

fn wait_for_report(reports: &Mutex<Vec<ReloadReport>>) -> Vec<ReloadReport> {
    wait_until(|| !reports.lock().unwrap().is_empty());
    reports.lock().unwrap().clone()
}

/// Whether `done` held within the reaction time.
fn wait_until(done: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + REACTION;
    while !done() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

#[test]
fn should_report_a_reload_applied_once_the_new_pipeline_runs() {
    // Given
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()));
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    // When
    let (result, reports) = reports_of(&runtime, with_hotkey(HotKey::F19));
    // Then
    assert!(result.is_ok());
    assert_eq!(wait_for_report(&reports), vec![ReloadReport::Applied]);
    assert_eq!(runtime.state(), Some(ready(HotKey::F19)));
}

#[test]
fn should_report_a_reload_rejected_with_the_reason_a_service_gave() {
    // Given
    let mut runtime = Runtime::new(Box::new(FailingReassembly(0)), Box::new(listener()));
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    // When
    let (_, reports) = reports_of(&runtime, with_hotkey(HotKey::F19));
    // Then
    let reports = wait_for_report(&reports);
    assert!(
        matches!(&reports[..], [ReloadReport::Rejected(reason)] if reason.contains("speech recognition")),
        "{reports:?}"
    );
}

#[test]
fn should_report_a_configuration_that_does_not_validate_rejected_at_once() {
    // Given
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()));
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    let mut remote = with_hotkey(HotKey::F19);
    remote.transcription.base_url = "http://192.0.2.1:8081".into();
    // When
    let (result, reports) = reports_of(&runtime, remote);
    // Then
    assert!(result.is_err());
    assert!(matches!(
        &reports.lock().unwrap()[..],
        [ReloadReport::Rejected(_)]
    ));
}

#[test]
fn should_tell_the_recording_mode_of_the_configuration_when_ready() {
    // Given
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()));
    let mut config = with_hotkey(HotKey::F13);
    config.hotkey.recording_mode = RecordingMode::Hold;
    // When
    runtime.start(config).unwrap();
    // Then
    assert!(matches!(
        runtime.state(),
        Some(RuntimeState::Ready { recording_mode: RecordingMode::Hold, .. })
    ));
}

#[test]
fn should_tell_an_observer_registered_after_the_start_ready_at_once() {
    // Given
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()));
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    // When
    runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
    // Then
    assert_eq!(*seen.lock().unwrap(), vec![ready(HotKey::F13)]);
}
/// The model files still missing, shared with the test.
#[derive(Clone, Default)]
struct FakeModels(Arc<Mutex<Vec<&'static str>>>);

impl FakeModels {
    fn missing_now(&self, files: &[&'static str]) {
        *self.0.lock().unwrap() = files.to_vec();
    }
}

impl ModelsGate for FakeModels {
    fn folder(&self, _config: &Config) -> std::path::PathBuf {
        "/models".into()
    }

    fn missing(&self, _config: &Config) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }
}

/// Services that answer as soon as they are required; how often they were.
#[derive(Clone, Default)]
/// Counts the requires; ready from the second look after the first require.
pub(crate) struct FakeServices(Arc<Mutex<usize>>, Arc<Mutex<usize>>);

impl LocalServices for FakeServices {
    fn require(&mut self, _config: &Config) {
        *self.0.lock().unwrap() += 1;
    }

    fn progress(&self) -> crate::core::domain::ServicesProgress {
        let required = *self.0.lock().unwrap() > 0;
        let mut looks = self.1.lock().unwrap();
        if required {
            *looks += 1;
        }
        crate::core::domain::ServicesProgress {
            ready: usize::from(*looks > 1),
            total: 1,
            failed: None,
        }
    }

    fn stop(&mut self) {}
}

fn waiting_for(files: &[&'static str]) -> RuntimeState {
    RuntimeState::WaitingForModels { folder: "/models".into(), missing: files.to_vec() }
}

fn wait_for_state(seen: &Mutex<Vec<RuntimeState>>, state: &RuntimeState) {
    let reached = wait_until(|| seen.lock().unwrap().last() == Some(state));
    assert!(reached, "seen: {:?}", seen.lock().unwrap());
}

#[test]
fn should_wait_for_models_without_requiring_a_service() {
    // Given
    let (models, services) = (FakeModels::default(), FakeServices::default());
    let (mut runtime, seen) = runtime_missing_the_dictation_model(&models, &services);
    // When
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    // Then
    assert_eq!(
        *seen.lock().unwrap(),
        vec![waiting_for(&["ggml-large-v3-turbo-q5_0.bin"])]
    );
    assert_eq!(*services.0.lock().unwrap(), 0);
}

#[test]
fn should_start_the_services_and_the_pipeline_once_the_models_are_present() {
    // Given
    let (models, services) = (FakeModels::default(), FakeServices::default());
    let (mut runtime, seen) = runtime_missing_the_dictation_model(&models, &services);
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    // When
    models.missing_now(&[]);
    wait_for_state(&seen, &ready(HotKey::F13));
    // Then
    assert_eq!(*services.0.lock().unwrap(), 1);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            waiting_for(&["ggml-large-v3-turbo-q5_0.bin"]),
            RuntimeState::StartingServices { ready: 0, total: 1 },
            RuntimeState::Starting,
            ready(HotKey::F13),
        ]
    );
}

#[test]
fn should_end_the_pipeline_and_wait_when_a_reload_needs_missing_models() {
    // Given
    let seen = Arc::new(Mutex::new(Vec::new()));
    let models = FakeModels::default();
    let mut runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()))
        .with_services(Box::new(FakeServices::default()))
        .with_models(Box::new(models.clone()));
    runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
    runtime.start(with_hotkey(HotKey::F13)).unwrap();
    wait_for_state(&seen, &ready(HotKey::F13));
    seen.lock().unwrap().clear();
    // When
    models.missing_now(&["ggml-small.bin"]);
    runtime.reload(with_hotkey(HotKey::F19)).unwrap();
    wait_for_state(&seen, &waiting_for(&["ggml-small.bin"]));
    // Then
    assert_eq!(
        *seen.lock().unwrap(),
        vec![RuntimeState::Restarting, waiting_for(&["ggml-small.bin"]),]
    );
}

/// A runtime whose dictation model is missing from `models`, with `services`, and the log of
/// the states it goes through.
fn runtime_missing_the_dictation_model(
    models: &FakeModels,
    services: &FakeServices,
) -> (Runtime, Arc<Mutex<Vec<RuntimeState>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    models.missing_now(&["ggml-large-v3-turbo-q5_0.bin"]);
    let runtime = Runtime::new(Box::new(IdleAssembler), Box::new(listener()))
        .with_services(Box::new(services.clone()))
        .with_models(Box::new(models.clone()));
    runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
    (runtime, seen)
}
