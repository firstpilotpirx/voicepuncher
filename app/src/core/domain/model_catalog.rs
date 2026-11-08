//! The model catalog: every model file the local services load, where it is downloaded from,
//! its sha256 and size, and which part of the app needs it.

/// How a downloaded file becomes what the services load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// The file itself.
    File,
    /// A `.tar.bz2` archive extracted into the models folder as `folder`, then deleted.
    TarBz2 { folder: &'static str },
}

/// The part of the app a model serves; it is needed only when that part is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelUse {
    Dictation,
    /// Voice control: the voice gate and the streaming recognizers.
    VoiceControl,
}

/// One model file of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    /// The downloaded file's name in the models folder.
    pub name: &'static str,
    pub url: &'static str,
    /// Of the downloaded file (the archive, for an archive).
    pub sha256: &'static str,
    /// Of the downloaded file, in bytes.
    pub size_bytes: u64,
    /// What it is for, in the user's words.
    pub purpose: &'static str,
    pub kind: ModelKind,
    pub used_by: ModelUse,
    /// The variable that names the file elsewhere, as the service catalog reads it.
    pub variable: &'static str,
}

impl ModelFile {
    /// The name the services find it under: the file, or the archive's folder.
    pub fn placed_name(&self) -> &'static str {
        match self.kind {
            ModelKind::File => self.name,
            ModelKind::TarBz2 { folder } => folder,
        }
    }
}

/// Every model file, in download order.
pub const CATALOG: [ModelFile; 5] = [
    ModelFile {
        name: "ggml-large-v3.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3.bin",
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        size_bytes: 3_095_033_483,
        purpose: "speech recognition — dictation",
        kind: ModelKind::File,
        used_by: ModelUse::Dictation,
        variable: "VOICEPUNCHER_WHISPER_MODEL",
    },
    ModelFile {
        name: "ggml-silero-v5.1.2.bin",
        url: "https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin",
        sha256: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
        size_bytes: 885_098,
        purpose: "voice activity detection — speech recognition",
        kind: ModelKind::File,
        used_by: ModelUse::Dictation,
        variable: "VOICEPUNCHER_VAD_MODEL",
    },
    ModelFile {
        name: "silero_vad.onnx",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
        sha256: "9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6",
        size_bytes: 643_854,
        purpose: "voice activity detection — voice control",
        kind: ModelKind::File,
        used_by: ModelUse::VoiceControl,
        variable: "VOICEPUNCHER_KWS_VAD_MODEL",
    },
    ModelFile {
        name: "sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25.tar.bz2",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25.tar.bz2",
        sha256: "0ae73a41cd51599dc7cac9ac083d9d35de53d762ca45923505fde47a3751814b",
        size_bytes: 463_945_198,
        purpose: "streaming speech recognition, English — voice control",
        kind: ModelKind::TarBz2 {
            folder: "sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25",
        },
        used_by: ModelUse::VoiceControl,
        variable: "VOICEPUNCHER_STREAMING_EN_MODEL_DIR",
    },
    ModelFile {
        name: "sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2",
        sha256: "cc2e99ed0c67cae8801170e7b7539b4cac00b716076af86f974bf5b888d9370c",
        size_bytes: 89_653_224,
        purpose: "streaming speech recognition, Russian — voice control",
        kind: ModelKind::TarBz2 {
            folder: "sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16",
        },
        used_by: ModelUse::VoiceControl,
        variable: "VOICEPUNCHER_STREAMING_RU_MODEL_DIR",
    },
];

/// The parts of the app that are on, and so which models are needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelNeeds {
    pub voice_control: bool,
}

impl ModelNeeds {
    /// Every model of the catalog: dictation and voice control.
    pub const ALL: Self = Self { voice_control: true };
    /// The dictation models alone: voice control off.
    pub const DICTATION: Self = Self { voice_control: false };

    const fn includes(self, used_by: ModelUse) -> bool {
        match used_by {
            ModelUse::Dictation => true,
            ModelUse::VoiceControl => self.voice_control,
        }
    }
}

