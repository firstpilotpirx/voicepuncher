use anyhow::Result;

/// Posts key presses into the focused app.
#[cfg_attr(test, mockall::automock)]
pub trait KeyPresser: Send {
    /// Presses Return.
    fn press_enter(&mut self) -> Result<()>;
    /// Presses Cmd+A, then Delete.
    fn clear_all_text(&mut self) -> Result<()>;
}
