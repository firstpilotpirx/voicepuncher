//! The login switch, the open window, the tabs and the models row.

use super::{
    main_thread, open_form, required_env, window_over, window_with_login_item, AppliedConfigs, Arc,
    Event, LoginItem, LoginItemStatus, ModelsPanel, NSAppearance, NSAppearanceCustomization,
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSBitmapImageFileType,
    NSDictionary, PathBuf, Rc, RememberedLoginItem, RIGHT_COMMAND, SNAPSHOT_DIR, UNAVAILABLE_NOTE,
};

pub(super) fn should_show_the_login_switch_disabled_outside_an_installed_app() {
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

pub(super) fn should_register_the_login_item_when_the_switch_turns_on_without_a_save() {
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

/// Showing a window takes the screen: only the it-real stage runs it (`needs_desktop`).
pub(super) fn should_bring_the_open_window_to_the_front_instead_of_a_second_one() {
    // Given
    required_env("VOICEPUNCHER_DESKTOP_IT");
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

/// Draws each tab into a picture `<tab>.png`, in `$VOICEPUNCHER_SETTINGS_SNAPSHOT` to compare
/// the window with the design, else in a temporary folder; every picture is written.
pub(super) fn should_draw_every_tab() {
    let scratch = tempfile::tempdir().unwrap();
    let dir =
        std::env::var_os(SNAPSHOT_DIR).map_or_else(|| scratch.path().to_owned(), PathBuf::from);
    let folder = tempfile::tempdir().unwrap();
    let (_window, form) = open_form(folder.path(), &AppliedConfigs::default());
    std::fs::create_dir_all(&dir).unwrap();
    // SAFETY: immutable framework constants, set before any Rust code runs.
    let appearance = std::env::var("VOICEPUNCHER_SETTINGS_SNAPSHOT_DARK")
        .map_or(unsafe { NSAppearanceNameAqua }, |_| unsafe {
            NSAppearanceNameDarkAqua
        });
    form.window
        .setAppearance(NSAppearance::appearanceNamed(appearance).as_deref());
    for (place, tab) in form.tabs.iter().enumerate() {
        form.select_tab(place);
        form.handle(Event::ToggleAdvanced(isize::try_from(place).unwrap()));
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
        let written = std::fs::metadata(&path).unwrap().len();
        assert!(written > 0, "{} is empty", path.display());
    }
    form.window.close();
}

/// Four of six model files present; counts the times the Download window was asked for.
#[derive(Default)]
pub(super) struct FourOfSix(std::cell::Cell<usize>);

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

pub(super) fn should_count_the_models_and_open_the_download_window_from_the_services_tab() {
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
