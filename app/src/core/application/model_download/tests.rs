use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use super::*;
use crate::core::domain::ModelUse;

/// A stand-in for sha256: equal bytes give equal text, and changed bytes other text.
fn digest(bytes: &[u8]) -> String {
    let mut sum: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        sum = (sum ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{sum:016x}{:048x}", bytes.len())
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn model(name: &str, bytes: &[u8]) -> ModelFile {
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

fn archive(name: &str, folder: &'static str, bytes: &[u8]) -> ModelFile {
    ModelFile { kind: ModelKind::TarBz2 { folder }, ..model(name, bytes) }
}

#[derive(Default)]
struct Disk {
    placed: HashMap<String, Vec<u8>>,
    parts: HashMap<String, Vec<u8>>,
    extracted: Vec<String>,
}

#[derive(Clone, Default)]
struct Store(Arc<Mutex<Disk>>);

impl Store {
    fn with_part(self, name: &str, bytes: &[u8]) -> Self {
        self.0
            .lock()
            .unwrap()
            .parts
            .insert(name.into(), bytes.to_vec());
        self
    }

    fn with_placed(self, name: &str, bytes: &[u8]) -> Self {
        self.0
            .lock()
            .unwrap()
            .placed
            .insert(name.into(), bytes.to_vec());
        self
    }

    fn placed(&self, name: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().placed.get(name).cloned()
    }

    fn part(&self, name: &str) -> Option<Vec<u8>> {
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

struct Sink {
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
enum Answer {
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
struct Server {
    answers: Arc<Mutex<HashMap<String, VecDeque<Answer>>>>,
    asked_from: Arc<Mutex<Vec<(String, u64)>>>,
}

impl Server {
    fn answers(self, file: &ModelFile, answers: Vec<Answer>) -> Self {
        self.answers
            .lock()
            .unwrap()
            .insert(file.url.into(), answers.into());
        self
    }

    fn requests(&self) -> Vec<(String, u64)> {
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
        let from = sink.len();
        self.asked_from.lock().unwrap().push((url.into(), from));
        let answer = {
            let mut answers = self.answers.lock().unwrap();
            let queue = answers.get_mut(url).expect("an address with answers");
            if queue.len() > 1 {
                queue.pop_front().unwrap()
            } else {
                queue.front().cloned().unwrap()
            }
        };
        let (bytes, limit, cancel_at) = match answer {
            Answer::Refuse => return Err(FetchError::Connection("refused".into())),
            Answer::Serve(bytes) => (bytes[from as usize..].to_vec(), None, None),
            Answer::IgnoreRange(bytes) => {
                sink.restart().map_err(FetchError::Disk)?;
                (bytes, None, None)
            }
            Answer::CutAfter(bytes, after) => (bytes[from as usize..].to_vec(), Some(after), None),
            Answer::CancelAfter(bytes, after) => {
                (bytes[from as usize..].to_vec(), None, Some(after))
            }
        };
        for (index, chunk) in bytes.chunks(1).enumerate() {
            if Some(index) == limit {
                return Err(FetchError::Connection("cut".into()));
            }
            if Some(index) == cancel_at {
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

struct Free(Option<u64>);

impl DiskSpace for Free {
    fn free_bytes(&self) -> Option<u64> {
        self.0
    }
}

#[derive(Clone)]
struct TestClock {
    now: Arc<Mutex<Instant>>,
    pauses: Arc<Mutex<Vec<Duration>>>,
}

impl TestClock {
    fn new() -> Self {
        Self { now: Arc::new(Mutex::new(Instant::now())), pauses: Arc::default() }
    }

    fn pauses(&self) -> Vec<Duration> {
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

struct Run {
    events: Vec<DownloadEvent>,
    summary: DownloadSummary,
}

fn download(
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

fn run(files: &[ModelFile], store: &Store, server: &Server) -> Run {
    download(files, store, server, None, &TestClock::new())
}

fn without_progress(events: &[DownloadEvent]) -> Vec<DownloadEvent> {
    events
        .iter()
        .filter(|e| !matches!(e, DownloadEvent::Progress { .. }))
        .cloned()
        .collect()
}

const DICTATION: &[u8] = b"dictation model bytes";
const VAD: &[u8] = b"vad";

#[test]
fn should_report_present_and_missing_files_with_the_missing_bytes_and_free_space() {
    // Given
    let (dictation, vad) = (model("dictation.bin", DICTATION), model("vad.bin", VAD));
    let store = Store::default().with_placed("vad.bin", VAD);
    let use_case = ModelDownload::new(
        vec![dictation, vad],
        Box::new(store),
        Box::new(Server::default()),
        Box::new(Free(Some(1_000))),
        Arc::new(TestClock::new()),
    );
    // When
    let report = use_case.check();
    // Then
    assert_eq!(
        report.rows.iter().map(|r| r.present).collect::<Vec<_>>(),
        vec![false, true]
    );
    assert_eq!(report.missing_bytes, DICTATION.len() as u64);
    assert_eq!(report.free_bytes, Some(1_000));
    assert!(!report.all_present());
}

#[test]
fn should_download_each_missing_file_in_turn_and_place_it() {
    // Given
    let (dictation, vad) = (model("dictation.bin", DICTATION), model("vad.bin", VAD));
    let store = Store::default();
    let server = Server::default()
        .answers(&dictation, vec![Answer::Serve(DICTATION.to_vec())])
        .answers(&vad, vec![Answer::Serve(VAD.to_vec())]);
    // When
    let run = run(&[dictation, vad], &store, &server);
    // Then
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
    assert_eq!(store.placed("vad.bin").as_deref(), Some(VAD));
    assert_eq!(store.part("dictation.bin"), None);
    assert_eq!(
        without_progress(&run.events),
        vec![
            DownloadEvent::Started("dictation.bin"),
            DownloadEvent::Verifying("dictation.bin"),
            DownloadEvent::Done("dictation.bin"),
            DownloadEvent::Started("vad.bin"),
            DownloadEvent::Verifying("vad.bin"),
            DownloadEvent::Done("vad.bin"),
            DownloadEvent::AllPresent,
        ]
    );
    assert!(run.summary.all_present);
}

#[test]
fn should_tell_the_progress_of_each_file() {
    // Given
    let vad = model("vad.bin", VAD);
    let server = Server::default().answers(&vad, vec![Answer::Serve(VAD.to_vec())]);
    // When
    let run = run(&[vad], &Store::default(), &server);
    // Then
    let received: Vec<u64> = run
        .events
        .iter()
        .filter_map(|e| match e {
            DownloadEvent::Progress { received, total, .. } => {
                assert_eq!(*total, 3);
                Some(*received)
            }
            _ => None,
        })
        .collect();
    assert_eq!(received, vec![1, 2, 3]);
}

#[test]
fn should_resume_from_the_bytes_of_the_part() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let store = Store::default().with_part("dictation.bin", &DICTATION[..10]);
    let server = Server::default().answers(&dictation, vec![Answer::Serve(DICTATION.to_vec())]);
    // When
    let run = run(&[dictation], &store, &server);
    // Then
    assert_eq!(server.requests(), vec![(dictation.url.to_string(), 10)]);
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
    assert_eq!(
        run.events[1],
        DownloadEvent::Verifying("dictation.bin"),
        "an existing part is hashed first"
    );
}

#[test]
fn should_read_the_file_from_its_start_when_the_server_ignores_the_resume() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let store = Store::default().with_part("dictation.bin", &DICTATION[..10]);
    let server =
        Server::default().answers(&dictation, vec![Answer::IgnoreRange(DICTATION.to_vec())]);
    // When
    run(&[dictation], &store, &server);
    // Then
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
}

#[test]
fn should_delete_a_part_larger_than_the_file_and_start_over() {
    // Given
    let vad = model("vad.bin", VAD);
    let store = Store::default().with_part("vad.bin", b"far too long a part");
    let server = Server::default().answers(&vad, vec![Answer::Serve(VAD.to_vec())]);
    // When
    run(&[vad], &store, &server);
    // Then
    assert_eq!(server.requests(), vec![(vad.url.to_string(), 0)]);
    assert_eq!(store.placed("vad.bin").as_deref(), Some(VAD));
}

#[test]
fn should_download_a_file_again_after_a_checksum_mismatch() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let mut corrupted = DICTATION.to_vec();
    corrupted[2] ^= 0xff;
    let store = Store::default().with_part("dictation.bin", &corrupted[..10]);
    let server = Server::default().answers(&dictation, vec![Answer::Serve(DICTATION.to_vec())]);
    // When
    let run = run(&[dictation], &store, &server);
    // Then
    assert!(run
        .events
        .contains(&DownloadEvent::MismatchRetrying("dictation.bin")));
    assert_eq!(
        server.requests(),
        vec![
            (dictation.url.to_string(), 10),
            (dictation.url.to_string(), 0)
        ]
    );
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
}

#[test]
fn should_fail_a_file_that_mismatches_twice_and_go_on_with_the_next() {
    // Given
    let (dictation, vad) = (model("dictation.bin", DICTATION), model("vad.bin", VAD));
    let server = Server::default()
        .answers(
            &dictation,
            vec![Answer::Serve(b"other bytes, same length".to_vec())],
        )
        .answers(&vad, vec![Answer::Serve(VAD.to_vec())]);
    let store = Store::default();
    // When
    let run = run(&[dictation, vad], &store, &server);
    // Then
    assert_eq!(
        run.summary.failed,
        vec![("dictation.bin", DownloadFailure::ChecksumMismatch)]
    );
    assert_eq!(server.requests().len(), 3);
    assert_eq!(store.placed("dictation.bin"), None);
    assert_eq!(store.part("dictation.bin"), None);
    assert_eq!(store.placed("vad.bin").as_deref(), Some(VAD));
    assert!(!run.summary.all_present);
}

#[test]
fn should_retry_a_connection_failure_after_2_4_and_8_seconds_then_report_no_internet() {
    // Given
    let (dictation, vad) = (model("dictation.bin", DICTATION), model("vad.bin", VAD));
    let store = Store::default().with_placed("vad.bin", VAD);
    let server = Server::default().answers(&dictation, vec![Answer::Refuse]);
    let clock = TestClock::new();
    // When
    let run = download(&[dictation, vad], &store, &server, None, &clock);
    // Then
    assert_eq!(server.requests().len(), 4);
    assert_eq!(clock.pauses(), RETRY_PAUSES.to_vec());
    assert_eq!(
        run.summary.failed,
        vec![("dictation.bin", DownloadFailure::NoInternet)]
    );
    assert_eq!(store.placed("vad.bin").as_deref(), Some(VAD));
}

#[test]
fn should_resume_after_a_cut_connection() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let server = Server::default().answers(
        &dictation,
        vec![
            Answer::CutAfter(DICTATION.to_vec(), 5),
            Answer::Serve(DICTATION.to_vec()),
        ],
    );
    let store = Store::default();
    // When
    run(&[dictation], &store, &server);
    // Then
    assert_eq!(
        server.requests(),
        vec![
            (dictation.url.to_string(), 0),
            (dictation.url.to_string(), 5)
        ]
    );
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
}

#[test]
fn should_fetch_nothing_when_the_space_is_short() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let server = Server::default().answers(&dictation, vec![Answer::Serve(DICTATION.to_vec())]);
    // When
    let run = download(
        &[dictation],
        &Store::default(),
        &server,
        Some(5),
        &TestClock::new(),
    );
    // Then
    assert_eq!(
        run.events,
        vec![DownloadEvent::NotEnoughSpace { needed: DICTATION.len() as u64, free: 5 }]
    );
    assert!(server.requests().is_empty());
}

#[test]
fn should_keep_the_part_on_cancel_and_resume_from_it_next_time() {
    // Given
    let (dictation, vad) = (model("dictation.bin", DICTATION), model("vad.bin", VAD));
    let server = Server::default()
        .answers(
            &dictation,
            vec![
                Answer::CancelAfter(DICTATION.to_vec(), 7),
                Answer::Serve(DICTATION.to_vec()),
            ],
        )
        .answers(&vad, vec![Answer::Serve(VAD.to_vec())]);
    let store = Store::default();
    // When
    let first = run(&[dictation, vad], &store, &server);
    // Then
    assert!(first.summary.cancelled);
    assert_eq!(
        first.events.last(),
        Some(&DownloadEvent::Cancelled("dictation.bin"))
    );
    assert_eq!(store.part("dictation.bin").map(|p| p.len()), Some(7));
    assert_eq!(store.placed("vad.bin"), None, "nothing after a cancel");
    // When
    let second = run(&[dictation, vad], &store, &server);
    // Then
    assert!(second.summary.all_present);
    assert_eq!(server.requests()[1], (dictation.url.to_string(), 7));
}

#[test]
fn should_extract_an_archive_once_it_is_verified() {
    // Given
    let kws = archive("kws.tar.bz2", "kws", b"archive");
    let server = Server::default().answers(&kws, vec![Answer::Serve(b"archive".to_vec())]);
    let store = Store::default();
    // When
    let run = run(&[kws], &store, &server);
    // Then
    assert!(run.summary.all_present);
    assert_eq!(store.0.lock().unwrap().extracted, vec!["kws".to_string()]);
    assert_eq!(store.placed("kws.tar.bz2"), None);
}

#[test]
fn should_fetch_from_the_mirror_when_one_is_named() {
    // Given
    let vad = model("vad.bin", VAD);
    let mirrored = ModelFile { url: "http://127.0.0.1:9/vad.bin", ..vad };
    let server = Server::default().answers(&mirrored, vec![Answer::Serve(VAD.to_vec())]);
    let use_case = ModelDownload::new(
        vec![vad],
        Box::new(Store::default()),
        Box::new(server.clone()),
        Box::new(Free(None)),
        Arc::new(TestClock::new()),
    )
    .with_mirror(Some("http://127.0.0.1:9/".into()));
    // When
    let summary = use_case.download_missing(&mut |_: &DownloadEvent| {}, &AtomicBool::new(false));
    // Then
    assert!(summary.all_present);
    assert_eq!(server.requests()[0].0, "http://127.0.0.1:9/vad.bin");
}

#[test]
fn should_tell_all_present_at_once_when_nothing_is_missing() {
    // Given
    let vad = model("vad.bin", VAD);
    let store = Store::default().with_placed("vad.bin", VAD);
    // When
    let run = run(&[vad], &store, &Server::default());
    // Then
    assert_eq!(run.events, vec![DownloadEvent::AllPresent]);
}
