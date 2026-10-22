//! The stand-ins of the app runtime scenarios.

use super::{
    push, Arc, AtomicUsize, CleanedText, CommandSpotter, Event, Events, FailureNotice, FocusProbe,
    FocusedElement, HotKey, HotkeyBinding, HotkeyListener, JoinHandle, Mutex, Ordering, Part,
    Permission, PermissionProbe, PermissionStatus, PermissionsGate, PttSignal, Result,
    RuntimeObserver, RuntimeState, Sender, SoundPlayer, TextPaster, VisualIndicator, VoiceCommand,
};

pub(super) struct StandInPaster(pub(super) Part);

impl TextPaster for StandInPaster {
    fn paste(&mut self, _text: &CleanedText) -> Result<()> {
        self.0.log(Event::Pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, _text: &CleanedText) -> Result<()> {
        Ok(())
    }
}

pub(super) struct StandInSounds(pub(super) Part);

impl SoundPlayer for StandInSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {
        self.0.log(Event::CancelPlayed);
    }
}

/// Shows nothing; a failure notice goes to the log.
pub(super) struct NoticeOverlay(pub(super) Part);

impl VisualIndicator for NoticeOverlay {
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {
        self.0.log(Event::Hidden);
    }
    fn show_failure_notice(&self, notice: &FailureNotice) {
        push(
            &self.0.events,
            Event::Notice(self.0.generation, notice.message()),
        );
    }
    voicepuncher::shows_no_voice_control_signs!();
}

pub(super) struct ScriptedFocus(pub(super) Arc<Mutex<bool>>);

impl FocusProbe for ScriptedFocus {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        let text_field = *self.0.lock().unwrap();
        Ok(Some(FocusedElement {
            role: if text_field {
                "AXTextField"
            } else {
                "AXButton"
            }
            .to_owned(),
            has_text_selection: text_field,
            value_settable: text_field,
            in_web_content: false,
        }))
    }
}

pub(super) struct StandInSpotter(pub(super) Part);

impl CommandSpotter for StandInSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, _commands: &[VoiceCommand]) -> Result<()> {
        self.0.log(Event::ListeningOn);
        Ok(())
    }

    fn listen_for(&mut self, _commands: &[VoiceCommand]) {}

    fn stop_listening(&mut self) {
        self.0.log(Event::ListeningOff);
    }
    voicepuncher::holds_no_heard_sound!();
}

pub(super) struct RecordingObserver(pub(super) Events);

impl RuntimeObserver for RecordingObserver {
    fn state_changed(&self, state: &RuntimeState) {
        push(&self.0, Event::State(state.clone()));
    }
}

/// The keyboard the runtime's listener reads: a press reaches the runtime only when the key
/// is the one the binding holds at that moment, as the real listener matches every event.
#[derive(Clone, Default)]
pub(super) struct FakeKeyboard {
    pub(super) wired: Arc<Mutex<Option<Wiring>>>,
}

/// The binding the runtime handed the listener, and where its signals go.
pub(super) type Wiring = (HotkeyBinding, Sender<PttSignal>);

impl FakeKeyboard {
    pub(super) fn press(&self, key: HotKey) {
        if let Some((binding, tx)) = self.wired.lock().unwrap().as_ref() {
            if binding.current() == key {
                let _ = tx.send(PttSignal::Pressed);
                let _ = tx.send(PttSignal::Released);
            }
        }
    }
}

impl HotkeyListener for FakeKeyboard {
    fn spawn(
        self: Box<Self>,
        binding: HotkeyBinding,
        tx: Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        *self.wired.lock().unwrap() = Some((binding, tx));
        Ok(std::thread::spawn(|| {}))
    }
}

pub(super) type Statuses = Arc<Mutex<[PermissionStatus; 3]>>;

/// The permissions as System Settings holds them, which the scenario changes, and how often
/// the runtime asked.
#[derive(Clone)]
pub(crate) struct Permissions {
    statuses: Statuses,
    checks: Arc<AtomicUsize>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            statuses: Arc::new(Mutex::new([PermissionStatus::Granted; 3])),
            checks: Arc::default(),
        }
    }
}

impl Permissions {
    pub(crate) fn set(&self, permission: Permission, status: PermissionStatus) {
        self.statuses.lock().unwrap()[index(permission)] = status;
    }

    pub(crate) fn checks(&self) -> usize {
        self.checks.load(Ordering::SeqCst)
    }

    /// A gate over the scenario's permissions, as the window asks it.
    pub(crate) fn gate(&self) -> Arc<PermissionsGate> {
        Arc::new(PermissionsGate::new(Box::new(ScriptedProbe(self.clone()))))
    }
}

pub(super) fn index(permission: Permission) -> usize {
    Permission::ALL
        .iter()
        .position(|p| *p == permission)
        .expect("every permission is in ALL")
}

pub(super) struct ScriptedProbe(pub(super) Permissions);

impl PermissionProbe for ScriptedProbe {
    fn status(&self, permission: Permission) -> PermissionStatus {
        self.0.checks.fetch_add(1, Ordering::SeqCst);
        self.0.statuses.lock().unwrap()[index(permission)]
    }

    fn request(&self, _permission: Permission) {}

    fn settings_url(&self, permission: Permission) -> String {
        format!("settings:{permission}")
    }

    fn responsible_app(&self) -> Option<String> {
        None
    }
}
