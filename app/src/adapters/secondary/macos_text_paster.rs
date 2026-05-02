use anyhow::Result;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::core::application::ports::text_paster::TextPaster;
use crate::core::domain::CleanedText;

pub struct MacOsTextPaster {
    restore_delay_ms: u64,
}

impl MacOsTextPaster {
    pub fn new(restore_delay_ms: u64) -> Self {
        Self { restore_delay_ms }
    }
}

impl TextPaster for MacOsTextPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        let prior: Option<String> = clipboard.get_text().ok();
        clipboard.set_text(text.as_str().to_owned())?;
        std::thread::sleep(std::time::Duration::from_millis(20));

        let mut enigo = Enigo::new(&Settings::default())?;
        enigo.key(Key::Meta, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Meta, Direction::Release)?;

        let delay_ms = self.restore_delay_ms;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            match arboard::Clipboard::new() {
                Ok(mut cb) => match prior {
                    Some(text) => {
                        if let Err(e) = cb.set_text(text) {
                            log::warn!("clipboard restore failed: {e}");
                        }
                    }
                    None => {
                        if let Err(e) = cb.clear() {
                            log::warn!("clipboard clear failed: {e}");
                        }
                    }
                },
                Err(e) => log::warn!("clipboard restore: failed to open clipboard: {e}"),
            }
        });

        Ok(())
    }
}
