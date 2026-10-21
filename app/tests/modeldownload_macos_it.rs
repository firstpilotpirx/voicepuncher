//! The Download window's model fed by a real download: the models folder store in a temporary
//! folder, a stand-in host, every event applied as the window applies it.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use voicepuncher::adapters::primary::modeldownload::macos::mapping::{
    status_text, MainButton, RowStatus, WindowModel,
};
use voicepuncher::adapters::secondary::modelfetch::fs::FsModelStore;
use voicepuncher::core::application::{
    Clock, DiskSpace, DownloadEvent, FetchError, ModelDownload, ModelFetcher, PartSink,
};
use voicepuncher::core::domain::{ModelFile, CATALOG};

const BODY: &[u8] = b"a small model file, served whole";

struct Host;

impl ModelFetcher for Host {
    fn fetch(
        &self,
        _url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        _cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let from = sink.len() as usize;
        sink.write(&BODY[from..]).map_err(FetchError::Disk)?;
        received(sink.len());
        Ok(())
    }
}

struct Plenty;

impl DiskSpace for Plenty {
    fn free_bytes(&self) -> Option<u64> {
        Some(1 << 40)
    }
}

struct NoPause;

impl Clock for NoPause {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, _pause: Duration) {}
}

fn small_file() -> ModelFile {
    let sha256: String = Sha256::digest(BODY)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    ModelFile {
        name: "small.bin",
        url: "https://models.test/small.bin",
        sha256: Box::leak(sha256.into_boxed_str()),
        size_bytes: BODY.len() as u64,
        ..CATALOG[1]
    }
}

fn download(folder: &Path, home: &Path) -> ModelDownload {
    ModelDownload::new(
        vec![small_file()],
        Box::new(FsModelStore::new(
            folder.to_path_buf(),
            home,
            HashMap::new(),
        )),
        Box::new(Host),
        Box::new(Plenty),
        Arc::new(NoPause),
    )
}

#[test]
fn should_show_a_missing_file_downloaded_placed_and_done() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let folder = home.path().join("models");
    let first = download(&folder, home.path());
    let mut model = WindowModel::new(&first.check(), &folder);
    assert_eq!(status_text(&model.rows[0].status), "Missing");
    assert_eq!(model.main_button(), MainButton::Download);
    // When
    model.started();
    first.download_missing(
        &mut |event: &DownloadEvent| model.apply(event),
        &AtomicBool::new(false),
    );
    model.ended();
    model.rechecked(&download(&folder, home.path()).check());
    // Then
    assert_eq!(model.rows[0].status, RowStatus::Present);
    assert!(model.done_enabled());
    assert!(folder.join("small.bin").is_file());
}
