//! Integration tests for the settings window against AppKit: the system sounds it offers,
//! and the real window built over a configuration file in a temporary folder. AppKit builds
//! windows only on the main thread, and libtest runs each test on a thread of its own, so
//! this harness (`harness = false`) answers cargo-nextest's listing and runs each test on the
//! main thread.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplication, NSBitmapImageFileType, NSSound,
};
use objc2_foundation::{NSDictionary, NSString};
use toml::Value;
use voicepuncher::adapters::primary::settings::macos::field::Choices;
use voicepuncher::adapters::primary::settings::macos::form::Form;
use voicepuncher::adapters::primary::settings::macos::login::UNAVAILABLE_NOTE;
use voicepuncher::adapters::primary::settings::macos::models_row::ModelsPanel;
use voicepuncher::adapters::primary::settings::macos::session::{
    ApplyStatus, ConfigApplier, SessionParts, SettingsSession, StatusListener,
};
use voicepuncher::adapters::primary::settings::macos::table::{find, FIELDS};
use voicepuncher::adapters::primary::settings::macos::target::Event;
use voicepuncher::adapters::primary::settings::macos::values::options;
use voicepuncher::adapters::primary::settings::macos::window::SettingsWindow;
use voicepuncher::adapters::secondary::loginitem::macos::MacOsLoginItem;
use voicepuncher::config::Config;
use voicepuncher::core::application::{InputDevices, LoginItem, LoginItemStatus};

type Test = (&'static str, fn());

const TESTS: &[Test] = &[
    (
        "should_find_every_offered_sound_among_the_system_sounds",
        should_find_every_offered_sound_among_the_system_sounds,
    ),
    (
        "should_build_a_control_for_every_option_and_close",
        should_build_a_control_for_every_option_and_close,
    ),
    (
        "should_save_a_value_picked_in_the_window_to_the_file",
        should_save_a_value_picked_in_the_window_to_the_file,
    ),
    (
        "should_mark_a_refused_value_and_leave_the_file_unchanged",
        should_mark_a_refused_value_and_leave_the_file_unchanged,
    ),
    (
        "should_bring_the_open_window_to_the_front_instead_of_a_second_one",
        should_bring_the_open_window_to_the_front_instead_of_a_second_one,
    ),
    (
        "should_play_a_sound_without_changing_the_file",
        should_play_a_sound_without_changing_the_file,
    ),
    ("should_draw_every_tab", should_draw_every_tab),
    (
        "should_show_the_login_switch_disabled_outside_an_installed_app",
        should_show_the_login_switch_disabled_outside_an_installed_app,
    ),
    (
        "should_register_the_login_item_when_the_switch_turns_on_without_a_save",
        should_register_the_login_item_when_the_switch_turns_on_without_a_save,
    ),
    (
        "should_count_the_models_and_open_the_download_window_from_the_services_tab",
        should_count_the_models_and_open_the_download_window_from_the_services_tab,
    ),
];

/// The configuration file every window test starts from.
const RIGHT_COMMAND: &str = "# my hotkey\n[hotkey]\nkey = \"RightCommand\"\nmin_press_ms = 150\n";
/// Where `should_draw_every_tab` writes its pictures; unset, it draws nothing.
const SNAPSHOT_DIR: &str = "VOICEPUNCHER_SETTINGS_SNAPSHOT";

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

/// Counts what the window's saves told the app to apply.
#[derive(Clone, Default)]
struct AppliedConfigs(Arc<Mutex<Vec<Config>>>);

impl ConfigApplier for AppliedConfigs {
    fn apply(&self, config: Config, mut listener: StatusListener) {
        self.0.lock().unwrap().push(config);
        listener(ApplyStatus::Applied);
    }
}

struct NoMicrophones;

impl InputDevices for NoMicrophones {
    fn names(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A login item that only remembers whether it is registered.
#[derive(Default)]
struct RememberedLoginItem(Mutex<bool>);

impl LoginItem for RememberedLoginItem {
    fn status(&self) -> LoginItemStatus {
        if *self.0.lock().unwrap() {
            LoginItemStatus::Enabled
        } else {
            LoginItemStatus::NotRegistered
        }
    }

    fn register(&self) -> Result<(), String> {
        *self.0.lock().unwrap() = true;
        Ok(())
    }

    fn unregister(&self) -> Result<(), String> {
        *self.0.lock().unwrap() = false;
        Ok(())
    }
}

/// A settings window over `file` in a temporary folder.
fn window_over(folder: &Path, applied: &AppliedConfigs) -> SettingsWindow {
    window_with_login_item(folder, applied, Arc::new(MacOsLoginItem::of_this_process()))
}

fn window_with_login_item(
    folder: &Path,
    applied: &AppliedConfigs,
    login_item: Arc<dyn LoginItem>,
) -> SettingsWindow {
    let config = folder.join("config.toml");
    let home = folder.to_path_buf();
    let applied = applied.clone();
    let opener = move || {
        let parts = SessionParts {
            overlay: Box::new(|_: &mut Config| {}),
            applier: Box::new(applied.clone()),
            devices: Box::new(NoMicrophones),
        };
        SettingsSession::open(&config, &home, parts)
    };
    SettingsWindow::new(Box::new(opener), folder.join("logs"), login_item)
}

fn open_form(folder: &Path, applied: &AppliedConfigs) -> (SettingsWindow, Rc<Form>) {
    std::fs::write(folder.join("config.toml"), RIGHT_COMMAND).unwrap();
    let mtm = main_thread();
    let _app = NSApplication::sharedApplication(mtm);
    let window = window_over(folder, applied);
    let form = window.prepare(mtm).expect("the window builds");
    (window, form)
}

fn control_of<'a>(
    form: &'a Form,
    section: &str,
    key: &str,
) -> &'a voicepuncher::adapters::primary::settings::macos::control::FieldControl {
    let field = find(section, key).expect("the option has a field");
    form.controls
        .iter()
        .find(|control| control.field.path() == field.path())
        .expect("the field has a control")
}

fn should_find_every_offered_sound_among_the_system_sounds() {
    // Given
    let sounds = options(Choices::Sounds, &[]);
    // When
    let missing: Vec<String> = sounds
        .iter()
        .filter_map(|choice| choice.value.as_str())
        .filter(|name| !name.is_empty())
        .filter(|name| NSSound::soundNamed(&NSString::from_str(name)).is_none())
        .map(str::to_owned)
        .collect();
    // Then
    assert!(missing.is_empty(), "NSSound does not know {missing:?}");
}

fn should_build_a_control_for_every_option_and_close() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    // When
    let (_window, form) = open_form(folder.path(), &AppliedConfigs::default());
    // Then
    assert_eq!(form.controls.len(), FIELDS.len());
    assert_eq!(form.tabs.len(), 8);
    let hotkey = control_of(&form, "hotkey", "key").read();
    assert_eq!(hotkey, Value::String("RightCommand".to_owned()));
    form.window.close();
}

