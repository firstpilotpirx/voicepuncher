#[cfg_attr(test, mockall::automock)]
pub trait SoundPlayer: Send {
    fn play_start(&self);
    fn play_stop(&self);
    fn play_cancel(&self);
    /// Voice control: the hotkey turned listening on.
    fn play_listening_on(&self) {}
    /// Voice control: the hotkey turned listening off.
    fn play_listening_off(&self) {}
    /// Voice control: "press enter" was carried out.
    fn play_enter(&self) {}
    /// Voice control: "clear all text" was carried out.
    fn play_clear(&self) {}
    /// Voice control: "restart dictation" started the dictation over.
    fn play_restart(&self) {}
}
