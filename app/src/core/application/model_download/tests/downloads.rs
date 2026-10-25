//! Downloads that find, fetch, resume and verify the model files.

use super::*;

#[test]
pub(super) fn should_report_present_and_missing_files_with_the_missing_bytes_and_free_space() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let vad = model("vad.bin", VAD);
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
pub(super) fn should_download_each_missing_file_in_turn_and_place_it() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let vad = model("vad.bin", VAD);
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
pub(super) fn should_tell_the_progress_of_each_file() {
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
pub(super) fn should_resume_from_the_bytes_of_the_part() {
    // Given
    let (dictation, store, server) =
        dictation_with_part(&DICTATION[..10], Answer::Serve(DICTATION.to_vec()));
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
pub(super) fn should_read_the_file_from_its_start_when_the_server_ignores_the_resume() {
    // Given
    let (dictation, store, server) =
        dictation_with_part(&DICTATION[..10], Answer::IgnoreRange(DICTATION.to_vec()));
    // When
    run(&[dictation], &store, &server);
    // Then
    assert_eq!(store.placed("dictation.bin").as_deref(), Some(DICTATION));
}

#[test]
pub(super) fn should_delete_a_part_larger_than_the_file_and_start_over() {
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
pub(super) fn should_download_a_file_again_after_a_checksum_mismatch() {
    // Given
    let mut corrupted = DICTATION.to_vec();
    corrupted[2] ^= 0xff;
    let (dictation, store, server) =
        dictation_with_part(&corrupted[..10], Answer::Serve(DICTATION.to_vec()));
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
pub(super) fn should_fail_a_file_that_mismatches_twice_and_go_on_with_the_next() {
    // Given
    let dictation = model("dictation.bin", DICTATION);
    let vad = model("vad.bin", VAD);
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

/// The dictation model, a store holding `part` of it and a server that gives `answer` for it.
fn dictation_with_part(part: &[u8], answer: Answer) -> (ModelFile, Store, Server) {
    let dictation = model("dictation.bin", DICTATION);
    let store = Store::default().with_part("dictation.bin", part);
    let server = Server::default().answers(&dictation, vec![answer]);
    (dictation, store, server)
}
