//! Steps of the log file's rotation.

use super::{
    ensure, filler, given, new_run, read, run, then, when, IntegrationWorld, Result, Write,
    KEPT_BACKUPS,
};

#[given("a log file just below its size limit")]
pub(super) fn just_below_limit(world: &mut IntegrationWorld) {
    world.app.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    run.writer().write_all(&filler(b'a', 90)).expect("written");
}

#[when("a line is written that would take it past the limit")]
pub(super) fn write_past_limit(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.last_line = b"the line that crosses the limit\n".to_vec();
    let line = run.last_line.clone();
    Ok(run.writer().write_all(&line)?)
}

#[then("the former content is in the first backup and the new log file holds only that whole line")]
pub(super) fn rotated_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(read(&run.backup(1)) == String::from_utf8(filler(b'a', 90))?);
    ensure!(read(&run.log_path()).as_bytes() == run.last_line.as_slice());
    Ok(())
}

#[given("a log file with five backups")]
pub(super) fn five_backups(world: &mut IntegrationWorld) {
    world.app.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    for marker in b'a'..=b'f' {
        run.writer()
            .write_all(&filler(marker, 90))
            .expect("written");
    }
}

#[when("the log file rotates again")]
pub(super) fn rotate_again(world: &mut IntegrationWorld) -> Result<()> {
    Ok(run(world).writer().write_all(&filler(b'g', 90))?)
}

#[then("each backup moved one number up, the former fifth backup is gone and six files remain")]
pub(super) fn shifted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    for (number, marker) in (1..=KEPT_BACKUPS).zip([b'f', b'e', b'd', b'c', b'b']) {
        let expected = String::from_utf8(filler(marker, 90))?;
        ensure!(
            read(&run.backup(number)) == expected,
            "backup {number} holds the wrong part"
        );
    }
    let files = std::fs::read_dir(run.logs())?.count();
    ensure!(files == KEPT_BACKUPS + 1, "{files} files");
    Ok(())
}

#[given("a log file filled past half its limit by an earlier session")]
pub(super) fn earlier_half(world: &mut IntegrationWorld) {
    world.app.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    run.writer().write_all(&filler(b'a', 60)).expect("written");
    run.writer = None;
}

#[when("a new session writes as much again")]
pub(super) fn new_session_writes(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.open_writer();
    Ok(run.writer().write_all(&filler(b'b', 60))?)
}

#[then("the log file rotated when it passed the limit, counting what was there before the session")]
pub(super) fn counted_earlier(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(read(&run.backup(1)) == String::from_utf8(filler(b'a', 60))?);
    ensure!(read(&run.log_path()) == String::from_utf8(filler(b'b', 60))?);
    Ok(())
}
