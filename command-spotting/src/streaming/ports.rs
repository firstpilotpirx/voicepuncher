//! What a streaming session needs from the models it runs: a gate that hears speech, and
//! recognizers that decode the audio while it streams in.

/// Hears whether there is speech in the audio, window by window.
pub trait SpeechGate: Send {
    /// Whether `window`, [`GATE_WINDOW`] samples at the microphone's level, holds speech.
    fn hears(&mut self, window: &[f32]) -> bool;
}

/// Samples per window the gate hears: Silero's window at 16 kHz.
pub const GATE_WINDOW: usize = 512;

/// A loaded streaming recognizer, shared by every connection.
pub trait StreamingModel: Send + Sync {
    /// Names it in log lines and the command history (`en`, `ru`).
    fn name(&self) -> &str;
    /// Whether the English words it writes in Cyrillic are respelled before matching.
    fn respelled(&self) -> bool;
    /// A new stream for one utterance.
    fn stream(&self) -> Box<dyn RecognizerStream + '_>;
}

/// One utterance's audio in a recognizer.
pub trait RecognizerStream: Send {
    fn accept(&mut self, samples: &[f32]);
    /// No more audio comes: what is left is decoded too.
    fn finish(&mut self);
    /// Decodes the audio accepted so far.
    fn decode(&mut self);
    /// The tokens decoded so far, each with its time in seconds of the stream.
    fn decoded(&self) -> Decoded;
}

/// Tokens and the stream time each was decoded at.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decoded {
    pub tokens: Vec<String>,
    pub times: Vec<f32>,
}
