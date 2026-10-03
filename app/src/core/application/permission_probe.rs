use anyhow::Result;

#[cfg_attr(test, mockall::automock)]
pub trait PermissionProbe: Send {
    fn check_microphone(&self) -> Result<()>;
    fn check_accessibility(&self) -> Result<()>;
    fn check_input_monitoring(&self) -> Result<()>;
    fn parent_process_name(&self) -> Result<String>;
}
