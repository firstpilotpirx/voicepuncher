use anyhow::Result;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::core::application::text_paster::TextPaster;
use crate::core::domain::CleanedText;

pub struct MacOsTextPaster {
    restore_delay_ms: u64,
}

impl MacOsTextPaster {
    pub fn new(restore_delay_ms: u64) -> Self {
        Self { restore_delay_ms }
    }

    fn post_cmd_v() -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|()| anyhow::anyhow!("failed to create CGEventSource"))?;

        let down = CGEvent::new_keyboard_event(source.clone(), KeyCode::ANSI_V, true)
            .map_err(|()| anyhow::anyhow!("failed to create Cmd+V key-down event"))?;
        down.set_flags(CGEventFlags::CGEventFlagCommand);
        down.post(CGEventTapLocation::Session);

        let up = CGEvent::new_keyboard_event(source, KeyCode::ANSI_V, false)
            .map_err(|()| anyhow::anyhow!("failed to create Cmd+V key-up event"))?;
        up.set_flags(CGEventFlags::CGEventFlagCommand);
        up.post(CGEventTapLocation::Session);

        Ok(())
    }
}

impl TextPaster for MacOsTextPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        let prior: Option<String> = clipboard.get_text().ok();
        clipboard.set_text(text.as_str().to_owned())?;
        std::thread::sleep(std::time::Duration::from_millis(20));

        Self::post_cmd_v()?;

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
