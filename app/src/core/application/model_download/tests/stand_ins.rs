//! Stand-in store, part sink, server, disk space and clock, and the download run they drive.

use super::*;

/// A stand-in for sha256: equal bytes give equal text, and changed bytes other text.
pub(super) fn digest(bytes: &[u8]) -> String {
    let mut sum: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        sum = (sum ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{sum:016x}{:048x}", bytes.len())
}

pub(super) fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

pub(super) fn model(name: &str, bytes: &[u8]) -> ModelFile {
    ModelFile {
        name: leak(name.into()),
        url: leak(format!("https://models.example/{name}")),
        sha256: leak(digest(bytes)),
        size_bytes: bytes.len() as u64,
        purpose: "test",
        kind: ModelKind::File,
        used_by: ModelUse::Dictation,
        variable: "VOICEPUNCHER_TEST_MODEL",
    }
}

pub(super) fn archive(name: &str, folder: &'static str, bytes: &[u8]) -> ModelFile {
    ModelFile { kind: ModelKind::TarBz2 { folder }, ..model(name, bytes) }
}

#[derive(Default)]
pub(super) struct Disk {
    placed: HashMap<String, Vec<u8>>,
    parts: HashMap<String, Vec<u8>>,
    pub(crate) extracted: Vec<String>,
}

#[derive(Clone, Default)]
pub(super) struct Store(pub(super) Arc<Mutex<Disk>>);

impl Store {
    pub(crate) fn with_part(self, name: &str, bytes: &[u8]) -> Self {
        self.0
            .lock()
            .unwrap()
            .parts
            .insert(name.into(), bytes.to_vec());
        self
    }

    pub(crate) fn with_placed(self, name: &str, bytes: &[u8]) -> Self {
        self.0
            .lock()
            .unwrap()
            .placed
            .insert(name.into(), bytes.to_vec());
        self
    }

    pub(crate) fn placed(&self, name: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().placed.get(name).cloned()
    }

    pub(crate) fn part(&self, name: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().parts.get(name).cloned()
    }
}

impl ModelStore for Store {
    fn present(&self, file: &ModelFile) -> bool {
        let disk = self.0.lock().unwrap();
        match file.kind {
            ModelKind::File => disk
                .placed
                .get(file.name)
                .is_some_and(|bytes| bytes.len() as u64 == file.size_bytes),
            ModelKind::TarBz2 { folder } => disk.extracted.iter().any(|f| f == folder),
        }
    }

    fn part_len(&self, file: &ModelFile) -> u64 {
        self.part(file.name).map_or(0, |part| part.len() as u64)
    }

    fn delete_part(&self, file: &ModelFile) -> Result<(), String> {
        self.0.lock().unwrap().parts.remove(file.name);
        Ok(())
    }

    fn open_part(&self, file: &ModelFile) -> Result<Box<dyn PartSink>, String> {
        self.0
            .lock()
            .unwrap()
            .parts
            .entry(file.name.into())
            .or_default();
        Ok(Box::new(Sink {
            store: self.clone(),
            name: file.name.into(),
        }))
    }

    fn place(&self, file: &ModelFile) -> Result<(), String> {
        let mut disk = self.0.lock().unwrap();
        let part = disk.parts.remove(file.name).ok_or("no part")?;
        disk.placed.insert(file.name.into(), part);
        Ok(())
    }

    fn extract(&self, file: &ModelFile) -> Result<(), String> {
        let mut disk = self.0.lock().unwrap();
        disk.placed.remove(file.name).ok_or("no archive")?;
        disk.extracted.push(file.placed_name().into());
        Ok(())
    }
}

pub(super) struct Sink {
    store: Store,
    name: String,
}

impl Sink {
    fn bytes(&self) -> Vec<u8> {
        self.store.part(&self.name).unwrap_or_default()
    }
}

impl PartSink for Sink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        let mut disk = self.store.0.lock().unwrap();
        disk.parts
            .entry(self.name.clone())
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }

    fn restart(&mut self) -> Result<(), String> {
        self.store
            .0
            .lock()
            .unwrap()
            .parts
            .insert(self.name.clone(), Vec::new());
        Ok(())
    }

    fn len(&self) -> u64 {
        self.bytes().len() as u64
    }

    fn sha256_hex(&mut self) -> String {
        digest(&self.bytes())
    }
}

/// What the server does with one request.
#[derive(Clone)]
pub(crate) enum Answer {
    /// Sends the bytes from the part's length on.
    Serve(Vec<u8>),
    /// Sends the whole file whatever the part holds.
    IgnoreRange(Vec<u8>),
    /// Sends `after` bytes from the part's length on, then the connection breaks.
    CutAfter(Vec<u8>, usize),
    /// Sends `after` bytes, then the user chooses Cancel.
    CancelAfter(Vec<u8>, usize),
    Refuse,
}

