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
        "ureq",
        "serde_json",
        "arboard",
        "core_graphics",
        "tungstenite",
        "sherpa_onnx",
        "voicepuncher_copy_pool",
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
        "ureq",
        "serde_json",
        "arboard",
        "core_graphics",
        "tungstenite",
        "sherpa_onnx",
        "voicepuncher_copy_pool",
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

fn leaf_adapter_dirs(root: &str) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .filter(|e| {
            let path = e.path();
            let has_mod_rs = path.join("mod.rs").exists();
            let depth = path
                .strip_prefix(root)
                .map(|p| p.components().count())
                .unwrap_or(0);
            if !has_mod_rs {
                return false;
            }
            if depth == 2 {
                return true;
            }
            if depth == 1 {
                let has_deeper = WalkDir::new(path)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .any(|e| {
                        e.file_type().is_dir()
                            && e.path() != path
                            && e.path().join("mod.rs").exists()
                    });
                return !has_deeper;
            }
            false
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[test]
fn r3_adapter_dirs_match_naming_convention() {
    let naming_re = Regex::new(r"^[a-z][a-z0-9]*$").unwrap();
    let adapter_roots = ["src/adapters/primary", "src/adapters/secondary"];
    let mut violations: Vec<String> = Vec::new();

    for root in &adapter_roots {
        for leaf_dir in leaf_adapter_dirs(root) {
            let feature = leaf_dir
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let tech = leaf_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if !naming_re.is_match(feature) {
                violations.push(format!(
                    "{}: feature segment '{}' must be a single lowercase word",
                    leaf_dir.display(),
                    feature
                ));
            }
            if !naming_re.is_match(tech) {
                violations.push(format!(
                    "{}: tech segment '{}' must be a single lowercase word",
                    leaf_dir.display(),
                    tech
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

fn secondary_leaf_adapter_names() -> Vec<(std::path::PathBuf, String)> {
    leaf_adapter_dirs("src/adapters/secondary")
        .into_iter()
        .map(|dir| {
            let tech = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let feature = dir
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let stem = format!("{feature}_{tech}");
            (dir, stem)
        })
        .collect()
}

#[test]
fn r4_every_secondary_adapter_has_it() {
    let mut violations: Vec<String> = Vec::new();

    for (dir, stem) in secondary_leaf_adapter_names() {
        let expected_it = format!("tests/{stem}_it.rs");
        if !std::path::Path::new(&expected_it).exists() {
            violations.push(format!(
                "{}: missing integration test at {}",
                dir.display(),
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

#[test]
fn r7_adapters_in_feature_tech_structure() {
    let adapter_roots = ["src/adapters/primary", "src/adapters/secondary"];
    let mut violations: Vec<String> = Vec::new();

    for root in &adapter_roots {
        let entries = match fs::read_dir(root) {
            Ok(e) => e,
            Err(err) => {
                violations.push(format!("cannot read {root}: {err}"));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if stem != "mod" {
                    violations.push(format!(
                        "{}: flat .rs adapter file not allowed — must be in <feature>/<tech>/mod.rs",
                        path.display()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R7 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r8_no_ports_subfolder() {
    assert!(
        !std::path::Path::new("src/core/application/ports").exists(),
        "R8: src/core/application/ports/ must not exist — ports live flat in core/application/"
    );
}

#[test]
fn r10_no_retained_into_raw_under_visualfeedback_macos() {
    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new("src/adapters/secondary/visualfeedback/macos")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let path = entry.path();
        for (line_no, line) in read_lines(path).iter().enumerate() {
            if line.contains("Retained::into_raw") {
                violations.push(format!(
                    "{}:{}: forbidden raw-pointer escape — {}",
                    path.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "R10 violations — Retained::into_raw is banned under visualfeedback/macos:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r_core_no_atomic_register() {
    let forbidden_patterns = [
        "AtomicU8",
        "AtomicU16",
        "AtomicU32",
        "AtomicU64",
        "AtomicUsize",
        "AtomicU128",
    ];

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
        "R-core-no-atomic-register violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn r9_every_primary_adapter_has_it() {
    let mut violations: Vec<String> = Vec::new();

    for (dir, stem) in leaf_adapter_dirs("src/adapters/primary")
        .into_iter()
        .map(|dir| {
            let tech = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let depth = dir
                .strip_prefix("src/adapters/primary")
                .map(|p| p.components().count())
                .unwrap_or(0);
            let feature = if depth == 2 {
                dir.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned()
            } else {
                String::new()
            };
            let stem = if feature.is_empty() {
                tech.clone()
            } else {
                format!("{feature}_{tech}")
            };
            (dir, stem)
        })
    {
        let expected_it = format!("tests/{stem}_it.rs");
        if !std::path::Path::new(&expected_it).exists() {
            violations.push(format!(
                "{}: missing integration test at {}",
                dir.display(),
                expected_it
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "R9 violations:\n{}",
        violations.join("\n")
    );
}
