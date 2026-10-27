//! Writes the command history off the path that reports commands: one thread per engine
//! takes the finished entries, writes each and keeps the folder within its size limit.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;

use anyhow::Result;

use super::entry::{write_files, Entry};

/// Where the command history is kept and how much of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCfg {
    pub dir: PathBuf,
    pub max_size_bytes: u64,
}

enum Message {
    Keep(Box<Entry>),
    Flush(Sender<()>),
}

/// The command history of an engine: entries handed to it are written by its own thread.
pub struct CommandHistory {
    sender: Mutex<Option<Sender<Message>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl CommandHistory {
    pub fn start(cfg: HistoryCfg) -> Result<Self> {
        let (sender, messages) = channel::<Message>();
        let thread = std::thread::Builder::new()
            .name("command-history".to_owned())
            .spawn(move || {
                for message in messages {
                    match message {
                        Message::Keep(entry) => keep_or_warn(&cfg, &entry),
                        Message::Flush(done) => drop(done.send(())),
                    }
                }
            })?;
        Ok(Self { sender: Mutex::new(Some(sender)), thread: Mutex::new(Some(thread)) })
    }

    /// Hands `entry` to the writer thread; never waits for the disk.
    pub fn keep(&self, entry: Entry) {
        self.send(Message::Keep(Box::new(entry)));
    }

    /// Waits until every entry handed over so far is written or given up.
    pub fn flush(&self) {
        let (done, written) = channel();
        self.send(Message::Flush(done));
        written.recv().ok();
    }

    fn send(&self, message: Message) {
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(sender) = sender.as_ref() {
            drop(sender.send(message));
        }
    }
}

impl Drop for CommandHistory {
    fn drop(&mut self) {
        let sender = self
            .sender
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        drop(sender.take());
        let thread = self
            .thread
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(thread) = thread.take() {
            drop(thread.join());
        }
    }
}

fn keep_or_warn(cfg: &HistoryCfg, entry: &Entry) {
    if let Err(e) = keep(cfg, entry) {
        log::warn!(
            "command history: the utterance {} was not kept: {e:#}",
            entry.folder_base()
        );
    }
}

/// Writes `entry` into the folder of `cfg`, then removes the oldest entries past its limit.
pub fn keep(cfg: &HistoryCfg, entry: &Entry) -> Result<String> {
    let name = voicepuncher_entry_folders::write_entry(&cfg.dir, &entry.folder_base(), |folder| {
        write_files(folder, entry)
    })?;
    log::info!("command history: kept {name} ({:?})", entry.outcome);
    voicepuncher_entry_folders::prune(&cfg.dir, cfg.max_size_bytes, &name)?;
    Ok(name)
}

#[cfg(test)]
mod tests;
