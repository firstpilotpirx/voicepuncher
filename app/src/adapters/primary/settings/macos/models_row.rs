//! The Models row of the Services tab: how many needed model files are present, Download
//! models… and Show in Finder, over what the app's composition root tells of the models.

use std::path::PathBuf;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_app_kit::{NSTextField, NSView};
use objc2_foundation::NSString;

use super::build::{button, stack, Parts};
use super::target::Action;

pub const MODELS_LABEL: &str = "Models:";
pub const DOWNLOAD_TITLE: &str = "Download models…";
pub const SHOW_TITLE: &str = "Show in Finder";

/// The needed model files of the saved configuration, and the Download window.
pub trait ModelsPanel {
    /// How many needed files are present, of how many.
    fn count(&self) -> (usize, usize);
    /// The models folder of the saved configuration.
    fn folder(&self) -> PathBuf;
    fn open_download(&self);
}

/// The row's words.
pub fn models_line(present: usize, total: usize) -> String {
    if present == total {
        format!("Every model is present ({present} of {total})")
    } else {
        format!("{present} of {total} present — {} missing", total - present)
    }
}

/// The row's views; none without a panel (the app leaves its services to the scripts).
pub struct ModelsRow {
    pub panel: Option<Rc<dyn ModelsPanel>>,
    pub line: Retained<NSTextField>,
    pub view: Retained<NSView>,
}

impl ModelsRow {
    pub fn new(parts: &Parts, panel: Option<Rc<dyn ModelsPanel>>) -> Self {
        let line = NSTextField::labelWithString(&NSString::new(), parts.mtm);
        let download = button(parts, DOWNLOAD_TITLE, Action::DOWNLOAD_MODELS, 0);
        let show = button(parts, SHOW_TITLE, Action::SHOW_MODELS, 0);
        let view = stack(
            parts.mtm,
            &[line.as_ref(), download.as_ref(), show.as_ref()],
        );
        let row = Self { panel, line, view: Retained::into_super(view) };
        row.refresh();
        row
    }

    /// Counts the files again.
    pub fn refresh(&self) {
        let Some(panel) = &self.panel else {
            self.view.setHidden(true);
            return;
        };
        let (present, total) = panel.count();
        self.line
            .setStringValue(&NSString::from_str(&models_line(present, total)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_that_every_model_is_present() {
        assert_eq!(models_line(6, 6), "Every model is present (6 of 6)");
    }

    #[test]
    fn should_count_the_missing_models() {
        assert_eq!(models_line(4, 6), "4 of 6 present — 2 missing");
    }
}
