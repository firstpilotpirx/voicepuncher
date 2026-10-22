use std::panic::PanicHookInfo;

/// Logs every panic, on any thread, as `panic: MESSAGE at FILE:LINE on thread NAME`, then
/// hands it to the hook that was installed before, so a terminal still shows it.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("{}", panic_line(info));
        previous(info);
    }));
}

fn panic_line(info: &PanicHookInfo) -> String {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a value that is not text".to_owned());
    let location = info.location().map_or_else(
        || "an unknown place".to_owned(),
        |place| format!("{}:{}", place.file(), place.line()),
    );
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("-");
    format!("panic: {message} at {location} on thread {name}")
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct CapturedLines(Mutex<Vec<String>>);

    impl log::Log for CapturedLines {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            self.0.lock().unwrap().push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    static LINES: CapturedLines = CapturedLines(Mutex::new(Vec::new()));

    #[test]
    fn should_log_the_message_place_and_thread_of_a_panic() {
        log::set_logger(&LINES).expect("one logger per test process");
        log::set_max_level(log::LevelFilter::Error);
        install_panic_hook();

        let worker = std::thread::Builder::new()
            .name("worker".into())
            .spawn(|| panic!("boom"))
            .unwrap();

        assert!(worker.join().is_err());
        let lines = LINES.0.lock().unwrap();
        let line = lines.iter().find(|l| l.starts_with("panic: boom at "));
        let line = line.expect("the panic is logged");
        let place = regex::Regex::new(r"panic_hook\.rs:\d+ on thread worker$").unwrap();
        assert!(place.is_match(line), "{line}");
        assert!(line.ends_with(" on thread worker"), "{line}");
    }
}
