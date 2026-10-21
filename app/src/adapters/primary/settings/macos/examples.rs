//! The refinement examples as a table of two editable columns, with + and − under it.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSScrollView, NSTableColumn, NSTableView, NSTableViewDataSource};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use toml::Value;

const TRANSCRIPT: &str = "transcript";
const NORMALIZED: &str = "normalized";
const COLUMNS: [(&str, &str); 2] = [(TRANSCRIPT, "Transcript"), (NORMALIZED, "Normalized")];
const COLUMN_WIDTH: f64 = 140.0;
/// Four rows show at once; the table scrolls past them.
pub const TABLE_SIZE: (f64, f64) = (300.0, 110.0);

/// One example: what the speech recognition service heard and how it should read.
type Example = (String, String);

pub struct ExamplesSourceIvars {
    rows: RefCell<Vec<Example>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPExamplesSource"]
    #[ivars = ExamplesSourceIvars]
    struct ExamplesSource;

    unsafe impl NSObjectProtocol for ExamplesSource {}

    unsafe impl NSTableViewDataSource for ExamplesSource {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> isize {
            self.ivars().rows.borrow().len() as isize
        }

        #[unsafe(method_id(tableView:objectValueForTableColumn:row:))]
        fn object_value(
            &self,
            _table: &NSTableView,
            column: Option<&NSTableColumn>,
            row: isize,
        ) -> Option<Retained<AnyObject>> {
            self.cell(column, row)
                .map(|text| Retained::into_super(Retained::into_super(NSString::from_str(&text))))
        }

        #[unsafe(method(tableView:setObjectValue:forTableColumn:row:))]
        unsafe fn set_object_value(
            &self,
            _table: &NSTableView,
            object: Option<&AnyObject>,
            column: Option<&NSTableColumn>,
            row: isize,
        ) {
            let text = object
                .and_then(|object| object.downcast_ref::<NSString>())
                .map(|text| text.to_string())
                .unwrap_or_default();
            let mut rows = self.ivars().rows.borrow_mut();
            let Some(example) = usize::try_from(row).ok().and_then(|row| rows.get_mut(row)) else {
                return;
            };
            if is_transcript(column) {
                example.0 = text;
            } else {
                example.1 = text;
            }
        }
    }
);

fn is_transcript(column: Option<&NSTableColumn>) -> bool {
    column.is_some_and(|column| column.identifier().to_string() == TRANSCRIPT)
}

impl ExamplesSource {
    /// The text of `row` in `column`.
    fn cell(&self, column: Option<&NSTableColumn>, row: isize) -> Option<String> {
        let rows = self.ivars().rows.borrow();
        let example = rows.get(usize::try_from(row).ok()?)?;
        Some(
            if is_transcript(column) {
                &example.0
            } else {
                &example.1
            }
            .clone(),
        )
    }

    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ExamplesSourceIvars {
            rows: RefCell::new(Vec::new()),
        });
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }
}

/// The examples table of the Refinement tab.
pub struct ExamplesTable {
    pub scroll: Retained<NSScrollView>,
    table: Retained<NSTableView>,
    source: Retained<ExamplesSource>,
}

impl ExamplesTable {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let frame = CGRect::new(CGPoint::ZERO, CGSize::new(TABLE_SIZE.0, TABLE_SIZE.1));
        let table = NSTableView::initWithFrame(NSTableView::alloc(mtm), frame);
        for (identifier, title) in COLUMNS {
            let column = NSTableColumn::initWithIdentifier(
                NSTableColumn::alloc(mtm),
                &NSString::from_str(identifier),
            );
            column.setTitle(&NSString::from_str(title));
            column.setWidth(COLUMN_WIDTH);
            table.addTableColumn(&column);
        }
        table.setUsesAlternatingRowBackgroundColors(true);
        let source = ExamplesSource::new(mtm);
        // SAFETY: the source lives as long as the table: both belong to this value.
        unsafe { table.setDataSource(Some(ProtocolObject::from_ref(&*source))) };
        let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
        scroll.setDocumentView(Some(&table));
        scroll.setHasVerticalScroller(true);
        Self {
            scroll,
            table,
            source,
        }
    }

    /// Shows the examples of `value`, a list of `{ transcript, normalized }` tables.
    pub fn show(&self, value: &Value) {
        let rows = value
            .as_array()
            .map(|examples| examples.iter().map(example).collect())
            .unwrap_or_default();
        self.source.ivars().rows.replace(rows);
        self.table.reloadData();
    }

    /// The examples as the file stores them.
    pub fn read(&self) -> Value {
        self.end_editing();
        let rows = self.source.ivars().rows.borrow();
        Value::Array(
            rows.iter()
                .map(|(transcript, normalized)| {
                    let mut table = toml::Table::new();
                    table.insert(TRANSCRIPT.to_owned(), Value::String(transcript.clone()));
                    table.insert(NORMALIZED.to_owned(), Value::String(normalized.clone()));
                    Value::Table(table)
                })
                .collect(),
        )
    }

    /// Adds an empty example at the end and starts editing it.
    pub fn add(&self) {
        let row = {
            let mut rows = self.source.ivars().rows.borrow_mut();
            rows.push((String::new(), String::new()));
            rows.len() - 1
        };
        self.table.reloadData();
        self.table
            .editColumn_row_withEvent_select(0, row as isize, None, true);
    }

    /// Removes the selected example, if one is selected.
    pub fn remove_selected(&self) {
        self.end_editing();
        let Ok(row) = usize::try_from(self.table.selectedRow()) else {
            return;
        };
        let mut rows = self.source.ivars().rows.borrow_mut();
        if row < rows.len() {
            rows.remove(row);
        }
        drop(rows);
        self.table.reloadData();
    }

    /// Commits the cell being edited, so reading sees what the user typed.
    fn end_editing(&self) {
        if let Some(window) = self.table.window() {
            window.makeFirstResponder(Some(&self.table));
        }
    }
}

fn example(value: &Value) -> Example {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    (text(TRANSCRIPT), text(NORMALIZED))
}
