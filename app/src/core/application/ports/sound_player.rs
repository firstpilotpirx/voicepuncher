#[cfg_attr(test, mockall::automock)]
pub trait SoundPlayer: Send {
    fn play_start(&self);
    fn play_stop(&self);
}
