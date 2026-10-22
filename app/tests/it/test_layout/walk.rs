//! The walk of a test binary's directory: every `.rs` file and directory under it must be a
//! module of the file that owns its directory.

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// The `mod <name>;` lines of a test binary's root file.
fn declared_modules(main: &Path) -> Vec<String> {
    let module = Regex::new(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;").expect("a valid regex");
    fs::read_to_string(main)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| module.captures(line).map(|c| c[1].to_owned()))
        .collect()
}

/// The file that declares the modules of `dir`: `main.rs` at the top of a binary, below it the
/// `<dir>.rs` beside the directory or the `mod.rs` inside it.
fn owner_of(dir: &Path) -> PathBuf {
    let beside = dir.with_extension("rs");
    if dir.join("main.rs").exists() {
        dir.join("main.rs")
    } else if beside.exists() {
        beside
    } else {
        dir.join("mod.rs")
    }
}

/// The `.rs` files and directories of `dir`.
fn modules_of(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() || path.extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();
    paths.sort();
    paths
}

/// Whether `declared` names `path`, or `path` is a directory with a same-named `.rs` file beside it.
fn is_declared(path: &Path, declared: &[String]) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    declared.iter().any(|d| d == stem) || (path.is_dir() && path.with_extension("rs").exists())
}

/// The entries under `dir`, at any depth, that the file owning their directory leaves out: a
/// `.rs` file with no `mod` line, or a directory with neither a `mod` line nor a same-named
/// `.rs` file beside it.
pub fn undeclared_in(dir: &Path) -> Vec<String> {
    let owner = owner_of(dir);
    let declared = declared_modules(&owner);
    let mut violations = Vec::new();
    for path in modules_of(dir) {
        if path.is_dir() {
            violations.extend(undeclared_in(&path));
        }
        if path != owner && !is_declared(&path, &declared) {
            violations.push(format!(
                "{}: not a module of {}",
                path.display(),
                owner.display()
            ));
        }
    }
    violations.sort();
    violations
}

#[test]
fn should_name_a_nested_file_its_owning_module_leaves_out() {
    let dir = std::env::temp_dir().join(format!("test_layout_nested_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("a/deep")).expect("a temp dir");
    fs::write(dir.join("main.rs"), "mod a;\n").expect("main.rs");
    fs::write(dir.join("a.rs"), "mod declared;\n").expect("a.rs");
    fs::write(dir.join("a/declared.rs"), "").expect("a/declared.rs");
    fs::write(dir.join("a/orphan.rs"), "").expect("a/orphan.rs");
    fs::write(dir.join("a/deep/mod.rs"), "").expect("a/deep/mod.rs");
    fs::write(dir.join("a/deep/lost.rs"), "").expect("a/deep/lost.rs");
    let violations = undeclared_in(&dir);
    let _ = fs::remove_dir_all(&dir);
    let names = |file: &str, owner: &str| {
        violations
            .iter()
            .any(|v| v.contains(&format!("{file}: not a module of")) && v.ends_with(owner))
    };
    assert_eq!(violations.len(), 3, "{violations:?}");
    assert!(names("a/deep", "a.rs"), "{violations:?}");
    assert!(names("a/deep/lost.rs", "deep/mod.rs"), "{violations:?}");
    assert!(names("a/orphan.rs", "a.rs"), "{violations:?}");
}
