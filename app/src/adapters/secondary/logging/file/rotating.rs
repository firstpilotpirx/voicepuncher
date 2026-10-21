use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Appends to a file and, before a write that would take it past the limit, moves it to
/// `<file>.1` (shifting earlier backups up to `keep`, dropping the oldest) and starts anew.
pub struct RotatingFileWriter {
    path: PathBuf,
    max_bytes: u64,
    keep: usize,
    file: File,
    written: u64,
}

impl RotatingFileWriter {
    /// Opens the file for appending, creating its folder when missing; the size count
    /// starts at the file's current length.
    pub fn open(path: &Path, max_bytes: u64, keep: usize) -> io::Result<Self> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let file = open_append(path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            max_bytes,
            keep,
            file,
            written,
        })
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        remove_if_exists(&backup_path(&self.path, self.keep))?;
        for number in (1..self.keep).rev() {
            rename_if_exists(
                &backup_path(&self.path, number),
                &backup_path(&self.path, number + 1),
            )?;
        }
        if self.keep > 0 {
            fs::rename(&self.path, backup_path(&self.path, 1))?;
        } else {
            fs::remove_file(&self.path)?;
        }
        self.file = open_append(&self.path)?;
        self.written = 0;
        Ok(())
    }

    fn would_pass_limit(&self, next: usize) -> bool {
        self.written > 0 && self.written + next as u64 > self.max_bytes
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.would_pass_limit(buf.len()) {
            self.rotate()?;
        }
        let count = self.file.write(buf)?;
        self.written += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

/// `<path>.<number>`: the suffix is appended, so `voicepuncher.log` becomes `voicepuncher.log.1`.
fn backup_path(path: &Path, number: usize) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(format!(".{number}"));
    PathBuf::from(name)
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn rename_if_exists(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: u64 = 100;
    const KEEP: usize = 5;

    fn log_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("nested").join("app.log")
    }

    fn writer(path: &Path) -> RotatingFileWriter {
        RotatingFileWriter::open(path, LIMIT, KEEP).unwrap()
    }

    fn line(len: usize) -> Vec<u8> {
        let mut bytes = vec![b'x'; len - 1];
        bytes.push(b'\n');
        bytes
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn should_create_folder_and_keep_writes_below_limit_in_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_path(&dir);
        let mut w = writer(&path);

        w.write_all(&line(40)).unwrap();
        w.write_all(&line(40)).unwrap();

        assert_eq!(read(&path).len(), 80);
        assert_eq!(files_in(path.parent().unwrap()), vec!["app.log"]);
    }

    #[test]
    fn should_rotate_before_the_write_that_passes_the_limit_keeping_the_line_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_path(&dir);
        let mut w = writer(&path);
        w.write_all(&line(90)).unwrap();

        w.write_all(b"the whole line\n").unwrap();

        assert_eq!(read(&path), "the whole line\n");
        assert_eq!(read(&backup_path(&path, 1)).len(), 90);
    }

    #[test]
    fn should_shift_backups_and_drop_the_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_path(&dir);
        let mut w = writer(&path);
        for session in 0..7u8 {
            w.write_all(&line(90)).unwrap();
            w.write_all(&[b'0' + session, b'\n']).unwrap();
        }

        let names = files_in(path.parent().unwrap());

        assert_eq!(
            names,
            vec![
                "app.log",
                "app.log.1",
                "app.log.2",
                "app.log.3",
                "app.log.4",
                "app.log.5"
            ]
        );
        assert!(read(&backup_path(&path, 1)).ends_with("5\n"));
        assert!(read(&backup_path(&path, 5)).ends_with("1\n"));
    }

    #[test]
    fn should_count_an_existing_file_when_reopened() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_path(&dir);
        writer(&path).write_all(&line(60)).unwrap();

        let mut reopened = writer(&path);
        reopened.write_all(&line(50)).unwrap();

        assert_eq!(read(&path).len(), 50);
        assert_eq!(read(&backup_path(&path, 1)).len(), 60);
    }

    #[test]
    fn should_write_a_line_longer_than_the_limit_into_a_fresh_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_path(&dir);
        let mut w = writer(&path);
        w.write_all(&line(10)).unwrap();

        w.write_all(&line(150)).unwrap();

        assert_eq!(read(&path).len(), 150);
        assert_eq!(read(&backup_path(&path, 1)).len(), 10);
    }
}
