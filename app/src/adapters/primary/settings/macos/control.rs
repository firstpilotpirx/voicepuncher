//! The system control of one field: the value it shows, the value the user set in it, and
//! the red outline of a refused value.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSButton, NSColor, NSControlStateValueOff, NSControlStateValueOn, NSPathControl, NSPopUpButton,
    NSSlider, NSStepper, NSSwitch, NSTextField, NSTextView, NSView,
};
use objc2_foundation::{NSString, NSURL};
use toml::Value;

use super::examples::ExamplesTable;
use super::field::Field;
use super::files::resolve;
use super::values::{languages_text, languages_value, Choice};

/// The outline of a control whose value Save refused.
const OUTLINE_WIDTH: f64 = 1.0;
/// Slider values are shown and kept to two decimals.
const SLIDER_STEPS: f64 = 100.0;

/// The `AppKit` views a field's value lives in.
pub enum Widget {
    Switch(Retained<NSSwitch>),
    PopUp(Retained<NSPopUpButton>, RefCell<Vec<Choice>>),
    Radio(Vec<(Retained<NSButton>, Value)>),
    Number { text: Retained<NSTextField>, stepper: Retained<NSStepper> },
    Slider { slider: Retained<NSSlider>, shown: Retained<NSTextField> },
    Text(Retained<NSTextField>),
    Languages(Retained<NSTextField>),
    TextArea(Retained<NSTextView>),
    Folder(Retained<NSPathControl>, RefCell<String>),
    Examples(ExamplesTable),
}

/// One field's control in the window.
pub struct FieldControl {
    pub field: &'static Field,
    /// What the grid holds in the field's row: the control with its unit or buttons.
    pub view: Retained<NSView>,
    /// The view the red outline goes around.
    pub outlined: Retained<NSView>,
    pub widget: Widget,
    /// The value last shown, kept when the control holds it at a coarser precision.
    shown: RefCell<Value>,
}

impl FieldControl {
    pub fn new(
        field: &'static Field,
        view: Retained<NSView>,
        outlined: Retained<NSView>,
        widget: Widget,
    ) -> Self {
        Self { field, view, outlined, widget, shown: RefCell::new(Value::Boolean(false)) }
    }

    /// The same control placed in the grid inside `view`, with the views that follow it.
    pub fn placed_in(self, view: Retained<NSView>) -> Self {
        Self { view, ..self }
    }

