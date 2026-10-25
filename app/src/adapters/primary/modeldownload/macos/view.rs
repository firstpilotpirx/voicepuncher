//! The content of the download window.

use super::{
    appkit, below_top, human_size, label, mapping, push_button, sel, status_text, wrapping,
    MainButton, MainThreadMarker, NSButton, NSColor, NSFont, NSFontWeightMedium,
    NSProgressIndicator, NSProgressIndicatorStyle, NSString, NSTextAlignment, NSView, Retained,
    WindowModel, WindowTarget, BUTTON_WIDTH, HEIGHT, MARGIN, ROW_HEIGHT, SIZE_WIDTH, SIZE_X,
    STATUS_X, TABLE_TOP, WIDTH,
};

pub(super) fn content(
    mtm: MainThreadMarker,
    target: &WindowTarget,
    model: &WindowModel,
) -> Retained<NSView> {
    let view = appkit::content_view(mtm, WIDTH, HEIGHT);
    let intro = wrapping(mtm, &model.intro(), &NSFont::systemFontOfSize(13.0));
    intro.setFrame(below_top(MARGIN, 14.0, WIDTH - 2.0 * MARGIN, 36.0));
    view.addSubview(&intro);
    for (index, row) in model.rows.iter().enumerate() {
        add_row(mtm, &view, TABLE_TOP + index as f64 * ROW_HEIGHT, row);
    }
    add_totals(mtm, &view, model);
    add_buttons(mtm, &view, target, model);
    view
}

/// One model file's row: its name, purpose, size and status.
fn add_row(mtm: MainThreadMarker, view: &NSView, top: f64, row: &mapping::Row) {
    let name = label(
        mtm,
        row.name,
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        &NSFont::monospacedSystemFontOfSize_weight(12.0, unsafe { NSFontWeightMedium }),
    );
    name.setFrame(below_top(MARGIN, top, SIZE_X - MARGIN, 16.0));
    view.addSubview(&name);
    let purpose = label(mtm, row.purpose, &NSFont::systemFontOfSize(11.0));
    purpose.setTextColor(Some(&NSColor::secondaryLabelColor()));
    purpose.setFrame(below_top(MARGIN, top + 17.0, SIZE_X - MARGIN, 15.0));
    view.addSubview(&purpose);
    let size = label(
        mtm,
        &human_size(row.size_bytes),
        &NSFont::systemFontOfSize(12.0),
    );
    size.setAlignment(NSTextAlignment::Right);
    size.setFrame(below_top(SIZE_X, top, SIZE_WIDTH, 16.0));
    view.addSubview(&size);
    let status = wrapping(
        mtm,
        &status_text(&row.status),
        &NSFont::systemFontOfSize(11.0),
    );
    let colour = match row.status {
        mapping::RowStatus::Present => NSColor::systemGreenColor(),
        mapping::RowStatus::Failed(_) => NSColor::systemRedColor(),
        _ => NSColor::labelColor(),
    };
    status.setTextColor(Some(&colour));
    status.setFrame(below_top(STATUS_X, top, WIDTH - MARGIN - STATUS_X, 32.0));
    view.addSubview(&status);
}

/// The progress bar, the total line and the free space line below the table.
fn add_totals(mtm: MainThreadMarker, view: &NSView, model: &WindowModel) {
    let text_width = WIDTH - 2.0 * MARGIN;
    let below_table = TABLE_TOP + model.rows.len() as f64 * ROW_HEIGHT + 8.0;
    let bar = NSProgressIndicator::initWithFrame(
        mtm.alloc::<NSProgressIndicator>(),
        below_top(MARGIN, below_table, text_width, 12.0),
    );
    bar.setStyle(NSProgressIndicatorStyle::Bar);
    bar.setIndeterminate(false);
    bar.setMinValue(0.0);
    bar.setMaxValue(1.0);
    bar.setDoubleValue(model.total_fraction());
    view.addSubview(&bar);
    let total = label(mtm, &model.total_line(), &NSFont::systemFontOfSize(12.0));
    total.setTextColor(Some(&NSColor::secondaryLabelColor()));
    total.setFrame(below_top(MARGIN, below_table + 16.0, text_width, 16.0));
    view.addSubview(&total);
    let (space, red) = model.space_line();
    let space = label(mtm, &space, &NSFont::systemFontOfSize(12.0));
    if red {
        space.setTextColor(Some(&NSColor::systemRedColor()));
    }
    space.setFrame(below_top(MARGIN, below_table + 34.0, text_width, 16.0));
    view.addSubview(&space);
}

/// The footer buttons, the default one answering Return.
fn add_buttons(mtm: MainThreadMarker, view: &NSView, target: &WindowTarget, model: &WindowModel) {
    let footer = HEIGHT - 42.0;
    let place = |button: &NSButton, x: f64, width: f64| {
        button.setFrame(below_top(x, footer, width, 28.0));
        view.addSubview(button);
    };
    let open = push_button(mtm, target, mapping::OPEN_FOLDER, sel!(openFolder:));
    place(&open, MARGIN, BUTTON_WIDTH + 10.0);
    let quit = push_button(mtm, target, mapping::QUIT, sel!(quit:));
    place(&quit, MARGIN + BUTTON_WIDTH + 16.0, BUTTON_WIDTH);
    let done = push_button(mtm, target, mapping::DONE, sel!(done:));
    done.setEnabled(model.done_enabled());
    place(&done, WIDTH - MARGIN - BUTTON_WIDTH, BUTTON_WIDTH);
    let main = model.main_button();
    let download = push_button(mtm, target, main.title(), sel!(download:));
    download.setEnabled(model.main_enabled());
    if main != MainButton::Cancel && !model.done_enabled() {
        download.setKeyEquivalent(&NSString::from_str("\r"));
    } else if model.done_enabled() {
        done.setKeyEquivalent(&NSString::from_str("\r"));
    }
    place(
        &download,
        WIDTH - MARGIN - 2.0 * BUTTON_WIDTH - 8.0,
        BUTTON_WIDTH,
    );
}