/// The catalog files `needs` asks for, in download order.
pub fn needed(needs: ModelNeeds) -> Vec<ModelFile> {
    CATALOG
        .into_iter()
        .filter(|file| needs.includes(file.used_by))
        .collect()
}

/// The catalog file of that name.
pub fn by_name(name: &str) -> Option<ModelFile> {
    CATALOG.into_iter().find(|file| file.name == name)
}

/// Their sizes together.
pub fn total_bytes(files: &[ModelFile]) -> u64 {
    files.iter().map(|file| file.size_bytes).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(files: &[ModelFile]) -> Vec<&'static str> {
        files.iter().map(|file| file.name).collect()
    }

    #[test]
    fn should_list_five_files_each_with_a_pinned_address_checksum_size_and_purpose() {
        // Then
        assert_eq!(CATALOG.len(), 5);
        for file in CATALOG {
            assert!(file.url.starts_with("https://"), "{}", file.url);
            assert!(file.url.ends_with(file.name), "{}", file.url);
            assert_eq!(file.sha256.len(), 64, "{}", file.name);
            assert!(file.size_bytes > 0, "{}", file.name);
            assert!(!file.purpose.is_empty(), "{}", file.name);
        }
    }

    #[test]
    fn should_pin_the_huggingface_addresses_to_a_revision() {
        for file in CATALOG.iter().filter(|f| f.url.contains("huggingface.co")) {
            assert!(!file.url.contains("/resolve/main/"), "{}", file.url);
        }
    }

    #[test]
    fn should_fetch_each_file_from_its_pinned_address() {
        let url = |name| by_name(name).unwrap().url;
        let whisper =
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1";
        let sherpa = "https://github.com/k2-fsa/sherpa-onnx/releases/download";
        assert_eq!(
            url("ggml-large-v3.bin"),
            format!("{whisper}/ggml-large-v3.bin")
        );
        assert_eq!(
            url("ggml-silero-v5.1.2.bin"),
            "https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin"
        );
        assert_eq!(
            url("silero_vad.onnx"),
            format!("{sherpa}/asr-models/silero_vad.onnx")
        );
        assert_eq!(
            url("sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2"),
            format!(
                "{sherpa}/asr-models/sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2"
            )
        );
    }

    #[test]
    fn should_keep_the_checksums_the_model_scripts_held() {
        let sha = |name| by_name(name).unwrap().sha256;
        assert!(sha("ggml-large-v3.bin").starts_with("64d182b440b9"));
        assert!(sha("silero_vad.onnx").starts_with("9e2449e1"));
    }

    #[test]
    fn should_need_only_the_dictation_models_with_voice_control_off() {
        // When
        let files = needed(ModelNeeds::DICTATION);
        // Then
        assert_eq!(
            names(&files),
            vec!["ggml-large-v3.bin", "ggml-silero-v5.1.2.bin"]
        );
    }

    #[test]
    fn should_need_every_file_with_everything_on() {
        assert_eq!(needed(ModelNeeds::ALL), CATALOG.to_vec());
    }

    #[test]
    fn should_need_the_streaming_models_for_voice_control() {
        // When
        let files = needed(ModelNeeds::ALL);
        // Then
        assert_eq!(
            names(&files),
            vec![
                "ggml-large-v3.bin",
                "ggml-silero-v5.1.2.bin",
                "silero_vad.onnx",
                "sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25.tar.bz2",
                "sherpa-onnx-streaming-zipformer-small-ru-vosk-2025-08-16.tar.bz2",
            ]
        );
    }

    #[test]
    fn should_place_the_archive_as_its_folder() {
        let archive = CATALOG
            .iter()
            .find(|f| matches!(f.kind, ModelKind::TarBz2 { .. }))
            .unwrap();
        assert_eq!(
            archive.placed_name(),
            "sherpa-onnx-nemotron-speech-streaming-en-0.6b-160ms-int8-2026-04-25"
        );
    }

    #[test]
    fn should_add_up_the_sizes() {
        assert_eq!(total_bytes(&CATALOG), 3_650_160_857);
    }
}
