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
    VoiceControl,
    Refinement,
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
pub const CATALOG: [ModelFile; 6] = [
    ModelFile {
        name: "ggml-large-v3-turbo-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin",
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        size_bytes: 574_041_195,
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
        name: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin",
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        size_bytes: 487_601_967,
        purpose: "speech recognition — commands",
        kind: ModelKind::File,
        used_by: ModelUse::VoiceControl,
        variable: "VOICEPUNCHER_COMMAND_WHISPER_MODEL",
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
        name: "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2",
        sha256: "f170013b4716e41b62b9bfd809687c207cef798ef9bc6534d524e17af9b6561a",
        size_bytes: 17_626_723,
        purpose: "keyword spotting — voice control",
        kind: ModelKind::TarBz2 {
            folder: "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01",
        },
        used_by: ModelUse::VoiceControl,
        variable: "VOICEPUNCHER_KWS_MODEL_DIR",
    },
    ModelFile {
        name: "Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/ae44f08e1392f39c0e474af10c3ff8355c8b6688/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        sha256: "2fde00ce69dd4899c70d020845e2638353015bba0fdf161b3eb965f2bca4464e",
        size_bytes: 2_497_280_736,
        purpose: "text refinement",
        kind: ModelKind::File,
        used_by: ModelUse::Refinement,
        variable: "VOICEPUNCHER_LLM_MODEL",
    },
];

/// The parts of the app that are on, and so which models are needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelNeeds {
    pub voice_control: bool,
    pub refinement: bool,
}

impl ModelNeeds {
    /// Every model of the catalog.
    pub const ALL: ModelNeeds = ModelNeeds {
        voice_control: true,
        refinement: true,
    };

    fn includes(self, used_by: ModelUse) -> bool {
        match used_by {
            ModelUse::Dictation => true,
            ModelUse::VoiceControl => self.voice_control,
            ModelUse::Refinement => self.refinement,
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
    fn should_list_six_files_each_with_a_pinned_address_checksum_size_and_purpose() {
        // Then
        assert_eq!(CATALOG.len(), 6);
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
            url("ggml-large-v3-turbo-q5_0.bin"),
            format!("{whisper}/ggml-large-v3-turbo-q5_0.bin")
        );
        assert_eq!(url("ggml-small.bin"), format!("{whisper}/ggml-small.bin"));
        assert_eq!(
            url("ggml-silero-v5.1.2.bin"),
            "https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/ggml-silero-v5.1.2.bin"
        );
        assert_eq!(
            url("Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"),
            "https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/ae44f08e1392f39c0e474af10c3ff8355c8b6688/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"
        );
        assert_eq!(
            url("silero_vad.onnx"),
            format!("{sherpa}/asr-models/silero_vad.onnx")
        );
        assert_eq!(
            url("sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2"),
            format!(
                "{sherpa}/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2"
            )
        );
    }

    #[test]
    fn should_keep_the_checksums_the_model_scripts_held() {
        let sha = |name| by_name(name).unwrap().sha256;
        assert!(sha("ggml-large-v3-turbo-q5_0.bin").starts_with("394221709cd5"));
        assert!(sha("ggml-small.bin").starts_with("1be3a9b2"));
        assert!(sha("Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf").starts_with("2fde00ce"));
    }

    #[test]
    fn should_need_only_the_dictation_models_with_voice_control_and_refinement_off() {
        // When
        let files = needed(ModelNeeds {
            voice_control: false,
            refinement: false,
        });
        // Then
        assert_eq!(
            names(&files),
            vec!["ggml-large-v3-turbo-q5_0.bin", "ggml-silero-v5.1.2.bin"]
        );
    }

    #[test]
    fn should_need_every_file_with_everything_on() {
        assert_eq!(needed(ModelNeeds::ALL), CATALOG.to_vec());
    }

    #[test]
    fn should_need_the_refinement_model_only_with_refinement_on() {
        let files = needed(ModelNeeds {
            voice_control: false,
            refinement: true,
        });
        assert!(names(&files).contains(&"Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"));
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn should_place_the_archive_as_its_folder() {
        let archive = CATALOG
            .iter()
            .find(|f| matches!(f.kind, ModelKind::TarBz2 { .. }))
            .unwrap();
        assert_eq!(
            archive.placed_name(),
            "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
        );
    }

    #[test]
    fn should_add_up_the_sizes() {
        assert_eq!(total_bytes(&CATALOG), 3_578_079_573);
    }
}
