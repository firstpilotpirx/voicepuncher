use anyhow::Result;

use crate::core::domain::FocusedElement;

/// Reports the element that has keyboard focus; `None` when the system names none.
#[cfg_attr(test, mockall::automock)]
pub trait FocusProbe: Send {
    fn focused_element(&self) -> Result<Option<FocusedElement>>;
}
