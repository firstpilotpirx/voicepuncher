use anyhow::Result;

use crate::core::domain::CleanedText;

#[cfg_attr(test, mockall::automock)]
pub trait TextPaster: Send {
    fn paste(&mut self, text: &CleanedText) -> Result<()>;
}
