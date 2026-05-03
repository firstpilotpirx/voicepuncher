use anyhow::Result;

use crate::core::domain::AudioBuffer16k;

#[cfg_attr(test, mockall::automock)]
pub trait AudioRecorder: Send {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<AudioBuffer16k>;
}
