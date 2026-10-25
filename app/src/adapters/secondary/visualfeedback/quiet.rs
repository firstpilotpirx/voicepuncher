//! The overlay of file mode: nothing is shown.

use crate::core::application::VisualIndicator;

/// An overlay that shows nothing: file mode has no screen to show a dictation on.
pub struct QuietVisualIndicator;

impl VisualIndicator for QuietVisualIndicator {
    crate::shows_no_dictation_states!();
    crate::shows_no_voice_control_signs!();
}
