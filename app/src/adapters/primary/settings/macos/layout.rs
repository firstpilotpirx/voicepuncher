//! The window's views: a toolbar tab per [`Tab`], a grid of labelled rows on each, and the
//! bottom bar with the status line, Revert and Save.

use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{
    NSBezelStyle, NSButtonType, NSColor, NSFont, NSGridCellPlacement, NSGridRow,
    NSGridRowAlignment, NSGridView, NSImage, NSLayoutAttribute, NSStackView, NSTabViewController,
    NSTabViewControllerTabStyle, NSTabViewItem, NSTextAlignment, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView, NSViewController,
};
use objc2_foundation::{NSArray, NSEdgeInsets, NSString};

use super::build::{button, stack, width, Parts};
use super::control::FieldControl;
use super::field::Tab;
use super::login::SWITCH_LABEL;
use super::login_row::LoginRow;
use super::models_row::{ModelsRow, MODELS_LABEL};
use super::target::actions;

const MARGIN: f64 = 20.0;
const LABEL_COLUMN: f64 = 160.0;
const ROW_SPACING: f64 = 10.0;
const COLUMN_SPACING: f64 = 8.0;
const CONTENT_WIDTH: f64 = 560.0;
const NOTE_SIZE: f64 = 11.0;
const ADVANCED_TITLE: &str = "Show advanced";
const LOGS_TITLE: &str = "Open Logs Folder";
const SAVE_TITLE: &str = "Save";
const REVERT_TITLE: &str = "Revert";
const RETURN_KEY: &str = "\r";
const LABEL_SUFFIX: &str = ":";

/// What one tab shows besides its controls.
pub struct TabViews {
    pub tab: Tab,
    pub page: Retained<NSViewController>,
    pub status: Retained<NSTextField>,
    pub advanced: Vec<Retained<NSGridRow>>,
    pub notes: Vec<Retained<NSTextField>>,
}

impl TabViews {
    /// Sizes the tab to its rows, the shown ones only; the window follows the selected tab.
    pub fn fit(&self) {
        let view = self.page.view();
        view.layoutSubtreeIfNeeded();
        let size = view.fittingSize();
        self.page.setPreferredContentSize(size);
        if let Some(window) = view.window() {
            window.setContentSize(size);
        }
    }
}

/// The tab view controller with every tab, and what each tab shows besides its controls.
pub fn tabs(
    parts: &Parts,
    controls: &[FieldControl],
    notes: &[(usize, String)],
    login: &LoginRow,
    models: &ModelsRow,
) -> (Retained<NSTabViewController>, Vec<TabViews>) {
    let controller = NSTabViewController::new(parts.mtm);
    controller.setTabStyle(NSTabViewControllerTabStyle::Toolbar);
    controller.setCanPropagateSelectedChildViewControllerTitle(false);
    let views = Tab::ALL
        .iter()
        .enumerate()
        .map(|(place, tab)| {
            let page = NSViewController::new(parts.mtm);
            let (view, views) =
                tab_view(parts, &page, place as isize, controls, notes, login, models);
            page.setView(&view);
            views.fit();
            let item = NSTabViewItem::tabViewItemWithViewController(&page);
            item.setLabel(&NSString::from_str(tab.title()));
            item.setImage(symbol(tab.symbol()).as_deref());
            controller.addTabViewItem(&item);
            views
        })
        .collect();
    (controller, views)
}

