use std::io::{self, IsTerminal, Stderr, Write};

/// Writes every log line to the log file and, when one is present, to the terminal too.
/// It never fails a write: a failing log file is reported once on the terminal, and
/// logging never stops the app.
pub struct TeeWriter<F: Write, T: Write> {
    file: F,
    terminal: Option<T>,
    file_failure_reported: bool,
}

impl<F: Write, T: Write> TeeWriter<F, T> {
    pub fn new(file: F, terminal: Option<T>) -> Self {
        Self {
            file,
            terminal,
            file_failure_reported: false,
        }
    }

    fn report_file_failure(&mut self, error: &io::Error) {
        if self.file_failure_reported {
            return;
        }
        self.file_failure_reported = true;
        if let Some(terminal) = self.terminal.as_mut() {
            let _ = writeln!(terminal, "log file write failed: {error}");
        }
    }
}

impl<F: Write, T: Write> Write for TeeWriter<F, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Err(error) = self.file.write_all(buf) {
            self.report_file_failure(&error);
        }
        if let Some(terminal) = self.terminal.as_mut() {
            let _ = terminal.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = self.file.flush();
        if let Some(terminal) = self.terminal.as_mut() {
            let _ = terminal.flush();
        }
        Ok(())
    }
}

/// The error output when it is a terminal; decided once, at start.
pub fn terminal_stderr() -> Option<Stderr> {
    let stderr = io::stderr();
    stderr.is_terminal().then_some(stderr)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingFile;

    impl Write for FailingFile {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("disk full"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("disk full"))
        }
    }

    const LINE: &[u8] = b"[t INFO main app] hello\n";

    #[test]
    fn should_write_each_line_to_the_file_and_the_terminal() {
        let mut file: Vec<u8> = Vec::new();
        let mut terminal: Vec<u8> = Vec::new();

        TeeWriter::new(&mut file, Some(&mut terminal))
            .write_all(LINE)
            .unwrap();

        assert_eq!(file, LINE);
        assert_eq!(terminal, LINE);
    }

    #[test]
    fn should_write_only_the_file_without_a_terminal() {
        let mut file: Vec<u8> = Vec::new();

        TeeWriter::new(&mut file, None::<Vec<u8>>)
            .write_all(LINE)
            .unwrap();

        assert_eq!(file, LINE);
    }

    #[test]
    fn should_report_a_failing_file_once_and_never_fail_a_write() {
        let mut terminal: Vec<u8> = Vec::new();
        let mut tee = TeeWriter::new(FailingFile, Some(&mut terminal));

        tee.write_all(LINE).unwrap();
        tee.write_all(LINE).unwrap();
        tee.flush().unwrap();

        let shown = String::from_utf8(terminal).unwrap();
        assert_eq!(shown.matches("log file write failed: disk full").count(), 1);
        assert_eq!(shown.matches("hello").count(), 2);
    }
}
