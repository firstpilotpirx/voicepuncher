use std::sync::{Arc, Mutex};

use crate::core::domain::ServiceStatus;

use super::{rechecking, ServicesView};

#[derive(Default)]
struct Rechecks(Mutex<Vec<u16>>);

impl ServicesView for Rechecks {
    fn statuses(&self) -> Vec<ServiceStatus> {
        Vec::new()
    }

    fn restart(&self, _port: u16) {}

    fn recheck(&self, port: u16) {
        self.0.lock().unwrap().push(port);
    }
}

#[test]
fn should_check_again_every_copy_that_refuses_voice_control() {
    // Given
    let view = Arc::new(Rechecks::default());
    let refused = rechecking(Arc::clone(&view) as Arc<dyn ServicesView>);
    // When
    refused.report(49083);
    // Then
    assert_eq!(*view.0.lock().unwrap(), vec![49083]);
}
