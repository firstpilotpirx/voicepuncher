//! The log lines of a test process, for the tests that check what a line says.

use std::sync::{Mutex, OnceLock};

/// Keeps every log line the process writes, each with the name of the thread that wrote it.
pub struct LogLines(Mutex<Vec<(String, String)>>);

impl log::Log for LogLines {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let thread = std::thread::current().name().unwrap_or("-").to_owned();
        self.0
            .lock()
            .unwrap()
            .push((thread, record.args().to_string()));
    }

    fn flush(&self) {}
}

impl LogLines {
    /// The lines logged so far that contain `fragment`, each as `[thread] message`.
    pub fn containing(&self, fragment: &str) -> Vec<String> {
        let lines = self.0.lock().unwrap();
        let named = lines
            .iter()
            .map(|(thread, message)| format!("[{thread}] {message}"));
        named.filter(|line| line.contains(fragment)).collect()
    }

    /// The messages logged so far that start with `prefix`.
    pub fn starting_with(&self, prefix: &str) -> Vec<String> {
        let lines = self.0.lock().unwrap();
        let messages = lines.iter().map(|(_, message)| message);
        messages
            .filter(|message| message.starts_with(prefix))
            .cloned()
            .collect()
    }
}

/// The one logger of the test process, set at its first call to keep the lines at `level` and
/// above; nextest runs each test in a process of its own, so it holds one test's lines.
pub fn captured(level: log::LevelFilter) -> &'static LogLines {
    static LINES: OnceLock<&'static LogLines> = OnceLock::new();
    LINES.get_or_init(|| {
        let lines: &'static LogLines = Box::leak(Box::new(LogLines(Mutex::default())));
        if log::set_logger(lines).is_ok() {
            log::set_max_level(level);
        }
        lines
    })
}
