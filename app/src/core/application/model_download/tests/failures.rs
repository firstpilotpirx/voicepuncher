//! Downloads that fail, retry, cancel, extract or use a mirror.

use super::*;

#[test]
pub(super) fn should_retry_a_connection_failure_after_2_4_and_8_seconds_then_report_no_internet() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let vad = model("vad.bin", VAD);
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
pub(super) fn should_resume_after_a_cut_connection() {
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
pub(super) fn should_fetch_nothing_when_the_space_is_short() {
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
pub(super) fn should_keep_the_part_on_cancel_and_resume_from_it_next_time() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let vad = model("vad.bin", VAD);
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
pub(super) fn should_extract_an_archive_once_it_is_verified() {
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
pub(super) fn should_fetch_from_the_mirror_when_one_is_named() {
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
pub(super) fn should_tell_all_present_at_once_when_nothing_is_missing() {
    // Given
    let vad = model("vad.bin", VAD);
    let store = Store::default().with_placed("vad.bin", VAD);
    // When
    let run = run(&[vad], &store, &Server::default());
    // Then
    assert_eq!(run.events, vec![DownloadEvent::AllPresent]);
}