    /// Shows `value` in the control.
    pub fn show(&self, value: &Value) {
        self.shown.replace(value.clone());
        match &self.widget {
            Widget::Switch(switch) => switch.setState(if value.as_bool() == Some(true) {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            }),
            Widget::PopUp(popup, choices) => select(popup, &mut choices.borrow_mut(), value),
            Widget::Radio(buttons) => buttons.iter().for_each(|(button, choice)| {
                button.setState(if choice == value {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }),
            Widget::Number { text, stepper } => {
                text.setStringValue(&NSString::from_str(&plain(value)));
                stepper.setDoubleValue(number(value).unwrap_or_default());
            }
            Widget::Slider { slider, shown } => {
                slider.setDoubleValue(number(value).unwrap_or_default());
                shown.setStringValue(&NSString::from_str(&two_decimals(slider.doubleValue())));
            }
            Widget::Text(text) => text.setStringValue(&NSString::from_str(&plain(value))),
            Widget::Languages(text) => {
                text.setStringValue(&NSString::from_str(&languages_text(value)));
            }
            Widget::TextArea(view) => view.setString(&NSString::from_str(&plain(value))),
            Widget::Folder(path, text) => show_folder(path, text, &plain(value)),
            Widget::Examples(table) => table.show(value),
        }
    }

    /// The value the control holds now.
    pub fn read(&self) -> Value {
        let shown = self.shown.borrow().clone();
        match &self.widget {
            Widget::Switch(switch) => Value::Boolean(switch.state() == NSControlStateValueOn),
            Widget::PopUp(popup, choices) => usize::try_from(popup.indexOfSelectedItem())
                .ok()
                .and_then(|index| choices.borrow().get(index).map(|c| c.value.clone()))
                .unwrap_or(shown),
            Widget::Radio(buttons) => buttons
                .iter()
                .find(|(button, _)| button.state() == NSControlStateValueOn)
                .map(|(_, value)| value.clone())
                .unwrap_or(shown),
            Widget::Number { text, .. } => typed_number(&text.stringValue().to_string(), &shown),
            Widget::Slider { slider, .. } => kept_slider(slider.doubleValue(), shown),
            Widget::Text(text) => Value::String(text.stringValue().to_string()),
            Widget::Languages(text) => languages_value(&text.stringValue().to_string()),
            Widget::TextArea(view) => Value::String(view.string().to_string()),
            Widget::Folder(_, text) => Value::String(text.borrow().clone()),
            Widget::Examples(table) => table.read(),
        }
    }

    /// Draws or removes the red outline of a refused value.
    pub fn mark(&self, refused: bool) {
        self.outlined.setWantsLayer(true);
        let Some(layer) = self.outlined.layer() else {
            return;
        };
        if refused {
            layer.setBorderColor(Some(&NSColor::systemRedColor().CGColor()));
            layer.setBorderWidth(OUTLINE_WIDTH);
        } else {
            layer.setBorderWidth(0.0);
        }
    }
}

/// Selects the choice holding `value`; a value none offers (a microphone that is not
/// connected) is added under its own name, so the window never changes it unasked.
fn select(popup: &NSPopUpButton, choices: &mut Vec<Choice>, value: &Value) {
    let index = if let Some(index) = choices.iter().position(|choice| choice.value == *value) {
        index
    } else {
        let title = plain(value);
        popup.addItemWithTitle(&NSString::from_str(&title));
        choices.push(Choice { title, value: value.clone() });
        choices.len() - 1
    };
    popup.selectItemAtIndex(index as isize);
}

/// The folder's own words (`~/…`) are kept; the path control shows where they lead.
fn show_folder(path: &NSPathControl, text: &RefCell<String>, folder: &str) {
    text.replace(folder.to_owned());
    let resolved = resolve(folder);
    let url = NSURL::fileURLWithPath(&NSString::from_str(&resolved.to_string_lossy()));
    path.setURL(Some(&url));
}

fn plain(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn number(value: &Value) -> Option<f64> {
    value.as_integer().map(|n| n as f64).or(value.as_float())
}

/// The typed number in the kind the file holds; text that is no number is kept as text,
/// so Save refuses it with the field's range.
fn typed_number(text: &str, shown: &Value) -> Value {
    let text = text.trim();
    match (shown, text.parse::<i64>(), text.parse::<f64>()) {
        (Value::Float(_), _, Ok(n)) => Value::Float(n),
        (_, Ok(n), _) => Value::Integer(n),
        (_, _, Ok(n)) => Value::Float(n),
        _ => Value::String(text.to_owned()),
    }
}

/// The slider's value; the shown value when the slider rounds to it, so a value the file
/// holds at more decimals is not counted as an edit.
fn kept_slider(position: f64, shown: Value) -> Value {
    let rounded = (position * SLIDER_STEPS).round() / SLIDER_STEPS;
    match number(&shown) {
        Some(n) if (n * SLIDER_STEPS).round() / SLIDER_STEPS == rounded => shown,
        _ => Value::Float(rounded),
    }
}

/// A slider value as its label shows it: `0.20`.
pub fn two_decimals(value: f64) -> String {
    format!("{value:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_a_whole_number_as_an_integer() {
        // Given / When
        let value = typed_number(" 300 ", &Value::Integer(150));
        // Then
        assert_eq!(value, Value::Integer(300));
    }

    #[test]
    fn should_keep_a_number_the_file_holds_as_a_float_a_float() {
        // Given / When
        let value = typed_number("2", &Value::Float(1.5));
        // Then
        assert_eq!(value, Value::Float(2.0));
    }

    #[test]
    fn should_keep_text_that_is_no_number_for_the_range_check() {
        // Given / When
        let value = typed_number("abc", &Value::Integer(150));
        // Then
        assert_eq!(value, Value::String("abc".to_owned()));
    }

    #[test]
    fn should_keep_the_file_value_when_the_slider_rounds_to_it() {
        // Given
        let file = Value::Float(f64::from(0.2_f32));
        // When
        let value = kept_slider(0.2, file.clone());
        // Then
        assert_eq!(value, file);
    }

    #[test]
    fn should_round_a_moved_slider_to_two_decimals() {
        // Given / When
        let value = kept_slider(0.456_789, Value::Float(0.0));
        // Then
        assert_eq!(value, Value::Float(0.46));
    }
}
