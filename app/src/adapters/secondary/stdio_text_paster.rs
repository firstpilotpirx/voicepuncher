use anyhow::Result;

use crate::core::application::ports::text_paster::TextPaster;
use crate::core::domain::CleanedText;

pub struct StdioTextPaster;

impl StdioTextPaster {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdioTextPaster {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPaster for StdioTextPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        println!("{}", text.as_str());
        Ok(())
    }
}
