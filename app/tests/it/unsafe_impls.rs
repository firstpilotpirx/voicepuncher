//! Every `unsafe impl Send` or `Sync` in the workspace is listed in `ALLOWED` with its reason
//! and carries a `// SAFETY:` comment directly above it; an entry with no impl left fails too.

use std::path::Path;
use walkdir::WalkDir;

const MEMBERS: [&str; 4] = ["app", "command-spotting", "copy-pool", "e2e-test"];

/// One allowed thread-crossing impl: file relative to the workspace root, type, trait, reason.
struct Allowed {
    file: &'static str,
    ty: &'static str,
    marker: &'static str,
    #[expect(
        dead_code,
        reason = "documents the entry; read by people, not by the check"
    )]
    reason: &'static str,
}

const ALLOWED: [Allowed; 0] = [];

/// An `unsafe impl Send|Sync for Type` found in a file, and whether a `SAFETY:` comment sits above it.
#[derive(Debug, PartialEq)]
struct Found {
    file: String,
    ty: String,
    marker: String,
    commented: bool,
}

/// The thread-crossing impls of one source file.
fn scan(file: &str, source: &str) -> Vec<Found> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((marker, rest)) = ["Send", "Sync"].iter().find_map(|m| {
            line.trim()
                .strip_prefix(&format!("unsafe impl {m} for "))
                .map(|rest| (*m, rest))
        }) else {
            continue;
        };
        let ty: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        found.push(Found {
            file: file.to_owned(),
            ty,
            marker: marker.to_owned(),
            commented: comment_above(&lines[..index]).contains("SAFETY:"),
        });
    }
    found
}

/// The `//` comment lines directly above the end of `before`, joined.
fn comment_above(before: &[&str]) -> String {
    before
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every violation of the allowlist rule, as one message each.
fn violations(found: &[Found], allowed: &[Allowed]) -> Vec<String> {
    let listed = |f: &Found| {
        allowed
            .iter()
            .any(|a| a.file == f.file && a.ty == f.ty && a.marker == f.marker)
    };
    let mut messages = Vec::new();
    for f in found {
        if !listed(f) {
            messages.push(format!(
                "{}: unsafe impl {} for {} is not in ALLOWED",
                f.file, f.marker, f.ty
            ));
        } else if !f.commented {
            messages.push(format!(
                "{}: unsafe impl {} for {} has no // SAFETY: comment above it",
                f.file, f.marker, f.ty
            ));
        }
    }
    for a in allowed {
        if !found
            .iter()
            .any(|f| a.file == f.file && a.ty == f.ty && a.marker == f.marker)
        {
            messages.push(format!(
                "{}: no unsafe impl {} for {} left; remove its ALLOWED entry",
                a.file, a.marker, a.ty
            ));
        }
    }
    messages
}

/// Every thread-crossing impl of the workspace's `.rs` files.
fn scan_workspace(root: &Path) -> Vec<Found> {
    MEMBERS
        .iter()
        .flat_map(|member| {
            WalkDir::new(root.join(member))
                .into_iter()
                .filter_entry(|e| e.file_name() != "target")
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        .flat_map(|e| {
            let file = e
                .path()
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            let source =
                std::fs::read_to_string(e.path()).unwrap_or_else(|err| panic!("{file}: {err}"));
            scan(&file, &source)
        })
        .collect()
}

#[test]
fn every_thread_crossing_impl_is_allowed_and_justified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("app has a parent");
    let messages = violations(&scan_workspace(root), &ALLOWED);
    assert!(
        messages.is_empty(),
        "thread-crossing impls:\n{}",
        messages.join("\n")
    );
}

#[test]
fn should_reject_an_unlisted_unsafe_impl_send() {
    let found = scan(
        "src/a.rs",
        "// SAFETY: owned by one thread.\nunsafe impl Send for Stray {}\n",
    );
    assert_eq!(
        violations(&found, &[]),
        ["src/a.rs: unsafe impl Send for Stray is not in ALLOWED"]
    );
}

#[test]
fn should_accept_a_listed_unsafe_impl_with_a_safety_comment() {
    let allowed =
        [Allowed { file: "src/a.rs", ty: "Window", marker: "Sync", reason: "main queue" }];
    let source = "// SAFETY: only touched on\n// the main queue.\nunsafe impl Sync for Window {}\n";
    assert!(violations(&scan("src/a.rs", source), &allowed).is_empty());
}

#[test]
fn should_reject_a_listed_unsafe_impl_without_a_safety_comment() {
    let allowed =
        [Allowed { file: "src/a.rs", ty: "Window", marker: "Send", reason: "main queue" }];
    let source = "// Only touched on the main queue.\nunsafe impl Send for Window {}\n";
    assert_eq!(
        violations(&scan("src/a.rs", source), &allowed),
        ["src/a.rs: unsafe impl Send for Window has no // SAFETY: comment above it"]
    );
}

#[test]
fn should_reject_a_stale_allowlist_entry() {
    let allowed = [Allowed { file: "src/a.rs", ty: "Gone", marker: "Send", reason: "main queue" }];
    assert_eq!(
        violations(&scan("src/a.rs", "struct Gone;\n"), &allowed),
        ["src/a.rs: no unsafe impl Send for Gone left; remove its ALLOWED entry"]
    );
}
