//! The documents written for people and agents (`AGENTS.md`, `README.md`, `docs/**/*.md`) name
//! only `just` recipes, repository paths and loopback ports that exist; a stale name fails here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use walkdir::WalkDir;

/// The top-level entries a documented repository path starts with.
const ROOTS: [&str; 8] = [
    "app/",
    "command-spotting/",
    "copy-pool/",
    "e2e-test/",
    "docs/",
    ".config/",
    ".github/",
    "openspec/",
];
/// Files at the repository root a document may name.
const ROOT_FILES: [&str; 11] = [
    "justfile",
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "AGENTS.md",
    "CLAUDE.md",
    "CHANGELOG.md",
    "deny.toml",
    "clippy.toml",
    ".typos.toml",
    ".gitignore",
];
/// Loopback ports live here; smaller numbers are years, counts and well-known ports.
const PORT_RANGE: std::ops::RangeInclusive<u32> = 1024..=65535;

/// One stale name: the document, its line and what is missing.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    document: String,
    line: usize,
    kind: &'static str,
    name: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {} `{}` does not exist",
            self.document, self.line, self.kind, self.name
        )
    }
}

/// What exists: the recipes, the declared ports and whether a repository path is there.
struct Known {
    recipes: BTreeSet<String>,
    ports: BTreeSet<u32>,
    path_exists: Box<dyn Fn(&str) -> bool>,
}

/// The recipe names of a `justfile`: every unindented `name [params]:` header, not a `:=` variable.
fn recipe_names(justfile: &str) -> BTreeSet<String> {
    let header = Regex::new(r"^@?([A-Za-z_][A-Za-z0-9_-]*)(?:\s[^:]*)?:(?:\s|$)")
        .unwrap_or_else(|e| panic!("{e}"));
    justfile
        .lines()
        .filter_map(|line| header.captures(line))
        .map(|caps| caps[1].to_owned())
        .collect()
}

/// The numbers in the port range a source text declares.
fn declared_ports(source: &str) -> BTreeSet<u32> {
    let number = Regex::new(r"\b\d{4,5}\b").unwrap_or_else(|e| panic!("{e}"));
    number
        .find_iter(source)
        .filter_map(|m| m.as_str().parse().ok())
        .filter(|port| PORT_RANGE.contains(port))
        .collect()
}

/// The `just <recipe>` names a line quotes in backticks.
fn quoted_recipes(line: &str) -> Vec<String> {
    let recipe =
        Regex::new(r"`just ([A-Za-z_][A-Za-z0-9_-]*)[^`]*`").unwrap_or_else(|e| panic!("{e}"));
    recipe
        .captures_iter(line)
        .map(|caps| caps[1].to_owned())
        .collect()
}

/// The repository paths a line quotes in backticks: rooted at a top-level entry, no pattern characters.
fn quoted_paths(line: &str) -> Vec<String> {
    let quoted = Regex::new(r"`([^`\s]+)`").unwrap_or_else(|e| panic!("{e}"));
    quoted
        .captures_iter(line)
        .map(|caps| caps[1].trim_end_matches(':').to_owned())
        .filter(|token| !token.contains(['*', '<', '>', '{', '}', '…']))
        .filter(|token| {
            ROOTS.iter().any(|root| token.starts_with(root)) || ROOT_FILES.contains(&token.as_str())
        })
        .collect()
}

/// The ports a line names: numbers after the word `port(s)`, or after `:` in an address.
fn named_ports(line: &str) -> Vec<u32> {
    let after_port =
        Regex::new(r"(?i)\bports?\b([^.;)`\n]{0,60})").unwrap_or_else(|e| panic!("{e}"));
    let in_address = Regex::new(r":(\d{4,5})\b").unwrap_or_else(|e| panic!("{e}"));
    let number = Regex::new(r"\b\d{4,5}\b").unwrap_or_else(|e| panic!("{e}"));
    let mut ports: Vec<u32> = after_port
        .captures_iter(line)
        .flat_map(|caps| {
            number
                .find_iter(caps.get(1).map_or("", |m| m.as_str()))
                .collect::<Vec<_>>()
        })
        .filter_map(|m| m.as_str().parse().ok())
        .collect();
    ports.extend(
        in_address
            .captures_iter(line)
            .filter_map(|caps| caps[1].parse::<u32>().ok()),
    );
    ports.retain(|port| PORT_RANGE.contains(port));
    ports
}

