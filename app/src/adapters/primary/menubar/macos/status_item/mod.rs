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

mod icon;
use icon::{draw, red_dot};
mod menu;
use menu::{build_menu, MenuTarget};

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
