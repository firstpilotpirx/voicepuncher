//! Builds the system control of each field kind, wired to the form's target.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{
    NSBorderType, NSButton, NSControl, NSLayoutConstraint, NSPathControl, NSPopUpButton, NSSlider,
    NSStackView, NSStepper, NSSwitch, NSTextField, NSTextView, NSUserInterfaceLayoutOrientation,
    NSView,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSString};

use super::control::{FieldControl, Widget};
use super::examples::{ExamplesTable, TABLE_SIZE};
use super::field::{Choices, Control, Field};
use super::session::SettingsSession;
use super::target::{actions, FormTarget};

const NUMBER_WIDTH: f64 = 80.0;
const TEXT_WIDTH: f64 = 300.0;
const SLIDER_WIDTH: f64 = 200.0;
const VALUE_WIDTH: f64 = 40.0;
const PATH_WIDTH: f64 = 140.0;
/// Text view heights: the Dictation prompt, the Refinement system prompt.
const PROMPT_HEIGHT: f64 = 90.0;
const SYSTEM_PROMPT_HEIGHT: f64 = 120.0;
const SYSTEM_PROMPT_KEY: &str = "system_prompt";
const ROW_SPACING: f64 = 8.0;
const LANGUAGES_PLACEHOLDER: &str = "comma-separated codes, e.g. en, ru";
const PLAY_TITLE: &str = "▶";
const CHOOSE_TITLE: &str = "Choose…";
const REVEAL_TITLE: &str = "Show in Finder";
const ADD_TITLE: &str = "+";
const REMOVE_TITLE: &str = "−";

/// What a control is built from.
pub struct Parts<'a> {
    pub mtm: MainThreadMarker,
    pub target: &'a FormTarget,
    pub session: &'a SettingsSession,
}

/// The control of `field`, the `tag`-th of the field table, showing the session's value.
pub fn control(parts: &Parts, field: &'static Field, tag: isize) -> FieldControl {
    let built = match field.control {
        Control::Switch => switch(parts, tag),
        Control::PopUp(choices) => popup(parts, choices, tag),
        Control::Radio(choices) => radio(parts, choices, tag),
        Control::Number { unit, .. } => number(parts, unit, tag),
        Control::Slider { min, max } => slider(parts, min, max, tag),
        Control::Text | Control::Address(_) => text(parts, tag, Widget::Text),
        Control::LanguageList => text(parts, tag, Widget::Languages),
        Control::TextArea => text_area(parts, field),
        Control::Folder => folder(parts, tag),
        Control::Examples => examples(parts),
    };
    let (view, outlined, widget) = built;
    let control = FieldControl::new(field, view, outlined, widget);
    control.show(&parts.session.value(field));
    if let (Control::PopUp(Choices::Sounds), Widget::PopUp(..)) = (field.control, &control.widget) {
        let play = button(parts, PLAY_TITLE, actions::play(), tag);
        return with_view_after(parts.mtm, control, &play);
    }
    control
}

type Built = (Retained<NSView>, Retained<NSView>, Widget);

fn view_of<T: AsRef<NSView>>(view: &T) -> Retained<NSView> {
    view.as_ref().retain()
}

fn switch(parts: &Parts, tag: isize) -> Built {
    let switch = NSSwitch::new(parts.mtm);
    parts.target.wire(&switch, actions::changed(), tag);
    (view_of(&switch), view_of(&switch), Widget::Switch(switch))
}

fn popup(parts: &Parts, choices: Choices, tag: isize) -> Built {
    let options = parts.session.options(choices);
    let popup =
        NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(parts.mtm), zero(), false);
    for option in &options {
        popup.addItemWithTitle(&NSString::from_str(&option.title));
    }
    parts.target.wire(&popup, actions::changed(), tag);
    let widget = Widget::PopUp(popup.clone(), RefCell::new(options));
    (view_of(&popup), view_of(&popup), widget)
}

fn radio(parts: &Parts, choices: Choices, tag: isize) -> Built {
    let buttons: Vec<_> = parts
        .session
        .options(choices)
        .into_iter()
        .map(|choice| {
            // SAFETY: the target and action are set by `wire` right after.
            let button = unsafe {
                NSButton::radioButtonWithTitle_target_action(
                    &NSString::from_str(&choice.title),
                    None,
                    None,
                    parts.mtm,
                )
            };
            parts.target.wire(&button, actions::changed(), tag);
            (button, choice.value)
        })
        .collect();
    let views: Vec<&NSView> = buttons.iter().map(|(button, _)| button.as_ref()).collect();
    let row = stack(parts.mtm, &views);
    (view_of(&row), view_of(&row), Widget::Radio(buttons))
}

fn number(parts: &Parts, unit: &str, tag: isize) -> Built {
    let text = NSTextField::textFieldWithString(&NSString::new(), parts.mtm);
    width(&text, NUMBER_WIDTH);
    parts.target.wire(&text, actions::changed(), tag);
    let stepper = NSStepper::new(parts.mtm);
    stepper.setMinValue(f64::MIN);
    stepper.setMaxValue(f64::MAX);
    parts.target.wire(&stepper, actions::stepped(), tag);
    let unit = NSTextField::labelWithString(&NSString::from_str(unit), parts.mtm);
    let row = stack(parts.mtm, &[text.as_ref(), stepper.as_ref(), unit.as_ref()]);
    (
        view_of(&row),
        view_of(&text),
        Widget::Number { text, stepper },
    )
}

