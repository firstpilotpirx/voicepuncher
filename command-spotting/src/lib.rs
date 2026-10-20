//! The command spotting service of voicepuncher: spots the voice control phrases and the
//! bounds of speech in audio streamed over a loopback WebSocket.

pub mod confirm;
pub mod engine;
pub mod health;
pub mod keywords;
pub mod matcher;
pub mod orderer;
pub mod outbox;
pub mod protocol;
pub mod server;
pub mod worker;

#[cfg(test)]
mod test_log;
