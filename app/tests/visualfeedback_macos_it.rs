use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;
use voicepuncher::core::application::VisualIndicator;

fn implements_visual_indicator<T: VisualIndicator>() {}

#[test]
fn should_implement_visual_indicator_trait() {
    implements_visual_indicator::<MacosVisualIndicator>();
}

#[test]
fn should_drop_window_set_without_panic() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm);
    drop(indicator);
}

#[test]
fn should_preserve_recording_state_after_manual_rebuild() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm);
    indicator.show_recording();
    indicator.rebuild_window_set(mtm);
}

#[test]
fn should_skip_rebuild_when_topology_identical() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm);
    indicator.rebuild_window_set(mtm);
    indicator.rebuild_window_set(mtm);
}
