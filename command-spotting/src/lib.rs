//! The command spotting service of voicepuncher: spots the voice control phrases and the
//! bounds of speech in audio streamed over a loopback WebSocket.

pub mod audio;
pub mod bench;
pub mod confirm;
pub mod conversions;
pub mod engine;
pub mod health;
pub mod history;
pub mod keywords;
pub mod listening;
pub mod matcher;
pub mod orderer;
pub mod outbox;
pub mod protocol;
pub mod server;
pub mod streaming;
pub mod worker;
