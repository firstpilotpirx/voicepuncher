//! The content of the permissions window: its rows, badges and buttons.

use super::{
    appkit, badge, below_top, button, label, mapping, sel, symbol, wrapping, BadgeMark,
    MainThreadMarker, NSButton, NSColor, NSFont, NSFontWeightMedium, NSImage,
    NSImageSymbolConfiguration, NSImageView, NSPoint, NSProgressIndicator,
    NSProgressIndicatorStyle, NSRect, NSSize, NSString, NSView, Permission, Retained, RowButton,
    RowModel, Sel, Shown, WindowModel, WindowTarget, CONTINUE_TITLE, FOOTER_BUTTON_WIDTH, HEIGHT,
    MARGIN, QUIT_TITLE, ROW_GAP, ROW_HEIGHT, STATUS_WIDTH, SYMBOL_SIZE, TILE, WIDTH,
};
use crate::core::domain::conversions::index_isize;

pub(super) fn content(
    mtm: MainThreadMarker,
    target: &WindowTarget,
    model: &WindowModel,
) -> Retained<NSView> {
    let view = crate::adapters::appkit::content_view(mtm, WIDTH, HEIGHT);
    let rows_top = add_heading(mtm, &view, model);
    for (index, row) in model.rows.iter().enumerate() {
        let top = rows_top + index as f64 * (ROW_HEIGHT + ROW_GAP);
        add_row(mtm, &view, target, row, top);
    }
    let quit = push_button(mtm, target, QUIT_TITLE, sel!(quit:), 0);
    quit.setFrame(below_top(MARGIN, HEIGHT - 42.0, FOOTER_BUTTON_WIDTH, 28.0));
    view.addSubview(&quit);
    let go_on = push_button(mtm, target, CONTINUE_TITLE, sel!(goOn:), 0);
    go_on.setKeyEquivalent(&NSString::from_str("\r"));
    go_on.setEnabled(model.continue_enabled);
    go_on.setFrame(below_top(
        WIDTH - MARGIN - FOOTER_BUTTON_WIDTH,
        HEIGHT - 42.0,
        FOOTER_BUTTON_WIDTH,
        28.0,
    ));
    view.addSubview(&go_on);
    view
}

/// The heading, the subheading and a revoked or developer note; returns where the rows begin.
fn add_heading(mtm: MainThreadMarker, view: &NSView, model: &WindowModel) -> f64 {
    let text_width = WIDTH - 2.0 * MARGIN;
    let heading = label(mtm, mapping::HEADING, &NSFont::boldSystemFontOfSize(15.0));
    heading.setFrame(below_top(MARGIN, 14.0, text_width, 20.0));
    view.addSubview(&heading);
    let sub = wrapping(mtm, mapping::SUBHEADING, &NSFont::systemFontOfSize(13.0));
    sub.setTextColor(Some(&NSColor::secondaryLabelColor()));
    sub.setFrame(below_top(MARGIN, 36.0, text_width, 34.0));
    view.addSubview(&sub);
    let note = model
        .revoked_line
        .as_deref()
        .map(|line| (line, NSColor::systemRedColor()))
        .or_else(|| {
            model
                .developer_line
                .as_deref()
                .map(|line| (line, NSColor::systemOrangeColor()))
        });
    let Some((line, colour)) = note else {
        return 80.0;
    };
    let note = label(mtm, line, &medium(13.0));
    note.setTextColor(Some(&colour));
    note.setFrame(below_top(MARGIN, 74.0, text_width, 18.0));
    view.addSubview(&note);
    100.0
}

pub(super) fn add_row(
    mtm: MainThreadMarker,
    view: &NSView,
    target: &WindowTarget,
    row: &RowModel,
    top: f64,
) {
    add_tile(mtm, view, row.permission, top);
    let text_x = MARGIN + TILE + 10.0;
    let text_width = WIDTH - MARGIN - STATUS_WIDTH - 10.0 - text_x;
    let name = label(mtm, row.permission.name(), &medium(13.0));
    name.setFrame(below_top(text_x, top + 2.0, text_width, 17.0));
    view.addSubview(&name);
    let reason = wrapping(mtm, &row.line, &NSFont::systemFontOfSize(12.0));
    reason.setTextColor(Some(&NSColor::secondaryLabelColor()));
    reason.setFrame(below_top(text_x, top + 19.0, text_width, 30.0));
    view.addSubview(&reason);
    add_badge(mtm, view, row.shown, top);
    let Some(choice) = button(row.shown) else {
        return;
    };
    let action = match choice {
        RowButton::Request => sel!(request:),
        RowButton::OpenSystemSettings => sel!(openSettings:),
    };
    let index = Permission::ALL
        .iter()
        .position(|p| *p == row.permission)
        .unwrap_or_default();
    let control = push_button(mtm, target, choice.title(), action, index_isize(index));
    control.setFrame(below_top(
        WIDTH - MARGIN - STATUS_WIDTH,
        top + 18.0,
        STATUS_WIDTH,
        24.0,
    ));
    view.addSubview(&control);
}

