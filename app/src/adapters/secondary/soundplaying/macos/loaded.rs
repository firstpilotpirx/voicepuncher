//! The sounds loaded once, as `NSSound`s, on a thread of their own that plays them when asked:
//! a cue plays at once instead of waiting for an `afplay` process to start.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_app_kit::NSSound;
use objc2_foundation::NSString;

enum Request {
    Load(PathBuf),
    Play(PathBuf),
}

/// The thread that holds the loaded sounds.
pub(super) struct LoadedSounds(Sender<Request>);

impl LoadedSounds {
    pub(super) fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("sounds".into())
            .spawn(move || serve(&rx));
        if let Err(e) = spawned {
            log::warn!("sound: no thread to play sounds on: {e}");
        }
        Self(tx)
    }

    /// Loads `path` now, so its first play does not wait for it.
    pub(super) fn load(&self, path: &Path) {
        let _ = self.0.send(Request::Load(path.to_owned()));
    }

    /// Plays `path`, from its start when it still plays.
    pub(super) fn play(&self, path: &Path) {
        if self.0.send(Request::Play(path.to_owned())).is_err() {
            super::MacosSoundPlayer::play_async(path);
        }
    }
}

fn serve(requests: &Receiver<Request>) {
    let mut sounds: HashMap<PathBuf, Option<Retained<NSSound>>> = HashMap::new();
    for request in requests {
        match request {
            Request::Load(path) => {
                sounds.entry(path).or_insert_with_key(|path| load(path));
            }
            Request::Play(path) => {
                let sound = sounds.entry(path).or_insert_with_key(|path| load(path));
                match sound {
                    Some(sound) => play(sound),
                    None => log::warn!("sound: nothing to play"),
                }
            }
        }
    }
}

/// The sound in the file at `path`; `None` when it cannot be read.
pub(super) fn load(path: &Path) -> Option<Retained<NSSound>> {
    let file = NSString::from_str(&path.to_string_lossy());
    let sound = NSSound::initWithContentsOfFile_byReference(NSSound::alloc(), &file, false);
    if sound.is_none() {
        log::warn!("sound: cannot load {}", path.display());
    }
    sound
}

fn play(sound: &NSSound) {
    if sound.isPlaying() {
        sound.stop();
    }
    if !sound.play() {
        log::warn!("sound: cannot play a loaded sound");
    }
}