fn slider(parts: &Parts, min: f64, max: f64, tag: isize) -> Built {
    let slider = NSSlider::new(parts.mtm);
    slider.setMinValue(min);
    slider.setMaxValue(max);
    width(&slider, SLIDER_WIDTH);
    parts.target.wire(&slider, actions::changed(), tag);
    let shown = NSTextField::labelWithString(&NSString::new(), parts.mtm);
    width(&shown, VALUE_WIDTH);
    let row = stack(parts.mtm, &[slider.as_ref(), shown.as_ref()]);
    (
        view_of(&row),
        view_of(&slider),
        Widget::Slider { slider, shown },
    )
}

fn text(parts: &Parts, tag: isize, widget: fn(Retained<NSTextField>) -> Widget) -> Built {
    let text = NSTextField::textFieldWithString(&NSString::new(), parts.mtm);
    width(&text, TEXT_WIDTH);
    parts.target.wire(&text, actions::changed(), tag);
    if matches!(widget(text.clone()), Widget::Languages(_)) {
        text.setPlaceholderString(Some(&NSString::from_str(LANGUAGES_PLACEHOLDER)));
    }
    (view_of(&text), view_of(&text), widget(text))
}

fn text_area(parts: &Parts, field: &Field) -> Built {
    let scroll = NSTextView::scrollableTextView(parts.mtm);
    let height = if field.key == SYSTEM_PROMPT_KEY {
        SYSTEM_PROMPT_HEIGHT
    } else {
        PROMPT_HEIGHT
    };
    scroll.setBorderType(NSBorderType::BezelBorder);
    width(&scroll, TEXT_WIDTH);
    height_of(&scroll, height);
    let view = scroll
        .documentView()
        .and_then(|view| view.downcast::<NSTextView>().ok())
        .expect("a scrollable text view holds a text view");
    view.setRichText(false);
    (view_of(&scroll), view_of(&scroll), Widget::TextArea(view))
}

fn folder(parts: &Parts, tag: isize) -> Built {
    let path = NSPathControl::new(parts.mtm);
    width(&path, PATH_WIDTH);
    let choose = button(parts, CHOOSE_TITLE, actions::choose(), tag);
    let reveal = button(parts, REVEAL_TITLE, actions::reveal(), tag);
    let row = stack(
        parts.mtm,
        &[path.as_ref(), choose.as_ref(), reveal.as_ref()],
    );
    let widget = Widget::Folder(path.clone(), RefCell::new(String::new()));
    (view_of(&row), view_of(&path), widget)
}

fn examples(parts: &Parts) -> Built {
    let table = ExamplesTable::new(parts.mtm);
    let add = button(parts, ADD_TITLE, actions::add_example(), 0);
    let remove = button(parts, REMOVE_TITLE, actions::remove_example(), 0);
    let buttons = stack(parts.mtm, &[add.as_ref(), remove.as_ref()]);
    let column = stack(parts.mtm, &[table.scroll.as_ref(), buttons.as_ref()]);
    column.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    column.setAlignment(objc2_app_kit::NSLayoutAttribute::Leading);
    width(&table.scroll, TABLE_SIZE.0);
    height_of(&table.scroll, TABLE_SIZE.1);
    let outlined = view_of(&table.scroll);
    (view_of(&column), outlined, Widget::Examples(table))
}

/// A push button calling `action` with `tag`.
pub fn button(
    parts: &Parts,
    title: &str,
    action: objc2::runtime::Sel,
    tag: isize,
) -> Retained<NSButton> {
    // SAFETY: the target and action are set by `wire` right after.
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(title), None, None, parts.mtm)
    };
    parts.target.wire(&button, action, tag);
    button
}

/// The control followed by `extra` in one row.
fn with_view_after(
    mtm: MainThreadMarker,
    control: FieldControl,
    extra: &NSControl,
) -> FieldControl {
    let row = stack(mtm, &[&control.view, extra.as_ref()]);
    control.placed_in(view_of(&row))
}

/// `views` side by side.
pub fn stack(mtm: MainThreadMarker, views: &[&NSView]) -> Retained<NSStackView> {
    let views: Vec<Retained<NSView>> = views.iter().map(|view| (*view).retain()).collect();
    let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(&views), mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    stack.setSpacing(ROW_SPACING);
    stack
}

fn zero() -> CGRect {
    CGRect::new(CGPoint::ZERO, CGSize::ZERO)
}

/// Fixes the width of `view`.
pub fn width<T: AsRef<NSView>>(view: &T, points: f64) {
    let constraint: Retained<NSLayoutConstraint> = view
        .as_ref()
        .widthAnchor()
        .constraintEqualToConstant(points);
    constraint.setActive(true);
}

fn height_of<T: AsRef<NSView>>(view: &T, points: f64) {
    let constraint = view
        .as_ref()
        .heightAnchor()
        .constraintEqualToConstant(points);
    constraint.setActive(true);
}
