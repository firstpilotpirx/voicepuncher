//! Given and When steps: what holds the focus and what the user does.

use super::{
    given, is_left_in_clipboard, run, when, Duration, Event, Focus, IntegrationWorld, Result,
    REACTION,
};

#[given("a text field has keyboard focus")]
pub(super) fn text_field_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TextField);
}

#[given("the focused app does not report its focused element")]
pub(super) fn focus_unreported(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::Unreported);
}

#[given("a text input on a web page has keyboard focus")]
pub(super) fn web_text_input_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebTextInput);
}

#[given("an editable element on a web page has keyboard focus")]
pub(super) fn web_editable_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebEditable);
}

#[given("a web page has keyboard focus and no text input on it has")]
pub(super) fn web_page_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebPage);
}

#[given("a web page with selected text has keyboard focus and no text input on it has")]
pub(super) fn web_page_with_selected_text_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebPageWithSelectedText);
}

#[given("the focused element is a terminal's window outside web content")]
pub(super) fn terminal_window_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TerminalWindow);
}

#[given("the focused element is a list that offers a text selection")]
pub(super) fn list_with_selection_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::ListWithTextSelection);
}

#[given("a button has keyboard focus")]
pub(super) fn button_focused(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::Button);
}

#[given("the config file sets the wait time to 0")]
pub(super) fn wait_time_zero(world: &mut IntegrationWorld) {
    run(world).wait_time = Duration::ZERO;
}

#[given("the app holds the text back because a button has focus")]
pub(super) fn text_held_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.set_focus(Focus::Button);
    run.finish_dictation()?;
    run.wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[given("a hotkey press ended a wait for focus")]
pub(super) fn wait_ended_by_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    text_held_back(world)?;
    let run = run(world);
    run.press_hotkey()?;
    run.wait_for(is_left_in_clipboard, REACTION)?;
    Ok(())
}

#[given("the app holds the text back because a web page has focus without a text input")]
pub(super) fn text_held_back_on_web_page(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.set_focus(Focus::WebPage);
    run.finish_dictation()?;
    run.wait_for(|e| *e == Event::WaitingShown, REACTION)?;
    Ok(())
}

#[when("a dictation finishes")]
pub(super) fn dictation_finishes(world: &mut IntegrationWorld) -> Result<()> {
    run(world).finish_dictation()
}

#[when("the user clicks into a text field")]
pub(super) fn click_into_text_field(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::TextField);
}

#[when("the user clicks into a text input on that page")]
pub(super) fn click_into_web_text_input(world: &mut IntegrationWorld) {
    run(world).set_focus(Focus::WebTextInput);
}

#[when("the wait time passes and the button still has focus")]
pub(super) fn wait_time_passes(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(is_left_in_clipboard, run.wait_time + REACTION)?;
    Ok(())
}

#[when("the user presses the hotkey")]
#[when("the user presses the hotkey again")]
pub(super) fn press_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).press_hotkey()
}
