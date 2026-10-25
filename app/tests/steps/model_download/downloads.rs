//! Steps of downloading, resuming, verifying and retrying.

use super::{
    app_misses_dictation_model, bail, download, given, runtime_run, then, when, Answer,
    DownloadEvent, DownloadFailure, Duration, IntegrationWorld, Result, GB,
};

// --- Download fetches, verifies and places each file

#[given("two needed files are missing")]
pub(super) fn two_missing(world: &mut IntegrationWorld) {
    let run = download(world);
    let present = run.add_file(100);
    run.place(&present);
    run.add_file(300);
    run.add_file(500);
}

#[when("the user chooses Download")]
#[when("the user chooses Download again")]
#[when("the file is downloaded")]
pub(super) fn user_chooses_download(world: &mut IntegrationWorld) {
    download(world).run_download();
}

#[then(
    "each file is fetched in turn with its progress shown, its checksum verified, and moved to its name in the models folder"
)]
fn each_fetched_in_turn(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let first = run.file(1);
    let second = run.file(2);
    if started_and_done(&run.events) != [first.name, first.name, second.name, second.name] {
        bail!("not fetched in turn: {:?}", run.events);
    }
    for file in [first, second] {
        if !all_received_shown(&run.events, &file) {
            bail!("no progress shown for {}: {:?}", file.name, run.events);
        }
        run.placed_matches(&file)?;
    }
    if run.requests_of(&run.file(0)) != Vec::<u64>::new() {
        bail!("the present file was fetched");
    }
    Ok(())
}

#[then("no part file is left")]
pub(super) fn no_part_left(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let parts: Vec<_> = std::fs::read_dir(run.folder())?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("part"))
        })
        .collect();
    if !parts.is_empty() {
        bail!("parts left: {parts:?}");
    }
    Ok(())
}

// --- A download resumes where it stopped

#[given("a file was half downloaded when the user chose Cancel")]
pub(super) fn half_downloaded_then_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.add_file(1000);
    run.answer(&file, Answer::CancelAfter(500));
    run.run_download();
    if !run
        .summary
        .as_ref()
        .is_some_and(|summary| summary.cancelled)
    {
        bail!("the download was not cancelled: {:?}", run.events);
    }
    run.answer(&file, Answer::Serve);
    run.host.requests.lock().unwrap().clear();
    Ok(())
}

#[then("only the remaining bytes are fetched")]
pub(super) fn only_remaining_fetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if run.requests_of(&file) != [500] {
        bail!("requests: {:?}", run.requests());
    }
    Ok(())
}

#[then("the placed file's checksum matches the catalog")]
pub(super) fn placed_checksum_matches(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    run.placed_matches(&run.file(0))
}

// --- A file whose checksum does not match is downloaded again

#[given("a part whose bytes were changed")]
pub(super) fn corrupted_part(world: &mut IntegrationWorld) {
    let run = download(world);
    let file = run.add_file(800);
    let corrupted: Vec<u8> = run.body(&file)[..400].iter().map(|b| b ^ 0x55).collect();
    run.write_part(&file, &corrupted);
}

#[then(
    "the mismatch is found, the file is fetched again from the start and placed with a matching checksum"
)]
fn mismatch_found_and_refetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if !run
        .events
        .contains(&DownloadEvent::MismatchRetrying(file.name))
    {
        bail!("no mismatch found: {:?}", run.events);
    }
    if run.requests_of(&file) != [400, 0] {
        bail!("requests: {:?}", run.requests());
    }
    run.placed_matches(&file)
}

#[given("the server sends bytes that never match the catalog checksum")]
pub(super) fn never_matches(world: &mut IntegrationWorld) {
    let run = download(world);
    let file = run.add_file(600);
    run.answer(&file, Answer::Corrupt);
}

#[then("the file is fetched twice and then reported Failed with checksum mismatch")]
pub(super) fn fetched_twice_then_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if run.requests_of(&file) != [0, 0] {
        bail!("requests: {:?}", run.requests());
    }
    let failed =
        DownloadEvent::Failed { name: file.name, failure: DownloadFailure::ChecksumMismatch };
    if !run.events.contains(&failed) {
        bail!(
            "not reported Failed with checksum mismatch: {:?}",
            run.events
        );
    }
    Ok(())
}

