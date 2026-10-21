//! The models folder on disk: presence by size, by variable and in the old folders; part
//! files appended, hashed and placed; an archive extracted; the free space.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use voicepuncher::adapters::secondary::modelfetch::fs::{FsDiskSpace, FsModelStore};
use voicepuncher::core::application::{DiskSpace, ModelStore};
use voicepuncher::core::domain::{ModelFile, ModelKind, ModelUse};

const VARIABLE: &str = "VOICEPUNCHER_TEST_MODEL";
/// sha256 of "hello world".
const HELLO_SHA256: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

fn file(name: &'static str, size: u64) -> ModelFile {
    ModelFile {
        name,
        url: "https://models.example/file",
        sha256: HELLO_SHA256,
        size_bytes: size,
        purpose: "test",
        kind: ModelKind::File,
        used_by: ModelUse::Dictation,
        variable: VARIABLE,
    }
}

fn store(home: &Path, env: HashMap<String, String>) -> FsModelStore {
    FsModelStore::new(home.join("models-folder"), home, env)
}

#[test]
fn should_count_a_file_of_the_catalog_size_present_and_another_size_missing() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path(), HashMap::new());
    std::fs::create_dir_all(home.path().join("models-folder")).unwrap();
    std::fs::write(home.path().join("models-folder/a.bin"), b"hello world").unwrap();
    std::fs::write(home.path().join("models-folder/b.bin"), b"short").unwrap();
    // Then
    assert!(store.present(&file("a.bin", 11)));
    assert!(!store.present(&file("b.bin", 11)));
    assert!(!store.present(&file("c.bin", 11)));
}

#[test]
fn should_count_a_file_in_the_old_folder_present() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let old = home.path().join(".voicepuncher");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("a.bin"), b"hello world").unwrap();
    // Then
    assert!(store(home.path(), HashMap::new()).present(&file("a.bin", 11)));
}

#[test]
fn should_count_a_file_named_by_its_variable_present() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let elsewhere = home.path().join("elsewhere.bin");
    std::fs::write(&elsewhere, b"x").unwrap();
    let env = HashMap::from([(VARIABLE.to_string(), elsewhere.display().to_string())]);
    // Then
    assert!(store(home.path(), env).present(&file("a.bin", 11)));
}

#[test]
fn should_append_to_a_part_hash_it_and_place_it() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path(), HashMap::new());
    let hello = file("a.bin", 11);
    {
        let mut sink = store.open_part(&hello).unwrap();
        sink.write(b"hello").unwrap();
    }
    assert_eq!(store.part_len(&hello), 5);
    // When
    let mut sink = store.open_part(&hello).unwrap();
    sink.write(b" world").unwrap();
    let hash = sink.sha256_hex();
    drop(sink);
    store.place(&hello).unwrap();
    // Then
    assert_eq!(hash, HELLO_SHA256);
    assert_eq!(store.part_len(&hello), 0);
    assert!(store.present(&hello));
}

#[test]
fn should_start_a_part_over_on_restart() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path(), HashMap::new());
    let hello = file("a.bin", 11);
    let mut sink = store.open_part(&hello).unwrap();
    sink.write(b"stale bytes").unwrap();
    // When
    sink.restart().unwrap();
    sink.write(b"hello world").unwrap();
    // Then
    assert_eq!(sink.sha256_hex(), HELLO_SHA256);
    assert_eq!(sink.len(), 11);
}

#[test]
fn should_delete_a_part() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path(), HashMap::new());
    let hello = file("a.bin", 11);
    store.open_part(&hello).unwrap().write(b"hel").unwrap();
    // When
    store.delete_part(&hello).unwrap();
    // Then
    assert_eq!(store.part_len(&hello), 0);
}

#[test]
fn should_extract_an_archive_into_its_folder_and_delete_it() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("source");
    std::fs::create_dir_all(source.join("kws-model")).unwrap();
    std::fs::write(source.join("kws-model/tokens.txt"), b"a b c").unwrap();
    let folder = home.path().join("models-folder");
    std::fs::create_dir_all(&folder).unwrap();
    let status = Command::new("/usr/bin/tar")
        .arg("-cjf")
        .arg(folder.join("kws.tar.bz2"))
        .arg("-C")
        .arg(&source)
        .arg("kws-model")
        .status()
        .unwrap();
    assert!(status.success());
    let kws = ModelFile {
        kind: ModelKind::TarBz2 {
            folder: "kws-model",
        },
        ..file("kws.tar.bz2", 1)
    };
    let store = store(home.path(), HashMap::new());
    // When
    store.extract(&kws).unwrap();
    // Then
    assert!(folder.join("kws-model/tokens.txt").is_file());
    assert!(!folder.join("kws.tar.bz2").exists());
    assert!(store.present(&kws));
}

#[test]
fn should_report_an_archive_that_is_no_archive() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let folder = home.path().join("models-folder");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("kws.tar.bz2"), b"not bzip2").unwrap();
    let kws = ModelFile {
        kind: ModelKind::TarBz2 {
            folder: "kws-model",
        },
        ..file("kws.tar.bz2", 1)
    };
    // When
    let result = store(home.path(), HashMap::new()).extract(&kws);
    // Then
    assert!(result.is_err());
}

#[test]
fn should_tell_the_free_space_of_a_folder_not_made_yet() {
    // Given
    let home = tempfile::tempdir().unwrap();
    // When
    let free = FsDiskSpace::new(home.path().join("not/made/yet")).free_bytes();
    // Then
    assert!(free.is_some_and(|bytes| bytes > 0));
}