fn symbol(name: &str) -> Option<Retained<NSImage>> {
    NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "size baseline"
)]
fn tab_view(
    parts: &Parts,
    page: &NSViewController,
    place: isize,
    controls: &[FieldControl],
    notes: &[(usize, String)],
    login: &LoginRow,
    models: &ModelsRow,
) -> (Retained<NSView>, TabViews) {
    let tab = Tab::ALL[place as usize];
    let grid = NSGridView::new(parts.mtm);
    grid.setRowSpacing(ROW_SPACING);
    grid.setColumnSpacing(COLUMN_SPACING);
    grid.setRowAlignment(NSGridRowAlignment::FirstBaseline);
    let mut views = TabViews {
        tab,
        page: page.retain(),
        status: NSTextField::labelWithString(&NSString::new(), parts.mtm),
        advanced: Vec::new(),
        notes: Vec::new(),
    };
    add_rows(parts, &grid, &mut views, controls, notes, false);
    if tab == Tab::General {
        add_row(parts.mtm, &grid, Some(SWITCH_LABEL), &login.view);
        add_row(parts.mtm, &grid, None, &login.note);
    }
    if tab == Tab::Services && models.panel.is_some() {
        add_row(parts.mtm, &grid, Some(MODELS_LABEL), &models.view);
    }
    if controls
        .iter()
        .any(|c| c.field.tab == tab && c.field.advanced)
    {
        let disclosure = disclosure(parts, place);
        add_row(parts.mtm, &grid, None, &disclosure);
        add_rows(parts, &grid, &mut views, controls, notes, true);
    }
    if tab == Tab::Logging {
        add_row(
            parts.mtm,
            &grid,
            None,
            &button(parts, LOGS_TITLE, actions::open_logs(), 0),
        );
    }
    let labels = grid.columnAtIndex(0);
    labels.setXPlacement(NSGridCellPlacement::Trailing);
    labels.setWidth(LABEL_COLUMN);
    let page = stack(
        parts.mtm,
        &[grid.as_ref(), bottom_bar(parts, &views.status).as_ref()],
    );
    page.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    page.setAlignment(NSLayoutAttribute::Leading);
    page.setSpacing(MARGIN);
    page.setEdgeInsets(NSEdgeInsets { top: MARGIN, left: MARGIN, bottom: MARGIN, right: MARGIN });
    width(&page, CONTENT_WIDTH);
    (Retained::into_super(page), views)
}

/// The rows of `views.tab`'s fields, the advanced ones hidden, each with its note under it.
#[expect(clippy::too_many_arguments, reason = "size baseline")]
fn add_rows(
    parts: &Parts,
    grid: &NSGridView,
    views: &mut TabViews,
    controls: &[FieldControl],
    notes: &[(usize, String)],
    advanced: bool,
) {
    let fields = controls.iter().enumerate();
    for (index, control) in fields.filter(|(_, c)| c.field.tab == views.tab) {
        if control.field.advanced != advanced {
            continue;
        }
        let label = format!("{}{LABEL_SUFFIX}", control.field.label);
        let mut rows = vec![add_row(parts.mtm, grid, Some(&label), &control.view)];
        for (_, note) in notes.iter().filter(|(field, _)| *field == index) {
            let note = note_label(parts.mtm, note);
            rows.push(add_row(parts.mtm, grid, None, &note));
            views.notes.push(note);
        }
        if advanced {
            rows.iter().for_each(|row| row.setHidden(true));
            views.advanced.extend(rows);
        }
    }
}

fn add_row(
    mtm: MainThreadMarker,
    grid: &NSGridView,
    label: Option<&str>,
    control: &NSView,
) -> Retained<NSGridRow> {
    let label =
        NSTextField::wrappingLabelWithString(&NSString::from_str(label.unwrap_or_default()), mtm);
    label.setAlignment(NSTextAlignment::Right);
    label.setPreferredMaxLayoutWidth(LABEL_COLUMN);
    let cells: [Retained<NSView>; 2] = [
        Retained::into_super(Retained::into_super(label)),
        control.retain(),
    ];
    grid.addRowWithViews(&NSArray::from_retained_slice(&cells))
}

fn note_label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let note = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    note.setFont(Some(&NSFont::systemFontOfSize(NOTE_SIZE)));
    note.setTextColor(Some(&NSColor::secondaryLabelColor()));
    note
}

/// The triangle that shows the tab's advanced rows, with its title beside it.
fn disclosure(parts: &Parts, place: isize) -> Retained<NSStackView> {
    let triangle = button(parts, "", actions::toggle_advanced(), place);
    triangle.setButtonType(NSButtonType::PushOnPushOff);
    triangle.setBezelStyle(NSBezelStyle::Disclosure);
    let title = NSTextField::labelWithString(&NSString::from_str(ADVANCED_TITLE), parts.mtm);
    stack(parts.mtm, &[triangle.as_ref(), title.as_ref()])
}

fn bottom_bar(parts: &Parts, status: &NSTextField) -> Retained<NSStackView> {
    status.setTextColor(Some(&NSColor::secondaryLabelColor()));
    let revert = button(parts, REVERT_TITLE, actions::revert(), 0);
    let save = button(parts, SAVE_TITLE, actions::save(), 0);
    save.setKeyEquivalent(&NSString::from_str(RETURN_KEY));
    let bar = stack(parts.mtm, &[status.as_ref()]);
    bar.addView_inGravity(&revert, objc2_app_kit::NSStackViewGravity::Trailing);
    bar.addView_inGravity(&save, objc2_app_kit::NSStackViewGravity::Trailing);
    width(&bar, CONTENT_WIDTH - 2.0 * MARGIN);
    bar
}
