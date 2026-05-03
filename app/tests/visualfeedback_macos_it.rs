use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;
use voicepuncher::core::application::VisualIndicator;

fn implements_visual_indicator<T: VisualIndicator>() {}

#[test]
fn should_implement_visual_indicator_trait() {
    implements_visual_indicator::<MacosVisualIndicator>();
}
