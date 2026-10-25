//! Where the adapters live and how their folders are named.

use super::*;

pub(super) fn leaf_adapter_dirs(root: &str) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_dir() && is_leaf_adapter(e.path(), root))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// A module folder two levels under `root`, or one level under it with no module folder
/// inside.
fn is_leaf_adapter(path: &std::path::Path, root: &str) -> bool {
    if !path.join("mod.rs").exists() {
        return false;
    }
    let depth = path
        .strip_prefix(root)
        .map(|p| p.components().count())
        .unwrap_or(0);
    match depth {
        2 => true,
        1 => !has_module_folder(path),
        _ => false,
    }
}

fn has_module_folder(path: &std::path::Path) -> bool {
    WalkDir::new(path)
        .max_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|e| e.file_type().is_dir() && e.path() != path && e.path().join("mod.rs").exists())
}

#[test]
pub(super) fn r3_adapter_dirs_match_naming_convention() {
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

/// The lines of `path` that import an external crate outside `allowed`.
fn disallowed_crate_uses(
    path: &std::path::Path,
    use_re: &Regex,
    allowed: &std::collections::HashSet<&str>,
) -> Vec<String> {
    let skip_prefixes = ["crate::", "std::", "super::", "self::"];
    read_lines(path)
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let used = line.trim_start().strip_prefix("use ");
            used.is_some_and(|used| !skip_prefixes.iter().any(|pfx| used.starts_with(pfx)))
        })
        .filter_map(|(line_no, line)| {
            let caps = use_re.captures(line)?;
            let first_segment = caps.get(1).map_or("", |m| m.as_str());
            let is_crate_name = first_segment.chars().next().is_some_and(char::is_lowercase);
            (is_crate_name && !allowed.contains(first_segment)).then(|| {
                format!(
                    "{}:{}: disallowed external crate '{}' — {}",
                    path.display(),
                    line_no + 1,
                    first_segment,
                    line.trim()
                )
            })
        })
        .collect()
}

#[test]
pub(super) fn r6_core_only_whitelisted_external_crates() {
    let allowed: std::collections::HashSet<&str> = ["anyhow", "thiserror", "serde", "log"]
        .iter()
        .copied()
        .collect();
    let use_re = Regex::new(r"^\s*use\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();

    let violations: Vec<String> = rust_files("src/core")
        .into_iter()
        .filter(|path| path.file_stem().and_then(|s| s.to_str()) != Some("mod"))
        .flat_map(|path| disallowed_crate_uses(&path, &use_re, &allowed))
        .collect();

    assert!(
        violations.is_empty(),
        "R6 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r7_adapters_in_feature_tech_structure() {
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
        let flat_files = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| path.file_stem().and_then(|s| s.to_str()) != Some("mod"));
        violations.extend(flat_files.map(|path| {
            format!(
                "{}: flat .rs adapter file not allowed — must be in <feature>/<tech>/mod.rs",
                path.display()
            )
        }));
    }

    assert!(
        violations.is_empty(),
        "R7 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r8_no_ports_subfolder() {
    assert!(
        !std::path::Path::new("src/core/application/ports").exists(),
        "R8: src/core/application/ports/ must not exist — ports live flat in core/application/"
    );
}