#[then("nothing is placed under its name")]
pub(super) fn nothing_placed(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let placed = run.folder().join(run.file(0).name);
    if placed.exists() {
        bail!("{} was placed", placed.display());
    }
    Ok(())
}

// --- Without internet the user is told and nothing is deleted

#[given("no connection can be made to the model hosts")]
pub(super) fn no_connection(world: &mut IntegrationWorld) {
    let run = download(world);
    let present = run.add_file(100);
    run.place(&present);
    for size in [200, 300] {
        let file = run.add_file(size);
        run.answer(&file, Answer::Refuse);
    }
}

#[then(
    "each missing file is tried 4 times with pauses of 2, 4 and 8 seconds and then reported Failed with no internet connection"
)]
fn tried_four_times(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let pauses: Vec<u64> = run
        .clock
        .0
        .lock()
        .unwrap()
        .iter()
        .map(Duration::as_secs)
        .collect();
    if pauses != [2, 4, 8, 2, 4, 8] {
        bail!("pauses: {pauses:?}");
    }
    for file in [run.file(1), run.file(2)] {
        if run.requests_of(&file).len() != 4 {
            bail!("{} tried {:?}", file.name, run.requests_of(&file));
        }
        let failed =
            DownloadEvent::Failed { name: file.name, failure: DownloadFailure::NoInternet };
        if !run.events.contains(&failed) {
            bail!(
                "{} not reported Failed with no internet connection",
                file.name
            );
        }
        if DownloadFailure::NoInternet.to_string() != "no internet connection" {
            bail!("the failure reads {}", DownloadFailure::NoInternet);
        }
    }
    Ok(())
}

#[then("the files that were present are still present")]
pub(super) fn present_still_present(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    run.placed_matches(&run.file(0))?;
    if !run.check().rows[0].present {
        bail!("the present file is no longer present");
    }
    Ok(())
}

// --- Not enough space is refused before starting

#[given("3 GB are missing and 1 GB is free")]
pub(super) fn three_missing_one_free(world: &mut IntegrationWorld) {
    let run = download(world);
    run.add_unfetched(2 * GB);
    run.add_unfetched(GB);
    run.free = Some(GB);
}

#[then("nothing is fetched")]
pub(super) fn nothing_fetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    if !run.requests().is_empty() {
        bail!("fetched: {:?}", run.requests());
    }
    Ok(())
}

#[then("the download reports that 3 GB are needed and 1 GB is free")]
pub(super) fn reports_needed_and_free(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let refused = DownloadEvent::NotEnoughSpace { needed: 3 * GB, free: GB };
    if run.events != [refused] {
        bail!("events: {:?}", run.events);
    }
    Ok(())
}

// --- The services start when the last file is present

#[given("the runtime waits for models")]
pub(super) fn runtime_waits(world: &mut IntegrationWorld) -> Result<()> {
    app_misses_dictation_model(world);
    let run = runtime_run(world);
    run.launch();
    run.waits_for_models()
}

#[when("the last missing file is downloaded and placed")]
pub(super) fn last_file_placed(world: &mut IntegrationWorld) -> Result<()> {
    runtime_run(world).place_missing_models()
}

#[then("the runtime starts the services and becomes Ready once they answer")]
pub(super) fn services_start_and_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    run.starting_services_then_ready()?;
    if !run.any_service_required() {
        bail!("no service was started");
    }
    Ok(())
}

/// The files the events start and finish, in their order.
fn started_and_done(events: &[DownloadEvent]) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|event| match event {
            DownloadEvent::Started(name) | DownloadEvent::Done(name) => Some(*name),
            _ => None,
        })
        .collect()
}

/// Whether the events show every byte of `file` received.
fn all_received_shown(
    events: &[DownloadEvent],
    file: &voicepuncher::core::domain::model_catalog::ModelFile,
) -> bool {
    events.iter().any(|event| {
        matches!(event, DownloadEvent::Progress { name, received, total, .. }
            if *name == file.name && *received == file.size_bytes && *total == file.size_bytes)
    })
}
