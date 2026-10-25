//! The download run with its stand-in host, disk and clock.

use super::{
    bail, Arc, AtomicBool, Clock, Context, Digest, DiskSpace, DownloadEvent, DownloadSummary,
    Duration, FetchError, FsModelStore, HashMap, Instant, ModelDownload, ModelFetcher, ModelFile,
    ModelKind, ModelUse, ModelsReport, Mutex, Ordering, PartSink, PathBuf, Result, Sha256, TempDir,
};
use std::fmt::Write as _;

/// How the stand-in host answers for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    Serve,
    /// Bytes that never match the checksum.
    Corrupt,
    Refuse,
    /// Stops after this many bytes, as Cancel does.
    CancelAfter(usize),
}

/// Every request: the file and the byte it asked to start from.
pub(super) type Requests = voicepuncher::test_doubles::ModelRequests;

#[derive(Clone)]
pub(crate) struct Host {
    bodies: HashMap<String, Vec<u8>>,
    answers: Arc<Mutex<HashMap<String, Answer>>>,
    pub(crate) requests: Requests,
}

impl ModelFetcher for Host {
    fn fetch(
        &self,
        url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let from = voicepuncher::test_doubles::note_request(&self.requests, url, sink);
        let body = self.bodies.get(url).cloned().unwrap_or_default();
        let answer = self
            .answers
            .lock()
            .unwrap()
            .get(url)
            .copied()
            .unwrap_or(Answer::Serve);
        let rest = &body[usize::try_from(from).unwrap()..];
        match answer {
            Answer::Refuse => Err(FetchError::Connection("connection refused".into())),
            Answer::Corrupt => {
                let wrong: Vec<u8> = rest.iter().map(|byte| byte ^ 0xff).collect();
                sink.write(&wrong).map_err(FetchError::Disk)?;
                received(sink.len());
                Ok(())
            }
            Answer::CancelAfter(bytes) => {
                sink.write(&rest[..bytes]).map_err(FetchError::Disk)?;
                received(sink.len());
                cancel.store(true, Ordering::SeqCst);
                Err(FetchError::Cancelled)
            }
            Answer::Serve => {
                for chunk in rest.chunks(64) {
                    sink.write(chunk).map_err(FetchError::Disk)?;
                    received(sink.len());
                }
                Ok(())
            }
        }
    }
}

pub(super) struct Disk(Option<u64>);

impl DiskSpace for Disk {
    fn free_bytes(&self) -> Option<u64> {
        self.0
    }
}

/// Records its pauses without sleeping.
#[derive(Clone, Default)]
pub(crate) struct PauseClock(pub(crate) Arc<Mutex<Vec<Duration>>>);

impl Clock for PauseClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, pause: Duration) {
        self.0.lock().unwrap().push(pause);
    }
}

/// What a download scenario shares with its steps.
pub struct DownloadRun {
    home: TempDir,
    files: Vec<ModelFile>,
    pub(crate) host: Host,
    pub(crate) free: Option<u64>,
    pub(crate) clock: PauseClock,
    pub(crate) events: Vec<DownloadEvent>,
    pub(crate) summary: Option<DownloadSummary>,
}

impl std::fmt::Debug for DownloadRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadRun")
            .field("events", &self.events)
            .finish_non_exhaustive()
    }
}

impl Default for DownloadRun {
    fn default() -> Self {
        Self {
            home: tempfile::tempdir().expect("a temporary home"),
            files: Vec::new(),
            host: Host {
                bodies: HashMap::new(),
                answers: Arc::default(),
                requests: Arc::default(),
            },
            free: None,
            clock: PauseClock::default(),
            events: Vec::new(),
            summary: None,
        }
    }
}

pub(super) fn leaked(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

impl DownloadRun {
    pub(crate) fn folder(&self) -> PathBuf {
        self.home.path().join("models")
    }

    /// A catalog file of `size` bytes, served by the host.
    pub(crate) fn add_file(&mut self, size: usize) -> ModelFile {
        let index = self.files.len();
        let body: Vec<u8> = (0..size)
            .map(|i| u8::try_from((i * 7 + index) % 256).unwrap())
            .collect();
        let name = leaked(format!("model-{index}.bin"));
        let file = ModelFile {
            name,
            url: leaked(format!("https://models.test/{name}")),
            sha256: leaked(sha256_hex(&body)),
            size_bytes: size as u64,
            purpose: "test model",
            kind: ModelKind::File,
            used_by: ModelUse::Dictation,
            variable: leaked(format!("VOICEPUNCHER_TEST_MODEL_{index}")),
        };
        self.host.bodies.insert(file.url.to_string(), body);
        self.files.push(file);
        file
    }

    /// A catalog file of `bytes` that is never fetched.
    pub(crate) fn add_unfetched(&mut self, bytes: u64) {
        let index = self.files.len();
        self.files.push(ModelFile {
            name: leaked(format!("big-{index}.bin")),
            url: leaked(format!("https://models.test/big-{index}.bin")),
            sha256: "0",
            size_bytes: bytes,
            purpose: "test model",
            kind: ModelKind::File,
            used_by: ModelUse::Dictation,
            variable: leaked(format!("VOICEPUNCHER_TEST_MODEL_{index}")),
        });
    }

    pub(crate) fn answer(&self, file: &ModelFile, answer: Answer) {
        self.host
            .answers
            .lock()
            .unwrap()
            .insert(file.url.to_string(), answer);
    }

    pub(crate) fn body(&self, file: &ModelFile) -> &[u8] {
        &self.host.bodies[file.url]
    }

    pub(crate) fn place(&self, file: &ModelFile) {
        std::fs::create_dir_all(self.folder()).unwrap();
        std::fs::write(self.folder().join(file.name), self.body(file)).unwrap();
    }

    pub(crate) fn write_part(&self, file: &ModelFile, bytes: &[u8]) {
        std::fs::create_dir_all(self.folder()).unwrap();
        std::fs::write(self.folder().join(format!("{}.part", file.name)), bytes).unwrap();
    }

    fn store(&self) -> FsModelStore {
        FsModelStore::new(self.folder(), self.home.path(), HashMap::new())
    }

    fn download(&self) -> ModelDownload {
        ModelDownload::new(
            self.files.clone(),
            Box::new(self.store()),
            Box::new(self.host.clone()),
            Box::new(Disk(self.free)),
            Arc::new(self.clock.clone()),
        )
    }

    pub(crate) fn check(&self) -> ModelsReport {
        self.download().check()
    }

    pub(crate) fn run_download(&mut self) {
        let mut events = Vec::new();
        let summary = self.download().download_missing(
            &mut |event: &DownloadEvent| events.push(event.clone()),
            &AtomicBool::new(false),
        );
        self.events.extend(events);
        self.summary = Some(summary);
    }

    pub(crate) fn requests(&self) -> Vec<(String, u64)> {
        self.host.requests.lock().unwrap().clone()
    }

    pub(crate) fn requests_of(&self, file: &ModelFile) -> Vec<u64> {
        self.requests()
            .into_iter()
            .filter(|(url, _)| url == file.url)
            .map(|(_, from)| from)
            .collect()
    }

    pub(crate) fn placed_matches(&self, file: &ModelFile) -> Result<()> {
        let placed = std::fs::read(self.folder().join(file.name))
            .with_context(|| format!("{} is not placed", file.name))?;
        if sha256_hex(&placed) != file.sha256 {
            bail!("{} was placed with another checksum", file.name);
        }
        Ok(())
    }

    pub(crate) fn file(&self, index: usize) -> ModelFile {
        self.files[index]
    }
}
