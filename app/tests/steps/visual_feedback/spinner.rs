//! Finds the spinner and the label on the real overlay by walking the app's windows, so no
//! inspector is added to the overlay itself.

use anyhow::{ensure, Context, Result};
use cucumber::then;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSProgressIndicator, NSTextField, NSView};
use objc2_foundation::NSRect;

use crate::IntegrationWorld;

/// The label the speech recognition stage shows.
const LABEL: &str = "Recognizing";

/// Every shown view of the app's visible windows, `view` and the views inside it first.
fn shown_views(view: &Retained<NSView>, into: &mut Vec<Retained<NSView>>) {
    if !view.isHiddenOrHasHiddenAncestor() && view.window().is_some_and(|w| w.isVisible()) {
        into.push(view.clone());
    }
    for sub in &view.subviews() {
        shown_views(&sub, into);
    }
}

/// A view's frame in its window's coordinates.
fn in_window(view: &NSView) -> NSRect {
    view.convertRect_toView(view.bounds(), None)
}

#[then("a spinner turns beside them")]
fn spinner_turns(_world: &mut IntegrationWorld) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the harness runs its steps on the main thread")?;
    let mut views = Vec::new();
    for window in &NSApplication::sharedApplication(mtm).windows() {
        if let Some(content) = window.contentView() {
            shown_views(&content, &mut views);
        }
    }
    let spinner = views
        .iter()
        .find_map(|v| v.downcast_ref::<NSProgressIndicator>().map(|p| (v, p)))
        .context("no spinner shows on the overlay")?;
    ensure!(
        spinner.1.isIndeterminate(),
        "the spinner fills instead of turning"
    );
    let label = views
        .iter()
        .find(|v| {
            v.downcast_ref::<NSTextField>()
                .is_some_and(|t| t.stringValue().to_string() == LABEL)
        })
        .context("the label does not show")?;
    let (spun, labelled) = (in_window(spinner.0), in_window(label));
    ensure!(
        spun.origin.x >= labelled.origin.x + labelled.size.width,
        "the spinner {spun:?} is not right of the label {labelled:?}"
    );
    Ok(())
}
