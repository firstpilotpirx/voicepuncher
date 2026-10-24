//! The recording's level waveform: rounded columns of the microphone level that enter at the
//! right edge and slide left, fading out over the leftmost points.

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSColor, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};

use crate::core::domain::level_history::{
    COLUMN_PITCH_PT, FULL_CAPACITY, MAX_HEIGHT_PT, MIN_HEIGHT_PT,
};
use crate::core::domain::{AudioLevel, LevelHistory};

const COLUMN_WIDTH: f64 = 2.0;
/// A silent column, a dot, shows at this share of the content's opacity.
const DOT_OPACITY: f32 = 0.45;
/// The leftmost points over which old columns fade out.
const FADE_WIDTH: f64 = 30.0;
const PITCH: f64 = COLUMN_PITCH_PT as f64;
const AREA_HEIGHT: f64 = MAX_HEIGHT_PT as f64;

/// The level history of the running recording and the moment it started.
pub(super) struct LevelWave {
    history: LevelHistory,
    origin: Instant,
}

impl LevelWave {
    /// A history of silence starting now.
    pub(super) fn starting_now() -> Self {
        Self { history: LevelHistory::new(FULL_CAPACITY, Duration::ZERO), origin: Instant::now() }
    }

    /// How long ago the recording started.
    pub(super) fn elapsed(&self) -> Duration {
        self.origin.elapsed()
    }

    /// Takes the microphone level heard now and returns the moment it was heard.
    pub(super) fn record(&mut self, level: AudioLevel) -> Duration {
        let at = self.origin.elapsed();
        self.history.record(level, at);
        at
    }
}

/// Width of a waveform of `columns` columns.
pub(super) fn waveform_width(columns: usize) -> f64 {
    columns.saturating_sub(1) as f64 * PITCH + COLUMN_WIDTH
}

/// The waveform's view and its column layers, made once and reused every frame.
pub(super) struct WaveformView {
    pub(super) view: Retained<NSView>,
    columns: Vec<Retained<CALayer>>,
}

impl WaveformView {
    pub(super) fn build(mtm: MainThreadMarker) -> Self {
        let view = NSView::new(mtm);
        view.setWantsLayer(true);
        let columns: Vec<Retained<CALayer>> = (0..FULL_CAPACITY).map(|_| column_layer()).collect();
        if let Some(layer) = view.layer() {
            layer.setMasksToBounds(true);
            for column in &columns {
                layer.addSublayer(column);
            }
        }
        Self { view, columns }
    }

    /// Draws the columns in `colour`.
    pub(super) fn set_colour(&self, colour: &NSColor) {
        let colour = colour.CGColor();
        without_animation(|| {
            for column in &self.columns {
                column.setBackgroundColor(Some(&colour));
            }
        });
    }

    /// Draws the newest `visible` columns of `wave` at `at`, slid left by how far the newest
    /// column has come, or standing still when `sliding` is off.
    pub(super) fn draw(&self, wave: &LevelWave, at: Duration, visible: usize, sliding: bool) {
        let slide = if sliding {
            f64::from(wave.history.slide_offset(at))
        } else {
            0.0
        };
        let hidden = FULL_CAPACITY.saturating_sub(visible);
        without_animation(|| {
            let heights = wave.history.column_heights().skip(hidden);
            for (index, (column, height)) in self.columns.iter().zip(heights).enumerate() {
                let x = index as f64 * PITCH - slide;
                place(column, x, f64::from(height));
                column.setOpacity(column_opacity(x, height));
                column.setHidden(false);
            }
            for column in self.columns.iter().skip(visible) {
                column.setHidden(true);
            }
        });
    }

    /// The tallest column shown, in points.
    pub(super) fn tallest_column(&self) -> f64 {
        self.columns
            .iter()
            .filter(|column| !column.isHidden())
            .map(|column| column.frame().size.height)
            .fold(0.0, f64::max)
    }
}

/// Opacity of a column at `x`: dots dimmer than speech, both fading out near the left edge.
fn column_opacity(x: f64, height: f32) -> f32 {
    let fade = (x / FADE_WIDTH).clamp(0.0, 1.0) as f32;
    let base = if height <= MIN_HEIGHT_PT {
        DOT_OPACITY
    } else {
        1.0
    };
    base * fade
}

fn place(column: &CALayer, x: f64, height: f64) {
    column.setFrame(NSRect {
        origin: NSPoint { x, y: (AREA_HEIGHT - height) / 2.0 },
        size: NSSize { width: COLUMN_WIDTH, height },
    });
}

fn column_layer() -> Retained<CALayer> {
    let column = CALayer::new();
    column.setCornerRadius(COLUMN_WIDTH / 2.0);
    column
}

/// Runs `change` with Core Animation's implicit animations off, so the columns move only as
/// the waveform says.
fn without_animation(change: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    change();
    CATransaction::commit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_as_wide_as_ninety_columns_four_points_apart() {
        assert_eq!(waveform_width(90), 358.0);
        assert_eq!(waveform_width(78), 310.0);
    }

    #[test]
    fn should_draw_a_dot_dimmer_than_a_column_of_speech() {
        assert_eq!(column_opacity(60.0, MIN_HEIGHT_PT), DOT_OPACITY);
        assert_eq!(column_opacity(60.0, 20.0), 1.0);
    }

    #[test]
    fn should_fade_out_columns_near_the_left_edge() {
        assert_eq!(column_opacity(15.0, 20.0), 0.5);
        assert_eq!(column_opacity(-3.0, 20.0), 0.0);
    }
}
