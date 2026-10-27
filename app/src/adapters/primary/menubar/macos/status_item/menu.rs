//! The menu and the target of its items.

use super::{
    define_class, msg_send, offers_restart, sel, service_line, AnyObject, DefinedClass,
    MainThreadMarker, MainThreadOnly, MenuActions, MenuEntry, NSMenu, NSMenuDelegate, NSMenuItem,
    NSObject, NSObjectProtocol, NSString, OnceCell, ProtocolObject, RefCell, Retained, Sel,
    StatusItemCell, MENU, NO_KEY, OPEN_SERVICE_LOG, QUIT_KEY, RESTART_SERVICE, SETTINGS_KEY,
};

pub(super) fn build_menu(mtm: MainThreadMarker, target: &MenuTarget) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    menu.setAutoenablesItems(false);
    menu.setDelegate(Some(ProtocolObject::from_ref(target)));
    for entry in MENU {
        menu.addItem(&entry_item(mtm, target, entry));
    }
    menu
}

fn entry_item(
    mtm: MainThreadMarker,
    target: &MenuTarget,
    entry: MenuEntry,
) -> Retained<NSMenuItem> {
    match entry {
        MenuEntry::TitleLine | MenuEntry::StatusLine => text_line(mtm, target),
        MenuEntry::Separator => NSMenuItem::separatorItem(mtm),
        MenuEntry::DownloadModels => {
            let item = chosen(mtm, target, entry, sel!(downloadModels:), NO_KEY);
            item.setHidden(true);
            item
        }
        MenuEntry::Services => services_item(mtm, entry),
        _ => {
            let (action, key) = action_of(entry);
            chosen(mtm, target, entry, action, key)
        }
    }
}

/// The action and key of an item the user chooses.
fn action_of(entry: MenuEntry) -> (Sel, &'static str) {
    match entry {
        MenuEntry::History => (sel!(history:), NO_KEY),
        MenuEntry::Settings => (sel!(settings:), SETTINGS_KEY),
        MenuEntry::Logs => (sel!(openLogs:), NO_KEY),
        MenuEntry::Quit => (sel!(quit:), QUIT_KEY),
        _ => (sel!(permissions:), NO_KEY),
    }
}

pub(super) fn chosen(
    mtm: MainThreadMarker,
    target: &MenuTarget,
    entry: MenuEntry,
    action: Sel,
    key: &str,
) -> Retained<NSMenuItem> {
    action_item(mtm, target, entry.title().unwrap_or_default(), action, key)
}

/// The Services item with an empty submenu, filled each time the menu opens.
pub(super) fn services_item(mtm: MainThreadMarker, entry: MenuEntry) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(entry.title().unwrap_or_default()));
    let submenu = NSMenu::new(mtm);
    submenu.setAutoenablesItems(false);
    item.setSubmenu(Some(&submenu));
    item.setHidden(true);
    item
}

/// A line of text, chosen only while `refresh` enables it: then it opens Permissions.
pub(super) fn text_line(mtm: MainThreadMarker, target: &MenuTarget) -> Retained<NSMenuItem> {
    let item = action_item(mtm, target, "", sel!(permissions:), NO_KEY);
    item.setEnabled(false);
    item
}

pub(super) fn action_item(
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
pub(super) fn port_of(item: &NSMenuItem) -> Option<u16> {
    u16::try_from(item.tag()).ok()
}

pub struct MenuTargetIvars {
    pub(crate) cell: OnceCell<StatusItemCell>,
    pub(crate) actions: RefCell<Option<Box<dyn MenuActions>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPStatusMenuTarget"]
    #[ivars = MenuTargetIvars]
    pub(super) struct MenuTarget;

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

        #[unsafe(method(history:))]
        fn history(&self, _sender: Option<&AnyObject>) {
            self.act(|actions| actions.open_history());
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
    pub(crate) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm)
            .set_ivars(MenuTargetIvars { cell: OnceCell::new(), actions: RefCell::new(None) });
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }

    /// Sets the title and the status line from the state at this moment.
    pub(crate) fn refresh(&self, menu: &NSMenu) {
        let Some(cell) = self.ivars().cell.get() else {
            return;
        };
        let model = cell.menu();
        if let Some(item) = menu.itemAtIndex(MenuEntry::TitleLine.place()) {
            item.setTitle(&NSString::from_str(&model.title_line));
        }
        if let Some(item) = menu.itemAtIndex(MenuEntry::StatusLine.place()) {
            item.setTitle(&NSString::from_str(&model.status_line));
            item.setEnabled(model.status_opens_permissions);
        }
        if let Some(item) = menu.itemAtIndex(MenuEntry::DownloadModels.place()) {
            let actions = self.ivars().actions.borrow();
            item.setHidden(actions.as_deref().is_none_or(|a| a.services().is_none()));
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
            .and_then(super::super::actions::MenuActions::services);
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
