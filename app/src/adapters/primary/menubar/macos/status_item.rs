//! The status item in the macOS menu bar: the icon the state cell picks, and the menu with the
//! title line, the status line, Permissions…, Settings, Open Logs Folder and Quit.

use std::cell::{OnceCell, RefCell};
use std::sync::atomic::{AtomicU64, Ordering};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSColor, NSFontWeightRegular, NSImage, NSImageSymbolConfiguration, NSMenu, NSMenuDelegate,
    NSMenuItem, NSSquareStatusItemLength, NSStatusBar, NSStatusBarButton, NSStatusItem,
    NSWorkspace,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};
use objc2_quartz_core::CALayer;

use crate::adapters::main_queue::{dispatch_to_main, dispatch_to_main_after};

use super::actions::MenuActions;
use super::indicator::{StatusItemCell, StatusItemSurface};
use super::mapping::{
    offers_restart, service_line, IconPicture, IconState, MenuEntry, FALLBACK_SYMBOL, MENU,
    OPEN_SERVICE_LOG, PROCESSING_FRAMES, RESTART_SERVICE, STILL_PROCESSING_FRAME,
};

/// The symbols' point size in the menu bar.
const POINT_SIZE: f64 = 16.0;
/// One frame of the processing animation.
const FRAME_MS: i64 = 400;
/// The red dot of the recording icon and its distance from the button's lower right corner.
const DOT_DIAMETER: f64 = 6.0;
const DOT_INSET: f64 = 2.0;
const SETTINGS_KEY: &str = ",";
const QUIT_KEY: &str = "q";
const NO_KEY: &str = "";

thread_local! {
    /// The drawn button, reached by the work the icon dispatches to the main queue.
    static BUTTON: RefCell<Option<Button>> = const { RefCell::new(None) };
}

/// Counts the icons asked for, so a processing frame stops once another icon is drawn.
static ICONS: AtomicU64 = AtomicU64::new(0);

struct Button {
    view: Retained<NSStatusBarButton>,
    dot: Retained<CALayer>,
}

/// The status item and its menu; the item leaves the menu bar when this is dropped.
pub struct MacosStatusItem {
    item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    target: Retained<MenuTarget>,
}

impl MacosStatusItem {
    /// Puts an empty status item with its menu in the menu bar; [`MacosStatusItem::icon`]
    /// draws on it, [`MacosStatusItem::connect`] gives the menu its words.
    pub fn install(mtm: MainThreadMarker) -> Self {
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSSquareStatusItemLength);
        if let Some(view) = item.button(mtm) {
            view.setWantsLayer(true);
            let dot = red_dot();
            if let Some(layer) = view.layer() {
                layer.addSublayer(&dot);
            }
            BUTTON.set(Some(Button { view, dot }));
        }
        let target = MenuTarget::new(mtm);
        let menu = build_menu(mtm, &target);
        item.setMenu(Some(&menu));
        Self { item, menu, target }
    }

    /// Draws the icons the state cell picks, from any thread.
    pub fn icon(&self) -> MenuBarIcon {
        MenuBarIcon
    }

    /// The cell the menu reads its status line from when it opens.
    pub fn connect(&self, cell: StatusItemCell) {
        let _ = self.target.ivars().cell.set(cell);
    }

    /// What Settings, Open Logs Folder and Quit do.
    pub fn set_actions(&self, actions: Box<dyn MenuActions>) {
        self.target.ivars().actions.replace(Some(actions));
    }

    /// The name of the symbol the icon shows.
    pub fn shown_symbol(&self, mtm: MainThreadMarker) -> Option<String> {
        let image = self.item.button(mtm)?.image()?;
        image
            .accessibilityDescription()
            .map(|name| name.to_string())
    }

    /// The status line as the menu reads when it opens.
    pub fn status_line(&self, _mtm: MainThreadMarker) -> Option<String> {
        self.target.refresh(&self.menu);
        let line = self.menu.itemAtIndex(MenuEntry::StatusLine.place())?;
        Some(line.title().to_string())
    }
}

