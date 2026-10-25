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
use super::field::{Choices, Control, Field};
use super::session::SettingsSession;
use super::target::{Action, FormTarget};

const NUMBER_WIDTH: f64 = 80.0;
const TEXT_WIDTH: f64 = 300.0;
const SLIDER_WIDTH: f64 = 200.0;
const VALUE_WIDTH: f64 = 40.0;
const PATH_WIDTH: f64 = 140.0;
/// Text view height: the Dictation prompt.
const PROMPT_HEIGHT: f64 = 90.0;
const ROW_SPACING: f64 = 8.0;
const LANGUAGES_PLACEHOLDER: &str = "comma-separated codes, e.g. en, ru";
const PLAY_TITLE: &str = "▶";
const CHOOSE_TITLE: &str = "Choose…";
const REVEAL_TITLE: &str = "Show in Finder";

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
        Control::Text | Control::Address(_) => text(parts, tag, Widget::Text, ""),
        Control::LanguageList => text(parts, tag, Widget::Languages, LANGUAGES_PLACEHOLDER),
        Control::TextArea => text_area(parts),
        Control::Folder => folder(parts, tag),
    };
    let (view, outlined, widget) = built;
    let control = FieldControl::new(field, view, outlined, widget);
    control.show(&parts.session.value(field));
    with_play_button(parts, control, tag)
}

/// A sound's pop-up with the button that plays the chosen sound after it; any other control
/// as it is.
fn with_play_button(parts: &Parts, control: FieldControl, tag: isize) -> FieldControl {
    if let (Control::PopUp(Choices::Sounds), Widget::PopUp(..)) =
        (control.field.control, &control.widget)
    {
        let play = button(parts, PLAY_TITLE, Action::PLAY, tag);
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
    parts.target.wire(&switch, Action::CHANGED, tag);
    (view_of(&switch), view_of(&switch), Widget::Switch(switch))
}

fn popup(parts: &Parts, choices: Choices, tag: isize) -> Built {
    let options = parts.session.options(choices);
    let popup =
        NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(parts.mtm), zero(), false);
    for option in &options {
        popup.addItemWithTitle(&NSString::from_str(&option.title));
    }
    parts.target.wire(&popup, Action::CHANGED, tag);
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
            parts.target.wire(&button, Action::CHANGED, tag);
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
    parts.target.wire(&text, Action::CHANGED, tag);
    let stepper = NSStepper::new(parts.mtm);
    stepper.setMinValue(f64::MIN);
    stepper.setMaxValue(f64::MAX);
    parts.target.wire(&stepper, Action::STEPPED, tag);
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
    parts.target.wire(&slider, Action::CHANGED, tag);
    let shown = NSTextField::labelWithString(&NSString::new(), parts.mtm);
    width(&shown, VALUE_WIDTH);
    let row = stack(parts.mtm, &[slider.as_ref(), shown.as_ref()]);
    (
        view_of(&row),
        view_of(&slider),
        Widget::Slider { slider, shown },
    )
}

/// Makes the widget of a text field.
type TextWidget = fn(Retained<NSTextField>) -> Widget;

fn text(parts: &Parts, tag: isize, widget: TextWidget, placeholder: &str) -> Built {
    let text = NSTextField::textFieldWithString(&NSString::new(), parts.mtm);
    width(&text, TEXT_WIDTH);
    parts.target.wire(&text, Action::CHANGED, tag);
    text.setPlaceholderString(Some(&NSString::from_str(placeholder)));
    (view_of(&text), view_of(&text), widget(text))
}

fn text_area(parts: &Parts) -> Built {
    let scroll = NSTextView::scrollableTextView(parts.mtm);
    scroll.setBorderType(NSBorderType::BezelBorder);
    width(&scroll, TEXT_WIDTH);
    height_of(&scroll, PROMPT_HEIGHT);
    let held = scroll
        .documentView()
        .and_then(|view| view.downcast::<NSTextView>().ok());
    // AppKit's scrollable text view holds one; should it not, the field gets its own.
    let view = held.unwrap_or_else(|| NSTextView::new(parts.mtm));
    scroll.setDocumentView(Some(&view));
    view.setRichText(false);
    (view_of(&scroll), view_of(&scroll), Widget::TextArea(view))
}

fn folder(parts: &Parts, tag: isize) -> Built {
    let path = NSPathControl::new(parts.mtm);
    width(&path, PATH_WIDTH);
    let choose = button(parts, CHOOSE_TITLE, Action::CHOOSE, tag);
    let reveal = button(parts, REVEAL_TITLE, Action::REVEAL, tag);
    let row = stack(
        parts.mtm,
        &[path.as_ref(), choose.as_ref(), reveal.as_ref()],
    );
    let widget = Widget::Folder(path.clone(), RefCell::new(String::new()));
    (view_of(&row), view_of(&path), widget)
}

/// A push button calling `action` with `tag`.
pub fn button(parts: &Parts, title: &str, action: Action, tag: isize) -> Retained<NSButton> {
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
