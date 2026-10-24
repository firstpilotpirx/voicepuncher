//! Relocating an engine's file into its own folder: every Homebrew path → `@rpath`, `@loader_path` the only
//! search path, a library's id `@rpath/<file>`, the compiled-in backend folders blanked so ggml loads its backends
//! from its own folder, then signed ad-hoc (the bundle signs it again).

use super::capture;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static KIND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(EXECUTE|DYLIB|BUNDLE)\b").unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s+(\S+) \(").unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static RPATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"cmd LC_RPATH\n.*\n\s+path (\S+)")
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});

/// Whether a reference points into Homebrew.
pub fn is_homebrew(reference: &str) -> bool {
    reference.starts_with("/opt/homebrew/") || reference.starts_with("/usr/local/")
}

/// The libraries `otool -L` names in its output.
pub fn references(otool: &str) -> Vec<String> {
    REFERENCE
        .captures_iter(otool)
        .map(|found| found[1].to_string())
        .collect()
}

/// The `install_name_tool` arguments that relocate a file: its kind (`otool -hv`), references (`otool -L`) and
/// load commands (`otool -l`).
pub fn arguments(file: &str, kind: &str, references_out: &str, load_commands: &str) -> Vec<String> {
    let mut args = Vec::new();
    for reference in references(references_out)
        .iter()
        .filter(|reference| is_homebrew(reference))
    {
        let name = Path::new(reference)
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        args.extend([
            "-change".to_string(),
            reference.clone(),
            format!("@rpath/{name}"),
        ]);
    }
    if kind == "DYLIB" {
        args.extend(["-id".to_string(), format!("@rpath/{file}")]);
    }
    let rpaths: Vec<String> = RPATH
        .captures_iter(load_commands)
        .map(|found| found[1].to_string())
        .collect();
    for rpath in rpaths.iter().filter(|rpath| *rpath != "@loader_path") {
        args.extend(["-delete_rpath".to_string(), rpath.clone()]);
    }
    if !rpaths.iter().any(|rpath| rpath == "@loader_path") {
        args.extend(["-add_rpath".to_string(), "@loader_path".into()]);
    }
    args
}

/// Overwrites every occurrence of each folder with as many zero bytes; how many it overwrote.
pub fn blank(bytes: &mut [u8], folders: &[String]) -> usize {
    let mut count = 0;
    for folder in folders
        .iter()
        .map(String::as_bytes)
        .filter(|folder| !folder.is_empty())
    {
        let mut at = 0;
        while let Some(offset) = bytes[at..]
            .windows(folder.len())
            .position(|window| window == folder)
        {
            let start = at + offset;
            bytes[start..start + folder.len()].fill(0);
            at = start + folder.len();
            count += 1;
        }
    }
    count
}

/// Relocates and signs one file of an engine's folder.
pub fn relocate(path: &Path, file: &str, folders: &[String]) -> Result<(), String> {
    let text = path.to_string_lossy();
    let header = capture("otool", &["-hv", &text])?;
    let kind = KIND
        .captures(&header)
        .map(|found| found[1].to_string())
        .ok_or_else(|| format!("{text} is no Mach-O file"))?;
    let args = arguments(
        file,
        &kind,
        &capture("otool", &["-L", &text])?,
        &capture("otool", &["-l", &text])?,
    );
    if !args.is_empty() {
        let mut all: Vec<&str> = args.iter().map(String::as_str).collect();
        all.push(&text);
        capture("install_name_tool", &all)?;
    }
    if !folders.is_empty() {
        let mut bytes = std::fs::read(path).map_err(|e| format!("{text}: {e}"))?;
        if blank(&mut bytes, folders) > 0 {
            std::fs::write(path, bytes).map_err(|e| format!("{text}: {e}"))?;
        }
    }
    capture(
        "codesign",
        &["--force", "--sign", "-", "--timestamp=none", &text],
    )?;
    Ok(())
}

/// Every reference of the files of a folder stays in it: none into Homebrew, every `@rpath/<file>` present.
pub fn check_folder(dir: &Path, shown: &str) -> Result<(), String> {
    for path in files(dir) {
        let text = path.to_string_lossy();
        for reference in references(&capture("otool", &["-L", &text])?) {
            if is_homebrew(&reference) {
                return Err(format!("{text} references {reference}, outside the bundle"));
            }
            if reference
                .strip_prefix("@rpath/")
                .is_some_and(|name| !dir.join(name).is_file())
            {
                return Err(format!(
                    "{text} references {reference}, which {shown} lacks"
                ));
            }
        }
    }
    Ok(())
}

/// The files of a folder but its `VERSION`, by name.
pub fn files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    found.retain(|path| path.is_file() && path.file_name().is_some_and(|name| name != "VERSION"));
    found.sort();
    found
}
