//! What the core and the adapters may import, call or print.

use super::*;

#[test]
pub(super) fn r1_domain_no_adapter_imports() {
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
    let violations = forbidden_lines(&rust_files("src/core/domain"), &forbidden_patterns);
    assert!(
        violations.is_empty(),
        "R1 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r2_application_no_adapter_imports() {
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
    let violations = forbidden_lines(&rust_files("src/core/application"), &forbidden_patterns);
    assert!(
        violations.is_empty(),
        "R2 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r10_no_retained_into_raw_under_visualfeedback_macos() {
    let files = rust_files("src/adapters/secondary/visualfeedback/macos");
    let violations = forbidden_lines(&files, &["Retained::into_raw"]);
    assert!(
        violations.is_empty(),
        "R10 violations — Retained::into_raw is banned under visualfeedback/macos:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r_core_no_atomic_register() {
    let forbidden_patterns = [
        "AtomicU8",
        "AtomicU16",
        "AtomicU32",
        "AtomicU64",
        "AtomicUsize",
        "AtomicU128",
    ];
    let files: Vec<_> = rust_files("src/core")
        .into_iter()
        .filter(|path| path.file_stem().and_then(|s| s.to_str()) != Some("mod"))
        .collect();
    let violations = forbidden_lines(&files, &forbidden_patterns);
    assert!(
        violations.is_empty(),
        "R-core-no-atomic-register violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r11_core_never_imports_the_runtime() {
    let violations = forbidden_lines(&rust_files("src/core"), &["crate::runtime"]);
    assert!(
        violations.is_empty(),
        "R11 violations: core imports the runtime\n{}",
        violations.join("\n")
    );
}

/// The core prints nothing: file mode's text and every error are printed by the binary.
#[test]
pub(super) fn r14_core_prints_nothing() {
    let violations = forbidden_lines(&rust_files("src/core"), &["print!(", "println!("]);
    assert!(
        violations.is_empty(),
        "R14 violations: the core prints\n{}",
        violations.join("\n")
    );
}

/// The lines of `path` that import a secondary adapter.
fn secondary_adapter_imports(path: &std::path::Path) -> Vec<String> {
    read_lines(path)
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("adapters::secondary"))
        .map(|(line_no, line)| {
            format!(
                "{}:{}: imports a secondary adapter — {}",
                path.display(),
                line_no + 1,
                line.trim()
            )
        })
        .collect()
}

/// The dictation pipeline of hotkey mode is assembled in `src/runtime/` only; `main.rs`
/// assembles the one of file mode and nothing else does, and no file outside the adapters,
/// the runtime and the binary (`main.rs` and its module `menubar_app.rs`) imports a
/// secondary adapter.
#[test]
pub(super) fn r12_only_the_runtime_assembles_the_hotkey_pipeline() {
    let mut violations: Vec<String> = Vec::new();
    for path in rust_files("src") {
        if path.starts_with("src/runtime") || path.starts_with("src/core") {
            continue;
        }
        let binary = ["src/main.rs", "src/menubar_app.rs"].map(std::path::Path::new);
        let composes = path.starts_with("src/adapters") || binary.contains(&path.as_path());
        if !composes {
            violations.extend(secondary_adapter_imports(&path));
        }
        let assemblies = read_lines(&path)
            .iter()
            .filter(|line| line.contains("Pipeline::new("))
            .count();
        let allowed = usize::from(path == std::path::Path::new("src/main.rs"));
        if assemblies > allowed {
            violations.push(format!(
                "{}: assembles {assemblies} pipelines, {allowed} allowed",
                path.display()
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "R12 violations: a pipeline assembled outside the runtime\n{}",
        violations.join("\n")
    );
}

/// The menu bar item drives the runtime (Quit) and observes it, and the Permissions window
/// observes it; no other adapter knows it.
#[test]
pub(super) fn r13_only_the_menu_bar_and_the_permissions_and_download_windows_import_the_runtime() {
    let mut violations: Vec<String> = Vec::new();
    for path in rust_files("src/adapters") {
        if path.starts_with("src/adapters/primary/menubar")
            || path.starts_with("src/adapters/primary/permissions")
            || path.starts_with("src/adapters/primary/modeldownload")
        {
            continue;
        }
        for (line_no, line) in read_lines(&path).iter().enumerate() {
            if line.contains("crate::runtime") {
                violations.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "R13 violations: an adapter other than the menu bar and the Permissions and Download windows imports the runtime\n{}",
        violations.join("\n")
    );
}
