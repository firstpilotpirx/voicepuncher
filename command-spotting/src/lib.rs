//! The command spotting service of voicepuncher: spots the voice control phrases and the
//! bounds of speech in audio streamed over a loopback WebSocket.

pub mod engine;
pub mod keywords;
pub mod orderer;
pub mod protocol;
pub mod server;
