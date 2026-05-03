use std::env;

use tempfile::tempdir;
use voicepuncher::adapters::primary::setup::cli::model_downloader::ModelDownloader;

#[test]
fn should_fail_on_unreachable_url() {
    // Given
    let dir = tempdir().unwrap();
    let dest = dir.path().join("model.bin");
    // When
    let result = ModelDownloader::download("http://127.0.0.1:19999/nonexistent.bin", &dest);
    // Then
    assert!(result.is_err(), "unreachable host must return an error");
    assert!(
        !dest.exists(),
        "final file must not exist after failed download"
    );
}

#[test]
fn should_download_small_file_when_env_set() {
    // Given
    if env::var("VOICEPUNCHER_HF_DOWNLOAD").is_err() {
        return;
    }
    let dir = tempdir().unwrap();
    let dest = dir.path().join("readme.md");
    // When — use a tiny stable public file on Hugging Face
    let result = ModelDownloader::download(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/README.md",
        &dest,
    );
    // Then
    assert!(result.is_ok(), "download should succeed: {result:?}");
    assert!(dest.exists());
    assert!(dest.metadata().unwrap().len() > 0);
}
