use anyhow::Result;

use crate::core::domain::CleanedText;

#[cfg_attr(test, mockall::automock)]
pub trait TextPaster: Send {
    fn paste(&mut self, text: &CleanedText) -> Result<()>;
    /// Puts the text in the clipboard and keeps it there, without pasting it.
    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()>;
}