/// The answers per address, the last one repeated; the ranges asked for.
#[derive(Clone, Default)]
pub(super) struct Server {
    answers: Arc<Mutex<HashMap<String, VecDeque<Answer>>>>,
    asked_from: crate::test_doubles::ModelRequests,
}

impl Server {
    /// The next answer for `url`; the last one repeats.
    fn next_answer(&self, url: &str) -> Answer {
        let mut answers = self.answers.lock().unwrap();
        let queue = answers.get_mut(url).expect("an address with answers");
        if queue.len() > 1 {
            queue.pop_front().unwrap()
        } else {
            queue.front().cloned().unwrap()
        }
    }

    pub(crate) fn answers(self, file: &ModelFile, answers: Vec<Answer>) -> Self {
        self.answers
            .lock()
            .unwrap()
            .insert(file.url.into(), answers.into());
        self
    }

    pub(crate) fn requests(&self) -> Vec<(String, u64)> {
        self.asked_from.lock().unwrap().clone()
    }
}

impl ModelFetcher for Server {
    fn fetch(
        &self,
        url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let from = crate::test_doubles::note_request(&self.asked_from, url, sink);
        let rest = usize::try_from(from).unwrap();
        let (bytes, limit, cancel_at) = match self.next_answer(url) {
            Answer::Refuse => return Err(FetchError::Connection("refused".into())),
            Answer::Serve(bytes) => (bytes[rest..].to_vec(), None, None),
            Answer::IgnoreRange(bytes) => {
                sink.restart().map_err(FetchError::Disk)?;
                (bytes, None, None)
            }
            Answer::CutAfter(bytes, after) => (bytes[rest..].to_vec(), Some(after), None),
            Answer::CancelAfter(bytes, after) => (bytes[rest..].to_vec(), None, Some(after)),
        };
        let reply = Reply { bytes, limit, cancel_at };
        reply.send(sink, received, cancel)
    }
}

/// The bytes one fetch sends, cut or cancelled after a number of them.
struct Reply {
    bytes: Vec<u8>,
    limit: Option<usize>,
    cancel_at: Option<usize>,
}

impl Reply {
    /// Writes the bytes one at a time, as a slow connection delivers them.
    fn send(
        self,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        for (index, chunk) in self.bytes.chunks(1).enumerate() {
            if Some(index) == self.limit {
                return Err(FetchError::Connection("cut".into()));
            }
            if Some(index) == self.cancel_at {
                cancel.store(true, Ordering::SeqCst);
            }
            if cancel.load(Ordering::SeqCst) {
                return Err(FetchError::Cancelled);
            }
            sink.write(chunk).map_err(FetchError::Disk)?;
            received(sink.len());
        }
        Ok(())
    }
}

pub(super) struct Free(pub(crate) Option<u64>);

impl DiskSpace for Free {
    fn free_bytes(&self) -> Option<u64> {
        self.0
    }
}

#[derive(Clone)]
pub(super) struct TestClock {
    now: Arc<Mutex<Instant>>,
    pauses: Arc<Mutex<Vec<Duration>>>,
}

impl TestClock {
    pub(crate) fn new() -> Self {
        Self { now: Arc::new(Mutex::new(Instant::now())), pauses: Arc::default() }
    }

    pub(crate) fn pauses(&self) -> Vec<Duration> {
        self.pauses.lock().unwrap().clone()
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap()
    }

    fn sleep(&self, pause: Duration) {
        self.pauses.lock().unwrap().push(pause);
        *self.now.lock().unwrap() += pause;
    }
}

pub(super) struct Run {
    pub(crate) events: Vec<DownloadEvent>,
    pub(crate) summary: DownloadSummary,
}

pub(super) fn download(
    files: &[ModelFile],
    store: &Store,
    server: &Server,
    free: Option<u64>,
    clock: &TestClock,
) -> Run {
    let use_case = ModelDownload::new(
        files.to_vec(),
        Box::new(store.clone()),
        Box::new(server.clone()),
        Box::new(Free(free)),
        Arc::new(clock.clone()),
    );
    let mut events = Vec::new();
    let summary = use_case.download_missing(
        &mut |event: &DownloadEvent| events.push(event.clone()),
        &AtomicBool::new(false),
    );
    Run { events, summary }
}

pub(super) fn run(files: &[ModelFile], store: &Store, server: &Server) -> Run {
    download(files, store, server, None, &TestClock::new())
}

pub(super) fn without_progress(events: &[DownloadEvent]) -> Vec<DownloadEvent> {
    events
        .iter()
        .filter(|e| !matches!(e, DownloadEvent::Progress { .. }))
        .cloned()
        .collect()
}

pub(super) const DICTATION: &[u8] = b"dictation model bytes";

pub(super) const VAD: &[u8] = b"vad";
