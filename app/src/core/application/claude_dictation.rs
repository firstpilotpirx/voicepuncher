use anyhow::Result;

/// The Claude desktop app's own dictation, which voicepuncher drives by key presses when
/// Claude is the frontmost app.
#[cfg_attr(test, mockall::automock)]
pub trait ClaudeDictation: Send {
    /// Whether the Claude desktop app is the frontmost app now.
    fn is_frontmost(&self) -> bool;
    /// Presses Claude's dictation shortcut: it starts Claude's dictation, or stops it.
    fn toggle(&mut self) -> Result<()>;
    /// Presses Escape: Claude drops its live dictation.
    fn cancel(&mut self) -> Result<()>;
}
