use regex::Regex;
use std::fs;
use walkdir::WalkDir;

mod imports;
mod layout;
use layout::*;
mod covered;

fn read_lines(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Every line of `files` holding one of `patterns`, as `path:line: forbidden 'pattern' — text`.
pub fn forbidden_lines(files: &[std::path::PathBuf], patterns: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    for path in files {
        for (line_no, line) in read_lines(path).iter().enumerate() {
            for pattern in patterns.iter().filter(|pattern| line.contains(*pattern)) {
                let at = format!("{}:{}", path.display(), line_no + 1);
                violations.push(format!("{at}: forbidden '{pattern}' — {}", line.trim()));
            }
        }
    }
    violations
}

pub fn rust_files(root: &str) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .map(|e| e.path().to_path_buf())
        .collect()
}
