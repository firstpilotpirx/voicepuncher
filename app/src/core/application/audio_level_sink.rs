use crate::core::domain::AudioLevel;

#[cfg_attr(test, mockall::automock)]
pub trait AudioLevelSink: Send + Sync {
    fn write_level(&self, level: AudioLevel);
}