fn should_save_a_value_picked_in_the_window_to_the_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    let (_window, form) = open_form(folder.path(), &applied);
    // When
    control_of(&form, "hotkey", "key").show(&Value::String("F13".to_owned()));
    form.handle(Event::Save);
    // Then
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert!(file.contains("# my hotkey"), "{file}");
    assert!(file.contains("key = \"F13\""), "{file}");
    assert_eq!(applied.0.lock().unwrap().len(), 1);
    assert_eq!(form.status_line(), "Saved");
    form.window.close();
}

fn should_mark_a_refused_value_and_leave_the_file_unchanged() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    let (_window, form) = open_form(folder.path(), &applied);
    // When
    control_of(&form, "hotkey", "min_press_ms").show(&Value::Integer(5));
    form.handle(Event::Save);
    // Then
    assert_eq!(
        form.status_line(),
        "Minimum press must be between 50 and 1000 ms"
    );
    let marked = control_of(&form, "hotkey", "min_press_ms").outlined.layer();
    assert!(marked.is_some_and(|layer| layer.borderWidth() > 0.0));
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert_eq!(file, RIGHT_COMMAND);
    assert!(applied.0.lock().unwrap().is_empty());
    form.handle(Event::Revert);
    form.window.close();
}

fn should_play_a_sound_without_changing_the_file() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    let applied = AppliedConfigs::default();
    let (_window, form) = open_form(folder.path(), &applied);
    let start = find("sounds", "start_sound").expect("the start sound has a field");
    let tag = FIELDS
        .iter()
        .position(|field| field.path() == start.path())
        .unwrap();
    control_of(&form, "sounds", "start_sound").show(&Value::String("Ping".to_owned()));
    // When
    form.handle(Event::Play(tag as isize));
    // Then
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert_eq!(file, RIGHT_COMMAND);
    assert!(applied.0.lock().unwrap().is_empty());
    assert_eq!(form.status_line(), "");
    form.handle(Event::Revert);
    form.window.close();
}

fn should_show_the_login_switch_disabled_outside_an_installed_app() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    // When
    let (_window, form) = open_form(folder.path(), &AppliedConfigs::default());
    // Then
    assert!(!form.login.control.isEnabled());
    assert_eq!(form.login.note.stringValue().to_string(), UNAVAILABLE_NOTE);
    assert!(form.login.open_settings.isHidden());
    form.window.close();
}

