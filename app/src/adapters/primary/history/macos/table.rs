//! The window's content: a table of the rows (time, text, Copy) or one line instead of it.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSButton, NSControlTextEditingDelegate, NSFont, NSLineBreakMode, NSScrollView, NSTableColumn,
    NSTableView, NSTableViewDataSource, NSTableViewDelegate, NSView,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::super::{HistoryView, Shown, ShownRow};
use super::{copy_row, WINDOW};
use crate::adapters::appkit::{self, label, push_button, wrapping};

const MARGIN: f64 = 16.0;
const ROW_HEIGHT: f64 = 26.0;
/// Room for the vertical scroller beside the rows.
const SCROLLER: f64 = 16.0;
const TIME_COLUMN: &str = "time";
const TEXT_COLUMN: &str = "text";
const COPY_COLUMN: &str = "copy";
/// Each column with its title and width; the text column takes what is left.
const COLUMNS: [(&str, &str, f64); 3] = [
    (TIME_COLUMN, "Time", 150.0),
    (TEXT_COLUMN, "Text", 0.0),
    (COPY_COLUMN, "", 70.0),
];
const COPY_TITLE: &str = "Copy";
const FONT_SIZE: f64 = 13.0;

/// The answer of `read` over the rows the table shows now; none while the window shows a line.
fn with_rows<T>(read: impl FnOnce(&[ShownRow]) -> T) -> T {
    WINDOW.with_borrow(|state| {
        match state
            .as_ref()
            .and_then(|s| s.view.as_ref())
            .map(HistoryView::shown)
        {
            Some(Shown::Rows(rows)) => read(rows),
            _ => read(&[]),
        }
    })
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPHistoryTableSource"]
    pub(super) struct TableSource;

    unsafe impl NSObjectProtocol for TableSource {}

    unsafe impl NSTableViewDataSource for TableSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> isize {
            isize::try_from(with_rows(<[ShownRow]>::len)).unwrap_or(isize::MAX)
        }
    }

    unsafe impl NSControlTextEditingDelegate for TableSource {}

    unsafe impl NSTableViewDelegate for TableSource {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for(
            &self,
            _table: &NSTableView,
            column: Option<&NSTableColumn>,
            row: isize,
        ) -> Option<Retained<NSView>> {
            self.cell_at(column, row)
        }
    }

    impl TableSource {
        #[unsafe(method(copy:))]
        fn copy(&self, sender: Option<&NSButton>) {
            if let Some(index) = sender.and_then(|button| usize::try_from(button.tag()).ok()) {
                copy_row(index);
            }
        }

        /// Copies the double-clicked row, else the selected one.
        #[unsafe(method(doubleClicked:))]
        fn double_clicked(&self, sender: Option<&NSTableView>) {
            let row = sender.map(|table| Some(table.clickedRow()).filter(|r| *r >= 0).unwrap_or_else(|| table.selectedRow()));
            if let Some(index) = row.and_then(|row| usize::try_from(row).ok()) {
                copy_row(index);
            }
        }
    }
);

impl TableSource {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        appkit::new_plain_object(mtm)
    }

    fn cell_at(&self, column: Option<&NSTableColumn>, row: isize) -> Option<Retained<NSView>> {
        let id = column?.identifier().to_string();
        let index = usize::try_from(row).ok()?;
        let shown = with_rows(|rows| rows.get(index).cloned())?;
        Some(self.cell(&id, &shown, row))
    }

    fn cell(&self, column: &str, shown: &ShownRow, row: isize) -> Retained<NSView> {
        let mtm = self.mtm();
        let font = NSFont::systemFontOfSize(FONT_SIZE);
        match column {
            COPY_COLUMN => {
                let button = push_button(mtm, self, COPY_TITLE, sel!(copy:));
                button.setTag(row);
                Retained::into_super(Retained::into_super(button))
            }
            TIME_COLUMN => {
                Retained::into_super(Retained::into_super(label(mtm, &shown.time, &font)))
            }
            _ => {
                let text = label(mtm, &shown.text, &font);
                text.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                text.setToolTip(Some(&NSString::from_str(&shown.text)));
                Retained::into_super(Retained::into_super(text))
            }
        }
    }
}

/// The content view of the window `width` by `height`.
pub(super) fn build_content(
    mtm: MainThreadMarker,
    source: &TableSource,
    shown: &Shown,
    width: f64,
    height: f64,
) -> Retained<NSView> {
    let content = appkit::content_view(mtm, width, height);
    let inner = NSRect::new(
        NSPoint::new(MARGIN, MARGIN),
        NSSize::new(width - 2.0 * MARGIN, height - 2.0 * MARGIN),
    );
    match shown {
        Shown::Note(note) => {
            let line = wrapping(mtm, note, &NSFont::systemFontOfSize(FONT_SIZE));
            line.setFrame(inner);
            content.addSubview(&line);
        }
        Shown::Rows(_) => content.addSubview(&scrolled_table(mtm, source, inner)),
    }
    content
}

fn scrolled_table(
    mtm: MainThreadMarker,
    source: &TableSource,
    frame: NSRect,
) -> Retained<NSScrollView> {
    let table = NSTableView::initWithFrame(mtm.alloc::<NSTableView>(), frame);
    // the text column takes what the fixed columns, the gaps between cells and the scroller leave
    let gaps = table.intercellSpacing().width * 3.0;
    let fixed: f64 = COLUMNS.iter().map(|(_, _, width)| width).sum::<f64>() + gaps + SCROLLER;
    for (id, title, width) in COLUMNS {
        let column = NSTableColumn::initWithIdentifier(
            mtm.alloc::<NSTableColumn>(),
            &NSString::from_str(id),
        );
        column.setTitle(&NSString::from_str(title));
        column.setWidth(if width > 0.0 {
            width
        } else {
            frame.size.width - fixed
        });
        table.addTableColumn(&column);
    }
    table.setRowHeight(ROW_HEIGHT);
    // SAFETY: the table only borrows its data source; the window state keeps `source` alive.
    unsafe { table.setDataSource(Some(ProtocolObject::from_ref(source))) };
    // SAFETY: as for the data source, the same object.
    unsafe { table.setDelegate(Some(ProtocolObject::from_ref(source))) };
    // SAFETY: `doubleClicked:` is a method of `source`, which the window state keeps alive.
    unsafe { table.setTarget(Some(source as &AnyObject)) };
    let double_clicked = sel!(doubleClicked:);
    // SAFETY: the action takes the sender, as a table view calls its double action.
    unsafe { table.setDoubleAction(Some(double_clicked)) };
    let scroll = NSScrollView::initWithFrame(mtm.alloc::<NSScrollView>(), frame);
    scroll.setHasVerticalScroller(true);
    scroll.setDocumentView(Some(&table));
    scroll
}
