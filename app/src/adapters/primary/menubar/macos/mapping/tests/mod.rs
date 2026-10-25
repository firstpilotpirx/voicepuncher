use super::*;

mod icons;
mod items;
mod signals;
mod words;

fn ready(mode: RecordingMode, voice_control: bool) -> RuntimeState {
    RuntimeState::Ready { hotkey: HotKey::RightOption, recording_mode: mode, voice_control }
}

fn state(dictation: Option<DictationStage>, runtime: Option<RuntimeState>) -> StatusItemState {
    StatusItemState { dictation, runtime, commands_unavailable: false }
}

fn ready_state() -> StatusItemState {
    state(None, Some(ready(RecordingMode::Hold, false)))
}
