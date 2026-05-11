use crate::core::domain::AudioLevel;

#[cfg_attr(test, mockall::automock)]
pub trait AudioLevelSource: Send + Sync {
    fn read_level(&self) -> AudioLevel;
}