impl Drop for MacosStatusItem {
    fn drop(&mut self) {
        ICONS.fetch_add(1, Ordering::SeqCst);
        BUTTON.set(None);
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}

/// Draws an icon on the status item: dispatched to the main queue, in the order asked.
pub struct MenuBarIcon;

impl StatusItemSurface for MenuBarIcon {
    fn show_icon(&self, icon: IconState) {
        let generation = ICONS.fetch_add(1, Ordering::SeqCst) + 1;
        dispatch_to_main(move || draw(icon.picture(), generation));
    }
}

fn draw(picture: IconPicture, generation: u64) {
    let still = reduce_motion();
    let frame = picture.animated.then_some(if still {
        STILL_PROCESSING_FRAME
    } else {
        PROCESSING_FRAMES[0]
    });
    BUTTON.with_borrow(|button| {
        let Some(button) = button else { return };
        button.view.setImage(icon_image(picture, frame).as_deref());
        place_dot(button);
        button.dot.setHidden(!picture.red_dot);
    });
    if picture.animated && !still {
        next_frame(picture, generation, 1);
    }
}

/// Steps the processing animation until another icon is drawn.
fn next_frame(picture: IconPicture, generation: u64, frame: usize) {
    dispatch_to_main_after(FRAME_MS, move || {
        if ICONS.load(Ordering::SeqCst) != generation {
            return;
        }
        let value = PROCESSING_FRAMES[frame % PROCESSING_FRAMES.len()];
        BUTTON.with_borrow(|button| {
            if let Some(button) = button {
                button
                    .view
                    .setImage(icon_image(picture, Some(value)).as_deref());
            }
        });
        next_frame(picture, generation, frame + 1);
    });
}

fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// The symbol of `picture` at `frame`, or the fallback microphone when the system lacks it;
/// a template image follows the menu bar's ink, the others are drawn in the label colour.
fn icon_image(picture: IconPicture, frame: Option<f64>) -> Option<Retained<NSImage>> {
    let image = symbol(picture.symbol, frame).or_else(|| symbol(FALLBACK_SYMBOL, None))?;
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    let weight = unsafe { NSFontWeightRegular };
    let mut config =
        NSImageSymbolConfiguration::configurationWithPointSize_weight(POINT_SIZE, weight);
    if !picture.template {
        let ink = NSColor::labelColor();
        let palette =
            NSImageSymbolConfiguration::configurationWithPaletteColors(&NSArray::from_slice(&[
                &*ink,
            ]));
        config = config.configurationByApplyingConfiguration(&palette);
    }
    let image = image.imageWithSymbolConfiguration(&config)?;
    image.setTemplate(picture.template);
    Some(image)
}

/// The SF Symbol `name`, at the variable value `frame` when given; its accessibility
/// description is its name, so a test can tell which symbol shows.
fn symbol(name: &str, frame: Option<f64>) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    match frame {
        Some(value) => NSImage::imageWithSystemSymbolName_variableValue_accessibilityDescription(
            &name,
            value,
            Some(&name),
        ),
        None => NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name)),
    }
}

fn red_dot() -> Retained<CALayer> {
    let dot = CALayer::new();
    dot.setBackgroundColor(Some(&NSColor::systemRedColor().CGColor()));
    dot.setCornerRadius(DOT_DIAMETER / 2.0);
    dot.setHidden(true);
    dot
}

/// The red dot at the lower right corner of the button, which may be flipped.
fn place_dot(button: &Button) {
    let bounds = button.view.bounds();
    let x = bounds.size.width - DOT_DIAMETER - DOT_INSET;
    let y = if button.view.isFlipped() {
        bounds.size.height - DOT_DIAMETER - DOT_INSET
    } else {
        DOT_INSET
    };
    button.dot.setFrame(CGRect::new(
        CGPoint::new(x, y),
        CGSize::new(DOT_DIAMETER, DOT_DIAMETER),
    ));
}

fn build_menu(mtm: MainThreadMarker, target: &MenuTarget) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    menu.setAutoenablesItems(false);
    menu.setDelegate(Some(ProtocolObject::from_ref(target)));
    for entry in MENU {
        let item = match entry {
            MenuEntry::TitleLine | MenuEntry::StatusLine => text_line(mtm),
            MenuEntry::Separator => NSMenuItem::separatorItem(mtm),
            MenuEntry::Permissions => chosen(mtm, target, entry, sel!(permissions:), NO_KEY),
            MenuEntry::DownloadModels => {
                let item = chosen(mtm, target, entry, sel!(downloadModels:), NO_KEY);
                item.setHidden(true);
                item
            }
            MenuEntry::Services => services_item(mtm, entry),
            MenuEntry::Settings => chosen(mtm, target, entry, sel!(settings:), SETTINGS_KEY),
            MenuEntry::Logs => chosen(mtm, target, entry, sel!(openLogs:), NO_KEY),
            MenuEntry::Quit => chosen(mtm, target, entry, sel!(quit:), QUIT_KEY),
        };
        menu.addItem(&item);
    }
    menu
}

fn chosen(
    mtm: MainThreadMarker,
    target: &MenuTarget,
    entry: MenuEntry,
    action: Sel,
    key: &str,
) -> Retained<NSMenuItem> {
    action_item(mtm, target, entry.title().unwrap_or_default(), action, key)
}

/// The Services item with an empty submenu, filled each time the menu opens.
fn services_item(mtm: MainThreadMarker, entry: MenuEntry) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(entry.title().unwrap_or_default()));
    let submenu = NSMenu::new(mtm);
    submenu.setAutoenablesItems(false);
    item.setSubmenu(Some(&submenu));
    item.setHidden(true);
    item
}

