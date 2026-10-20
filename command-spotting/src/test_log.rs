//! The log lines of a test process, for the tests that check what a line names.

/// Keeps every log line, with the name of the thread that logged it.
pub(crate) struct Lines(pub(crate) std::sync::Mutex<Vec<String>>);

impl log::Log for Lines {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        let thread = std::thread::current();
        let line = format!("[{}] {}", thread.name().unwrap_or("-"), record.args());
        self.0.lock().unwrap().push(line);
    }
    fn flush(&self) {}
}

impl Lines {
    /// The lines logged so far that contain `fragment`.
    pub(crate) fn containing(&self, fragment: &str) -> Vec<String> {
        let lines = self.0.lock().unwrap();
        lines
            .iter()
            .filter(|line| line.contains(fragment))
            .cloned()
            .collect()
    }
}

/// The one logger of the test process, set at its first use.
pub(crate) fn captured() -> &'static Lines {
    static LINES: std::sync::OnceLock<&'static Lines> = std::sync::OnceLock::new();
    LINES.get_or_init(|| {
        let lines: &'static Lines = Box::leak(Box::new(Lines(std::sync::Mutex::default())));
        let _ = log::set_logger(lines);
        log::set_max_level(log::LevelFilter::Trace);
        lines
    })
}