fn should_register_the_login_item_when_the_switch_turns_on_without_a_save() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("config.toml"), RIGHT_COMMAND).unwrap();
    let applied = AppliedConfigs::default();
    let item = Arc::new(RememberedLoginItem::default());
    let mtm = main_thread();
    let _app = NSApplication::sharedApplication(mtm);
    let window = window_with_login_item(folder.path(), &applied, item.clone());
    let form = window.prepare(mtm).expect("the window builds");
    // When
    form.login
        .control
        .setState(objc2_app_kit::NSControlStateValueOn);
    form.handle(Event::LoginToggled);
    // Then
    assert_eq!(item.status(), LoginItemStatus::Enabled);
    assert!(form.login.is_on());
    let file = std::fs::read_to_string(folder.path().join("config.toml")).unwrap();
    assert_eq!(file, RIGHT_COMMAND);
    assert!(applied.0.lock().unwrap().is_empty());
    assert!(form.should_close(), "closing asked about unsaved edits");
    form.window.close();
}

/// Showing a window takes the screen: only with `VOICEPUNCHER_WINDOWS_IT` set.
fn windows_allowed() -> bool {
    if std::env::var("VOICEPUNCHER_WINDOWS_IT").is_err() {
        eprintln!("GATED: VOICEPUNCHER_WINDOWS_IT not set, skipping");
        return false;
    }
    true
}

fn should_bring_the_open_window_to_the_front_instead_of_a_second_one() {
    if !windows_allowed() {
        return;
    }
    // Given
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("config.toml"), RIGHT_COMMAND).unwrap();
    let mtm = main_thread();
    let window = window_over(folder.path(), &AppliedConfigs::default());
    window.show(mtm).unwrap();
    let first = window.form().unwrap();
    // When
    window.show(mtm).unwrap();
    // Then
    assert!(Rc::ptr_eq(&first, &window.form().unwrap()));
    assert!(first.window.isVisible());
    first.window.close();
}

/// Draws each tab into `$VOICEPUNCHER_SETTINGS_SNAPSHOT/<tab>.png`, to compare the window
/// with the design; does nothing without that folder.
fn should_draw_every_tab() {
    let Some(dir) = std::env::var_os(SNAPSHOT_DIR).map(PathBuf::from) else {
        eprintln!("GATED: {SNAPSHOT_DIR} not set, skipping");
        return;
    };
    let folder = tempfile::tempdir().unwrap();
    let (_window, form) = open_form(folder.path(), &AppliedConfigs::default());
    std::fs::create_dir_all(&dir).unwrap();
    let appearance = std::env::var("VOICEPUNCHER_SETTINGS_SNAPSHOT_DARK")
        .map_or(unsafe { NSAppearanceNameAqua }, |_| unsafe {
            NSAppearanceNameDarkAqua
        });
    form.window
        .setAppearance(NSAppearance::appearanceNamed(appearance).as_deref());
    for (place, tab) in form.tabs.iter().enumerate() {
        form.select_tab(place);
        form.handle(Event::ToggleAdvanced(place as isize));
        let content = form
            .window
            .contentView()
            .expect("the window has a content view");
        // SAFETY: the content view is in the window, under its frame view.
        let view = unsafe { content.superview() }.unwrap_or(content);
        let bounds = view.bounds();
        let picture = view.bitmapImageRepForCachingDisplayInRect(bounds).unwrap();
        view.cacheDisplayInRect_toBitmapImageRep(bounds, &picture);
        // SAFETY: an empty property dictionary is a valid argument.
        let png = unsafe {
            picture.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        }
        .unwrap();
        let path = dir.join(format!("{}.png", tab.tab.title().to_lowercase()));
        std::fs::write(&path, png.to_vec()).unwrap();
    }
    form.window.close();
}

/// Four of six model files present; counts the times the Download window was asked for.
#[derive(Default)]
struct FourOfSix(std::cell::Cell<usize>);

impl ModelsPanel for FourOfSix {
    fn count(&self) -> (usize, usize) {
        (4, 6)
    }

    fn folder(&self) -> PathBuf {
        PathBuf::from("/nowhere")
    }

    fn open_download(&self) {
        self.0.set(self.0.get() + 1);
    }
}

fn should_count_the_models_and_open_the_download_window_from_the_services_tab() {
    // Given
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("config.toml"), RIGHT_COMMAND).unwrap();
    let mtm = main_thread();
    let _app = NSApplication::sharedApplication(mtm);
    let panel = Rc::new(FourOfSix::default());
    let window = window_over(folder.path(), &AppliedConfigs::default()).with_models(panel.clone());
    let form = window.prepare(mtm).expect("the window builds");
    // When
    form.handle(Event::DownloadModels);
    // Then
    assert_eq!(
        form.models.line.stringValue().to_string(),
        "4 of 6 present — 2 missing"
    );
    assert!(!form.models.view.isHidden());
    assert_eq!(panel.0.get(), 1);
    form.window.close();
}