/// A line of text that cannot be chosen.
fn text_line(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setEnabled(false);
    item
}

fn action_item(
    mtm: MainThreadMarker,
    target: &MenuTarget,
    title: &str,
    action: Sel,
    key: &str,
) -> Retained<NSMenuItem> {
    // SAFETY: `action` is a method of `target` taking the sender, as a menu item calls it.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    };
    // SAFETY: the target lives as long as the status item that owns the menu.
    unsafe { item.setTarget(Some(target as &AnyObject)) };
    item
}

/// The port a service's menu item carries in its tag.
fn port_of(item: &NSMenuItem) -> Option<u16> {
    u16::try_from(item.tag()).ok()
}

pub struct MenuTargetIvars {
    cell: OnceCell<StatusItemCell>,
    actions: RefCell<Option<Box<dyn MenuActions>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPStatusMenuTarget"]
    #[ivars = MenuTargetIvars]
    struct MenuTarget;

    unsafe impl NSObjectProtocol for MenuTarget {}

    unsafe impl NSMenuDelegate for MenuTarget {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            self.refresh(menu);
        }
    }

    impl MenuTarget {
        #[unsafe(method(permissions:))]
        fn permissions(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.open_permissions());
        }

        #[unsafe(method(downloadModels:))]
        fn download_models(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.open_download_models());
        }

        #[unsafe(method(settings:))]
        fn settings(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.open_settings());
        }

        #[unsafe(method(openLogs:))]
        fn open_logs(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.open_logs());
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.quit());
        }

        #[unsafe(method(restartService:))]
        fn restart_service(&self, sender: Option<&NSMenuItem>) {
            if let Some(port) = sender.and_then(port_of) {
                self.act(|actions| actions.restart_service(port));
            }
        }

        #[unsafe(method(openServiceLog:))]
        fn open_service_log(&self, sender: Option<&NSMenuItem>) {
            if let Some(port) = sender.and_then(port_of) {
                self.act(|actions| actions.open_service_log(port));
            }
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm)
            .set_ivars(MenuTargetIvars { cell: OnceCell::new(), actions: RefCell::new(None) });
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }

    /// Sets the title and the status line from the state at this moment.
    fn refresh(&self, menu: &NSMenu) {
        let Some(cell) = self.ivars().cell.get() else {
            return;
        };
        let model = cell.menu();
        if let Some(item) = menu.itemAtIndex(MenuEntry::TitleLine.place()) {
            item.setTitle(&NSString::from_str(&model.title_line));
        }
        if let Some(item) = menu.itemAtIndex(MenuEntry::StatusLine.place()) {
            item.setTitle(&NSString::from_str(&model.status_line));
        }
        if let Some(item) = menu.itemAtIndex(MenuEntry::DownloadModels.place()) {
            let managed = self
                .ivars()
                .actions
                .borrow()
                .as_deref()
                .is_some_and(|actions| actions.services().is_some());
            item.setHidden(!managed);
        }
        if let Some(item) = menu.itemAtIndex(MenuEntry::Services.place()) {
            self.refresh_services(&item);
        }
    }

    /// Lists every service with its state, each with Restart (for one the app started) and
    /// Open Log; hidden when the app leaves the services to the scripts.
    fn refresh_services(&self, item: &NSMenuItem) {
        let mtm = self.mtm();
        let statuses = self
            .ivars()
            .actions
            .borrow()
            .as_deref()
            .and_then(super::actions::MenuActions::services);
        item.setHidden(statuses.is_none());
        let Some(submenu) = item.submenu() else {
            return;
        };
        submenu.removeAllItems();
        for status in statuses.unwrap_or_default() {
            let line = NSMenuItem::new(mtm);
            line.setTitle(&NSString::from_str(&service_line(&status)));
            let choices = NSMenu::new(mtm);
            choices.setAutoenablesItems(false);
            if offers_restart(&status) {
                choices.addItem(&self.service_choice(
                    RESTART_SERVICE,
                    sel!(restartService:),
                    status.port,
                ));
            }
            choices.addItem(&self.service_choice(
                OPEN_SERVICE_LOG,
                sel!(openServiceLog:),
                status.port,
            ));
            line.setSubmenu(Some(&choices));
            submenu.addItem(&line);
        }
    }

    fn service_choice(&self, title: &str, action: Sel, port: u16) -> Retained<NSMenuItem> {
        let item = action_item(self.mtm(), self, title, action, NO_KEY);
        item.setTag(isize::try_from(port).unwrap_or_default());
        item
    }

    fn mtm(&self) -> MainThreadMarker {
        MainThreadMarker::from(self)
    }

    fn act(&self, action: impl FnOnce(&dyn MenuActions)) {
        match self.ivars().actions.borrow().as_deref() {
            Some(actions) => action(actions),
            None => log::warn!("menu: chosen before the app is ready"),
        }
    }
}
