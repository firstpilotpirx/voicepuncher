#[cfg_attr(test, mockall::automock)]
pub trait VisualIndicator: Send {
    fn show_recording(&self);
    fn show_processing(&self);
    fn show_waiting_for_focus(&self);
    fn hide(&self);
}
