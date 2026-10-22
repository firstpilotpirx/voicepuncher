//! The one-time offer to start at login, shown at the first Ready of the installed app.

use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSApplication};
use objc2_foundation::NSString;

use crate::adapters::main_queue::dispatch_to_main;
use crate::adapters::primary::settings::macos::login::{
    LoginOffer, OfferAnswer, NOT_NOW_TITLE, OFFER_INFO, OFFER_MESSAGE, START_AT_LOGIN_TITLE,
};
use crate::runtime::{RuntimeObserver, RuntimeState};

/// Asks on the main thread whenever the runtime reaches Ready and the offer is due.
pub struct LoginOfferObserver {
    offer: Arc<LoginOffer>,
}

impl LoginOfferObserver {
    pub fn new(offer: LoginOffer) -> Self {
        Self { offer: Arc::new(offer) }
    }
}

impl RuntimeObserver for LoginOfferObserver {
    fn state_changed(&self, state: &RuntimeState) {
        if !matches!(state, RuntimeState::Ready { .. }) {
            return;
        }
        let offer = Arc::clone(&self.offer);
        dispatch_to_main(move || {
            if offer.due() {
                ask(&offer);
            }
        });
    }
}

fn ask(offer: &LoginOffer) {
    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("login offer: asked for off the main thread");
        return;
    };
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(OFFER_MESSAGE));
    alert.setInformativeText(&NSString::from_str(OFFER_INFO));
    for title in [START_AT_LOGIN_TITLE, NOT_NOW_TITLE] {
        alert.addButtonWithTitle(&NSString::from_str(title));
    }
    NSApplication::sharedApplication(mtm).activate();
    let answer = if alert.runModal() == NSAlertFirstButtonReturn {
        OfferAnswer::StartAtLogin
    } else {
        OfferAnswer::NotNow
    };
    match offer.answer(answer) {
        Ok(()) => log::info!("login offer: {answer:?}"),
        Err(reason) => log::warn!("login offer: {answer:?} failed: {reason}"),
    }
}
