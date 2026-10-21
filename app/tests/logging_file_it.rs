use std::io::Write;
use std::path::{Path, PathBuf};

use voicepuncher::adapters::secondary::logging::file::{logs_dir, RotatingFileWriter};

const LIMIT: u64 = 64;
const KEEP: usize = 2;

fn line(text: &str) -> Vec<u8> {
    format!("{text}\n").into_bytes()
}

fn first_backup(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.1", path.display()))
}

#[test]
fn should_continue_the_size_count_across_writer_instances() {
    let home = tempfile::tempdir().unwrap();
    let path = logs_dir(home.path()).join("app.log");
    let first = line(&"a".repeat(40));
    let second = line(&"b".repeat(40));

    RotatingFileWriter::open(&path, LIMIT, KEEP)
        .unwrap()
        .write_all(&first)
        .unwrap();
    RotatingFileWriter::open(&path, LIMIT, KEEP)
        .unwrap()
        .write_all(&second)
        .unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), second);
    assert_eq!(std::fs::read(first_backup(&path)).unwrap(), first);
}

#[test]
fn should_resolve_logs_dir_inside_the_home() {
    let home = Path::new("/tmp/home");

    assert_eq!(logs_dir(home), Path::new("/tmp/home/.voicepuncher/logs"));
}