/// The tinted tile with the permission's symbol at the left of a row.
fn add_tile(mtm: MainThreadMarker, view: &NSView, permission: Permission, top: f64) {
    let tile = NSView::initWithFrame(
        mtm.alloc::<NSView>(),
        below_top(MARGIN, top + (ROW_HEIGHT - TILE) / 2.0, TILE, TILE),
    );
    tile.setWantsLayer(true);
    if let Some(layer) = tile.layer() {
        let tint = NSColor::systemBlueColor().colorWithAlphaComponent(0.15);
        layer.setBackgroundColor(Some(&tint.CGColor()));
        layer.setCornerRadius(8.0);
    }
    if let Some(image) = symbol_image(symbol(permission)) {
        let picture = NSImageView::imageViewWithImage(&image, mtm);
        picture.setContentTintColor(Some(&NSColor::systemBlueColor()));
        picture.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(TILE, TILE)));
        tile.addSubview(&picture);
    }
    view.addSubview(&tile);
}

/// The badge at the right of a row: a coloured dot or a spinner, then the words.
pub(super) fn add_badge(mtm: MainThreadMarker, view: &NSView, shown: Shown, top: f64) {
    let (words, mark) = badge(shown);
    let right = WIDTH - MARGIN;
    let text = label(mtm, words, &NSFont::systemFontOfSize(11.0));
    text.setTextColor(Some(&NSColor::secondaryLabelColor()));
    text.sizeToFit();
    let width = text.frame().size.width;
    let y = if button(shown).is_some() {
        top
    } else {
        top + 13.0
    };
    text.setFrame(below_top(right - width, y, width, 16.0));
    view.addSubview(&text);
    let mark_x = right - width - 14.0;
    match mark {
        BadgeMark::Spinner => {
            let spinner = NSProgressIndicator::initWithFrame(
                mtm.alloc::<NSProgressIndicator>(),
                below_top(mark_x, y + 2.0, 12.0, 12.0),
            );
            spinner.setStyle(NSProgressIndicatorStyle::Spinning);
            spinner.setIndeterminate(true);
            // SAFETY: a main-thread view built with a MainThreadMarker; a nil sender is allowed.
            unsafe { spinner.startAnimation(None) };
            view.addSubview(&spinner);
        }
        mark => {
            let colour = match mark {
                BadgeMark::Green => NSColor::systemGreenColor(),
                BadgeMark::Red => NSColor::systemRedColor(),
                _ => NSColor::systemGrayColor(),
            };
            let dot = label(mtm, "●", &NSFont::systemFontOfSize(9.0));
            dot.setTextColor(Some(&colour));
            dot.setFrame(below_top(mark_x, y + 1.0, 12.0, 14.0));
            view.addSubview(&dot);
        }
    }
}

pub(super) fn medium(size: f64) -> Retained<NSFont> {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    NSFont::systemFontOfSize_weight(size, unsafe { NSFontWeightMedium })
}

/// A push button tagged `tag`, so the action knows which row sent it.
pub(super) fn push_button(
    mtm: MainThreadMarker,
    target: &WindowTarget,
    title: &str,
    action: Sel,
    tag: isize,
) -> Retained<NSButton> {
    let control = appkit::push_button(mtm, target, title, action);
    control.setTag(tag);
    control
}

pub(super) fn symbol_image(name: &str) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name))?;
    let config =
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        NSImageSymbolConfiguration::configurationWithPointSize_weight(SYMBOL_SIZE, unsafe {
            objc2_app_kit::NSFontWeightRegular
        });
    image.imageWithSymbolConfiguration(&config)
}
