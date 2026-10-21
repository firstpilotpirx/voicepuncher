//! Integration tests for the status item in the macOS menu bar: the item itself, what its
//! menu says, and the actions it runs against the real system. AppKit builds the item only on
//! the main thread, and libtest runs each test on a thread of its own, so this harness
//! (`harness = false`) answers cargo-nextest's listing and runs each test on the main thread.

use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use voicepuncher::adapters::primary::menubar::macos::actions::{
    ensure_folder, open_in_text_editor, show_in_file_browser,
};
use voicepuncher::adapters::primary::menubar::macos::indicator::StatusItemCell;
use voicepuncher::adapters::primary::menubar::macos::mapping::{title_line, StatusItemState};
use voicepuncher::adapters::primary::menubar::macos::status_item::MacosStatusItem;
use voicepuncher::core::domain::{HotKey, RecordingMode};
use voicepuncher::runtime::{RuntimeObserver, RuntimeState};

type Test = (&'static str, fn());

const TESTS: &[Test] = &[
    (
        "should_title_the_menu_with_the_name_and_version",
        should_title_the_menu_with_the_name_and_version,
    ),
    (
        "should_say_starting_before_the_runtime_reports",
        should_say_starting_before_the_runtime_reports,
    ),
    (
        "should_create_a_missing_logs_folder",
        should_create_a_missing_logs_folder,
    ),
    (
        "should_keep_an_existing_logs_folder_and_its_files",
        should_keep_an_existing_logs_folder_and_its_files,
    ),
    (
        "should_show_the_logs_folder_in_the_file_browser",
        should_show_the_logs_folder_in_the_file_browser,
    ),
    (
        "should_open_the_configuration_file_in_a_text_editor",
        should_open_the_configuration_file_in_a_text_editor,
    ),
    (
        "should_put_the_status_item_in_the_menu_bar_and_take_it_away",
        should_put_the_status_item_in_the_menu_bar_and_take_it_away,
    ),
];

/// How long the main queue gets to draw what was dispatched to it.
const DRAW: f64 = 0.2;
/// The failure notice's lifetime in these tests; none is shown.
const NOTICE_LIFETIME: Duration = Duration::from_secs(4);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse", "--ignored"] => {}
        ["--list", "--format", "terse"] => {
            for (name, _) in TESTS {
                println!("{name}: test");
            }
        }
        ["--exact", name, ..] => {
            let (_, test) = TESTS
                .iter()
                .find(|(listed, _)| listed == name)
                .unwrap_or_else(|| panic!("no test named {name}"));
            test();
        }
        _ => TESTS.iter().for_each(|(_, test)| test()),
    }
}

fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("the harness runs every test on the main thread")
}

fn pump_run_loop(secs: f64) {
    use objc2_foundation::{NSDate, NSRunLoop};
    let run_loop = NSRunLoop::currentRunLoop();
    let deadline = NSDate::dateWithTimeIntervalSinceNow(secs);
    run_loop.runUntilDate(&deadline);
}

fn should_put_the_status_item_in_the_menu_bar_and_take_it_away() {
    // Given
    let mtm = main_thread();
    let _app = NSApplication::sharedApplication(mtm);
    let status_item = MacosStatusItem::install(mtm);
    let cell = StatusItemCell::new(Box::new(status_item.icon()), NOTICE_LIFETIME);
    status_item.connect(cell.clone());
    pump_run_loop(DRAW);
    assert_eq!(status_item.shown_symbol(mtm).as_deref(), Some("hourglass"));
    // When
    cell.observer().state_changed(&RuntimeState::Ready {
        hotkey: HotKey::RightOption,
        recording_mode: RecordingMode::Hold,
        voice_control: false,
    });
    pump_run_loop(DRAW);
    // Then
    assert_eq!(status_item.shown_symbol(mtm).as_deref(), Some("mic"));
    assert_eq!(
        status_item.status_line(mtm).as_deref(),
        Some("Ready — hold Right Option to dictate")
    );
    drop(status_item);
    pump_run_loop(DRAW);
}

fn should_title_the_menu_with_the_name_and_version() {
    assert_eq!(
        title_line(),
        format!("voicepuncher {}", env!("CARGO_PKG_VERSION"))
    );
}

fn should_say_starting_before_the_runtime_reports() {
    assert_eq!(StatusItemState::default().status_line(), "Starting…");
}

fn should_create_a_missing_logs_folder() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let logs = home.path().join(".voicepuncher/logs");
    // When
    ensure_folder(&logs).unwrap();
    // Then
    assert!(logs.is_dir());
}

fn should_keep_an_existing_logs_folder_and_its_files() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let logs = home.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("whisper.log"), "kept").unwrap();
    // When
    ensure_folder(&logs).unwrap();
    // Then
    assert_eq!(
        std::fs::read_to_string(logs.join("whisper.log")).unwrap(),
        "kept"
    );
}

/// Opening a file or a folder shows a window: only with `VOICEPUNCHER_WINDOWS_IT` set.
fn windows_allowed() -> bool {
    if std::env::var("VOICEPUNCHER_WINDOWS_IT").is_err() {
        eprintln!("GATED: VOICEPUNCHER_WINDOWS_IT not set, skipping");
        return false;
    }
    true
}

fn should_show_the_logs_folder_in_the_file_browser() {
    if !windows_allowed() {
        return;
    }
    // Given
    let home = tempfile::tempdir().unwrap();
    let logs = home.path().join("logs");
    // When
    let shown = show_in_file_browser(&logs);
    // Then
    assert!(shown, "the file browser did not show {}", logs.display());
}

fn should_open_the_configuration_file_in_a_text_editor() {
    if !windows_allowed() {
        return;
    }
    // Given
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    std::fs::write(&config, "[hotkey]\nkey = \"F13\"\n").unwrap();
    // When
    let opened = open_in_text_editor(&config);
    // Then
    assert!(opened.is_ok(), "{opened:?}");
}
