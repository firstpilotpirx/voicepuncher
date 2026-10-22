//! The method bodies the test doubles of the ports share.
//!
//! A recorder, an overlay and a spotter that take no part in voice control. Each macro is
//! invoked inside the `impl` of its port, so every double still names what it does.

/// The three live-editing methods of a recorder that records whole buffers only: it cuts
/// nothing, takes no lead-in and keeps no recording. Invoked inside its `impl AudioRecorder`.
#[macro_export]
macro_rules! records_whole_buffers_only {
    () => {
        fn truncate_at(&mut self, _cut: ::std::time::Instant) {}
        fn begin_with(&mut self, _lead_in: $crate::core::domain::AudioBuffer16k) {}
        fn recording(&mut self) -> ::std::option::Option<$crate::core::domain::AudioBuffer16k> {
            ::std::option::Option::None
        }
    };
}

/// The voice control signs of an overlay that shows none and a hidden live transcript.
/// Invoked inside its `impl VisualIndicator`.
#[macro_export]
macro_rules! shows_no_voice_control_signs {
    () => {
        fn show_command_sign(&self, _command: $crate::core::domain::VoiceCommand) {}
        fn show_commands_unavailable(&self) {}
        fn hide_commands_unavailable(&self) {}
        fn live_transcript_view(
            &self,
        ) -> ::std::boxed::Box<
            dyn $crate::core::application::live_transcript_view::LiveTranscriptView,
        > {
            ::std::boxed::Box::new(
                $crate::core::application::live_transcript_view::HiddenLiveTranscript,
            )
        }
    };
}

/// `heard_since` of a spotter that holds no sound: an empty buffer. Invoked inside its
/// `impl CommandSpotter`.
#[macro_export]
macro_rules! holds_no_heard_sound {
    () => {
        fn heard_since(
            &self,
            _since: ::std::time::Instant,
        ) -> $crate::core::domain::AudioBuffer16k {
            $crate::core::domain::AudioBuffer16k::new(::std::vec::Vec::new())
        }
    };
}
