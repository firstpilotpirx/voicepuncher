use regex::Regex;
use std::fs;
use walkdir::WalkDir;

fn read_lines(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn r1_domain_no_adapter_imports() {
    let forbidden_patterns = [
        "use crate::core::application",
        "use crate::adapters",
        "cpal",
        "rdev",
        "whisper_rs",
        "llama_cpp_2",
        "arboard",
        "enigo",
    ];

    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new("src/core/domain")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        for (line_no, line) in read_lines(path).iter().enumerate() {
            for pattern in &forbidden_patterns {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{}: forbidden '{}' — {}",
                        path.display(),
                        line_no + 1,
                        pattern,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R1 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r2_application_no_adapter_imports() {
    let forbidden_patterns = [
        "use crate::adapters",
        "cpal",
        "rdev",
        "whisper_rs",
        "llama_cpp_2",
        "arboard",
        "enigo",
    ];

    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new("src/core/application")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        for (line_no, line) in read_lines(path).iter().enumerate() {
            for pattern in &forbidden_patterns {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{}: forbidden '{}' — {}",
                        path.display(),
                        line_no + 1,
                        pattern,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R2 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r3_adapter_filenames_match_naming_convention() {
    let naming_re = Regex::new(r"^[a-z][a-z0-9]*(_[a-z][a-z0-9]*)*$").unwrap();
    let adapter_dirs = ["src/adapters/primary", "src/adapters/secondary"];
    let mut violations: Vec<String> = Vec::new();

    for dir in &adapter_dirs {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(err) => {
                violations.push(format!("cannot read {dir}: {err}"));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if stem == "mod" {
                continue;
            }
            if !naming_re.is_match(stem) {
                violations.push(format!(
                    "{}: filename stem '{}' does not match snake_case pattern",
                    path.display(),
                    stem
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R3 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r4_every_secondary_adapter_has_it() {
    let mut violations: Vec<String> = Vec::new();

    let entries = match fs::read_dir("src/adapters/secondary") {
        Ok(e) => e,
        Err(err) => {
            panic!("cannot read src/adapters/secondary: {err}");
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem == "mod" {
            continue;
        }
        let expected_it = format!("tests/{stem}_it.rs");
        if !std::path::Path::new(&expected_it).exists() {
            violations.push(format!(
                "{}: missing integration test at {}",
                path.display(),
                expected_it
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "R4 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r5_domain_vos_have_test_blocks() {
    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new("src/core/domain")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem == "mod" {
            continue;
        }
        let content = fs::read_to_string(path).unwrap_or_default();
        if !content.contains("mod tests") {
            violations.push(format!("{}: missing 'mod tests' block", path.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "R5 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r6_core_only_whitelisted_external_crates() {
    let allowed: std::collections::HashSet<&str> = ["anyhow", "thiserror", "serde", "log"]
        .iter()
        .copied()
        .collect();

    let use_re = Regex::new(r"^\s*use\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    let skip_prefixes = ["crate::", "std::", "super::", "self::"];
    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new("src/core")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if stem == "mod" {
            continue;
        }
        for (line_no, line) in read_lines(path).iter().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            if skip_prefixes
                .iter()
                .any(|pfx| trimmed.trim_start_matches("use ").starts_with(pfx))
            {
                continue;
            }
            if let Some(caps) = use_re.captures(line) {
                let first_segment = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let is_crate_name = first_segment
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false);
                if is_crate_name && !allowed.contains(first_segment) {
                    violations.push(format!(
                        "{}:{}: disallowed external crate '{}' — {}",
                        path.display(),
                        line_no + 1,
                        first_segment,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R6 violations:\n{}",
        violations.join("\n")
    );
}
