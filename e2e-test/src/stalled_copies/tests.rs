use super::*;

const LOG: &str = "\
[2026-10-22T17:58:52Z INFO] trigger \"restart trigger\" at 29.69-30.58s
[2026-10-22T17:58:53Z WARN] confirmation cut short after 1000ms on copy http://127.0.0.1:38088/v1: still decoding, nothing heard
[2026-10-22T17:58:53Z WARN] confirmation cut short after 1000ms on copy http://127.0.0.1:38087/v1: the copy does not answer, sending it to another copy
[2026-10-22T17:58:54Z INFO] audio waited for processing 1.2s
";

#[test]
fn should_name_each_confirmation_cut_short_with_its_copy_and_budget() {
    assert_eq!(
        cut_short(LOG),
        [
            "copy 38088: cut short after 1000 ms (still decoding, nothing heard)",
            "copy 38087: cut short after 1000 ms (the copy does not answer, sending it to another copy)",
        ]
    );
}

#[test]
fn should_name_nothing_when_no_confirmation_was_cut_short() {
    assert!(cut_short("[INFO] trigger \"start trigger\" at 1.0-1.4s\n").is_empty());
}

#[test]
fn should_read_only_what_the_log_got_after_the_watch_started() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let logs = home.path().join(".voicepuncher/logs");
    std::fs::create_dir_all(&logs).unwrap();
    let log = logs.join("voicepuncher-command-spotting-28082.log");
    std::fs::write(&log, LOG).unwrap();
    let watch = Watch::start("ws://127.0.0.1:28082", home.path());
    // When
    let later = "[WARN] confirmation cut short after 1300ms on copy http://127.0.0.1:38086/v1: still decoding, nothing heard\n";
    std::fs::write(&log, format!("{LOG}{later}")).unwrap();
    // Then
    assert_eq!(
        watch.cut_short(),
        ["copy 38086: cut short after 1300 ms (still decoding, nothing heard)"]
    );
}

#[test]
fn should_list_each_cut_short_copy_in_the_report_of_a_missing_cue() {
    let cut = cut_short(LOG);
    let report = report(&cut);
    assert!(
        report.contains("copy 38088: cut short after 1000 ms"),
        "{report}"
    );
    assert!(
        report.contains("copy 38087: cut short after 1000 ms"),
        "{report}"
    );
    assert_eq!(super::report(&[]), "no confirmation was cut short");
}
