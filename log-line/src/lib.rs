//! The form of one log line, shared by the app and the command spotting service.

use std::io::Write;

/// One log line with the thread that wrote it, so one session or connection traces across its
/// threads: `[time LEVEL thread target] message`.
pub fn log_line(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    let thread = std::thread::current();
    writeln!(
        buf,
        "[{} {:<5} {} {}] {}",
        buf.timestamp(),
        record.level(),
        thread.name().unwrap_or("-"),
        record.target(),
        record.args()
    )
}
