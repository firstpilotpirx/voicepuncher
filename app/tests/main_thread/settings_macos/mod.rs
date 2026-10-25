//! Integration tests for the settings window against `AppKit`: the system sounds it offers,
//! and the real window built over a configuration file in a temporary folder. `AppKit` builds
//! windows only on the main thread, and libtest runs each test on a thread of its own, so
//! this harness (`harness = false`) answers cargo-nextest's listing and runs each test on the
//! main thread.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplication, NSBitmapImageFileType, NSSound,
};
use objc2_foundation::{NSDictionary, NSString};
use toml::Value;
use voicepuncher::adapters::primary::settings::macos::control::FieldControl;
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

use voicepuncher_test_steps::required_env;

use crate::{main_thread, Test};

mod saving;
use saving::{
    should_build_a_control_for_every_option_and_close,
    should_find_every_offered_sound_among_the_system_sounds,
    should_mark_a_refused_value_and_leave_the_file_unchanged,
    should_play_a_sound_without_changing_the_file,
    should_save_a_value_picked_in_the_window_to_the_file,
};
mod login_and_tabs;
use login_and_tabs::{
    should_bring_the_open_window_to_the_front_instead_of_a_second_one,
    should_count_the_models_and_open_the_download_window_from_the_services_tab,
    should_draw_every_tab, should_register_the_login_item_when_the_switch_turns_on_without_a_save,
    should_show_the_login_switch_disabled_outside_an_installed_app,
};

pub const TESTS: &[Test] = &[
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
        "needs_desktop::should_bring_the_open_window_to_the_front_instead_of_a_second_one",
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
/// Where `should_draw_every_tab` writes its pictures; unset, a temporary folder.
const SNAPSHOT_DIR: &str = "VOICEPUNCHER_SETTINGS_SNAPSHOT";

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

fn control_of<'a>(form: &'a Form, section: &str, key: &str) -> &'a FieldControl {
    let field = find(section, key).expect("the option has a field");
    form.controls
        .iter()
        .find(|control| control.field.path() == field.path())
        .expect("the field has a control")
}