/// Every stale recipe, path and port one document names.
fn check_document(document: &str, text: &str, known: &Known) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut push = |line: usize, kind: &'static str, name: String| {
        findings.push(Finding { document: document.to_owned(), line, kind, name });
    };
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        for recipe in quoted_recipes(line) {
            if !known.recipes.contains(&recipe) {
                push(number, "recipe", recipe);
            }
        }
        for path in quoted_paths(line) {
            if !(known.path_exists)(&path) {
                push(number, "path", path);
            }
        }
        for port in named_ports(line) {
            if !known.ports.contains(&port) {
                push(number, "port", port.to_string());
            }
        }
    }
    findings
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// The documents the check covers, relative to the repository root.
fn documents(root: &Path) -> Vec<String> {
    let mut found = vec!["AGENTS.md".to_owned(), "README.md".to_owned()];
    for entry in WalkDir::new(root.join("docs"))
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.path().extension().is_some_and(|e| e == "md") {
            if let Ok(rel) = entry.path().strip_prefix(root) {
                found.push(rel.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

/// A path exists when it is on disk or git ignores it (generated folders are documented on purpose).
fn on_disk_or_ignored(root: &Path, path: &str) -> bool {
    if root.join(path).exists() {
        return true;
    }
    let as_directory = format!("{}/", path.trim_end_matches('/'));
    [path, as_directory.as_str()].iter().any(|candidate| {
        Command::new("git")
            .args(["check-ignore", "-q", candidate])
            .current_dir(root)
            .status()
            .is_ok_and(|status| status.success())
    })
}

/// A non-test Rust file of a member, or an E2E script.
fn is_port_source(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(extension, "rs" | "py") && !text.contains("/tests/") && !text.ends_with("/tests.rs")
}

/// The sources that declare ports: the members' non-test Rust files and the E2E scripts.
fn port_sources(root: &Path) -> impl Iterator<Item = PathBuf> + use<> {
    let root = root.to_path_buf();
    [
        "app/src",
        "command-spotting/src",
        "copy-pool/src",
        "e2e-test/src",
        "e2e-test/scripts",
    ]
    .into_iter()
    .flat_map(move |member| WalkDir::new(root.join(member)).into_iter())
    .filter_map(Result::ok)
    .map(walkdir::DirEntry::into_path)
    .filter(|path| is_port_source(path))
}

/// The copies of a speech recognition service listen 10000 above its port, up to four of them
/// (`copy-pool`: `SPEECH_RECOGNITION_COPY_OFFSET`, copy 1 to 4), so a documented copy port is derived.
fn copy_ports(services: &BTreeSet<u32>) -> Vec<u32> {
    services
        .iter()
        .filter(|port| (8000..=8999).contains(*port))
        .flat_map(|port| (1..=4).map(move |copy| port + 10_000 + copy))
        .collect()
}

fn known_in_repository(root: &Path) -> Known {
    let justfile = std::fs::read_to_string(root.join("justfile")).unwrap_or_default();
    let mut ports = declared_ports(&justfile);
    for path in port_sources(root) {
        ports.extend(declared_ports(
            &std::fs::read_to_string(path).unwrap_or_default(),
        ));
    }
    ports.extend(copy_ports(&ports));
    let root_for_paths = root.to_path_buf();
    Known {
        recipes: recipe_names(&justfile),
        ports,
        path_exists: Box::new(move |path| on_disk_or_ignored(&root_for_paths, path)),
    }
}

#[test]
fn should_pass_when_every_named_thing_exists() {
    let root = repository_root();
    let known = known_in_repository(&root);
    let mut findings = Vec::new();
    for document in documents(&root) {
        let text = std::fs::read_to_string(root.join(&document)).unwrap_or_default();
        findings.extend(check_document(&document, &text, &known));
    }
    let report: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert!(
        findings.is_empty(),
        "stale names in the documents:\n{}",
        report.join("\n")
    );
}

#[cfg(test)]
mod fixtures {
    use super::*;

    const JUSTFILE: &str = "stt_port := \"8081\"\n\n# Every test\ntest:\n    cargo test\n\ncheck *args: test\n    echo {{args}}\n\n_private-helper:\n    true\n";

    fn known() -> Known {
        Known {
            recipes: recipe_names(JUSTFILE),
            ports: declared_ports(JUSTFILE),
            path_exists: Box::new(|path| path == "app/src/main.rs" || path == "docs/"),
        }
    }

    fn names(findings: &[Finding]) -> Vec<(usize, &'static str, &str)> {
        findings
            .iter()
            .map(|f| (f.line, f.kind, f.name.as_str()))
            .collect()
    }

    #[test]
    fn should_read_recipe_headers_and_skip_variables() {
        let recipes = recipe_names(JUSTFILE);
        assert_eq!(
            recipes,
            ["_private-helper", "check", "test"]
                .map(str::to_owned)
                .into()
        );
    }

    #[test]
    fn should_fail_on_a_recipe_the_justfile_does_not_have() {
        let doc = "Run `just test` first.\nThen `just release-check` (gone) and `just check`.\n";
        let findings = check_document("docs/x.md", doc, &known());
        assert_eq!(names(&findings), vec![(2, "recipe", "release-check")]);
        assert_eq!(findings[0].document, "docs/x.md");
    }

    #[test]
    fn should_fail_on_a_path_that_is_not_in_the_repository() {
        let doc = "See `app/src/main.rs` and `docs/`.\n\nThe old `app/src/cli.rs` is gone; `docs/architecture/*.md` and `app/tests/<adapter>_it.rs` are patterns; `Contents/MacOS/` is inside the bundle.\n";
        let findings = check_document("README.md", doc, &known());
        assert_eq!(names(&findings), vec![(3, "path", "app/src/cli.rs")]);
    }

    #[test]
    fn should_fail_on_a_port_the_code_does_not_declare() {
        let doc = "Dictation on port 8081, at `http://127.0.0.1:8081/v1`.\nThe bench on ports 8090 and 8081; a 16000 Hz stream; 21600 s.\n";
        let findings = check_document("README.md", doc, &known());
        assert_eq!(names(&findings), vec![(2, "port", "8090")]);
    }

    #[test]
    fn should_pass_a_clean_fixture() {
        let doc = "`just test` reads `app/src/main.rs` on port 8081.\n";
        assert!(check_document("docs/x.md", doc, &known()).is_empty());
    }
}
