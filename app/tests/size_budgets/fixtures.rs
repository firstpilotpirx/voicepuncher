//! Each budget rule proven on a source written in the test.

use crate::baseline::{parse, Baseline};
use crate::budget::{Kind, Measures};
use crate::measure::measure_source;
use crate::verdict::violations;
use crate::workspace::directory_measures;

const PATH: &str = "app/src/fixture.rs";
const MARKER: &str = "#[expect(clippy::too_many_lines, reason = \"size baseline\")]\n";

/// A function `name` with `code` code lines, `noise` comment and blank lines between them, and `attrs` before it.
fn function(attrs: &str, name: &str, code: usize, noise: usize) -> String {
    let mut body = String::new();
    for i in 0..code {
        body.push_str(&format!("    let _v{i} = {i};\n"));
        if i < noise {
            body.push_str(if i % 2 == 0 {
                "    // a comment\n"
            } else {
                "\n"
            });
        }
    }
    format!("{attrs}fn {name}() {{\n{body}}}\n")
}

fn measure(source: &str) -> Measures {
    measure_source(PATH, source).measures
}

fn listed(entries: &[(&str, &str, usize)]) -> Baseline {
    let text: String = entries
        .iter()
        .map(|(kind, id, n)| format!("[{kind}]\n\"{id}\" = {n}\n"))
        .collect();
    parse(&text)
}

fn found(source: &str, baseline: &Baseline) -> String {
    violations(&measure(source), baseline).join("\n")
}

#[test]
fn should_not_count_comments_and_blank_lines() {
    let measures = measure(&function("", "f", 30, 20));
    assert_eq!(
        measures.units[&(Kind::FunctionLines, format!("{PATH}::f"))],
        30
    );
    assert!(violations(&measures, &Baseline::new()).is_empty());
}

#[test]
fn should_fail_a_new_function_over_budget() {
    let report = found(&function("", "long", 41, 0), &Baseline::new());
    assert!(
        report.contains("app/src/fixture.rs::long: 41 code lines, over the budget of 40"),
        "{report}"
    );
}

#[test]
fn should_fail_a_listed_function_that_grew() {
    let baseline = listed(&[("function_lines", "app/src/fixture.rs::long", 52)]);
    let report = found(&function(MARKER, "long", 53, 0), &baseline);
    assert!(
        report.contains("long: grew from 52 to 53 code lines"),
        "{report}"
    );
}

#[test]
fn should_pass_a_listed_function_that_shrank() {
    let baseline = listed(&[("function_lines", "app/src/fixture.rs::long", 52)]);
    assert_eq!(found(&function(MARKER, "long", 45, 0), &baseline), "");
}

#[test]
fn should_fail_a_listed_file_back_within_budget() {
    let source: String = (0..280).map(|i| format!("const C{i}: u8 = 0;\n")).collect();
    let report = found(&source, &listed(&[("file_lines", PATH, 420)]));
    assert!(
        report.contains("fixture.rs: 280 code lines, back within the budget of 300; remove its size-baseline entry"),
        "{report}"
    );
}

#[test]
fn should_fail_a_struct_with_too_many_fields() {
    let fields: String = (0..13).map(|i| format!("    f{i}: u8,\n")).collect();
    let report = found(&format!("struct Wide {{\n{fields}}}\n"), &Baseline::new());
    assert!(
        report.contains("struct_fields app/src/fixture.rs::Wide: 13 fields"),
        "{report}"
    );
}

#[test]
fn should_fail_a_type_with_too_large_a_public_surface() {
    let methods: String = (0..36)
        .map(|i| format!("    pub fn m{i}(&self) {{}}\n    fn p{i}(&self) {{}}\n"))
        .collect();
    let report = found(
        &format!("struct Big;\nimpl Big {{\n{methods}}}\n"),
        &Baseline::new(),
    );
    assert!(
        report.contains("public_members app/src/fixture.rs::Big: 36 public members"),
        "{report}"
    );
}

#[test]
fn should_fail_a_module_directory_over_budget() {
    let files = [
        ("app/src/big/a.rs".to_owned(), 1500),
        ("app/src/big/b.rs".to_owned(), 501),
    ];
    let report = violations(&directory_measures(&files), &Baseline::new()).join("\n");
    assert!(
        report.contains("directory_lines app/src/big: 2001 code lines"),
        "{report}"
    );
}

#[test]
fn should_fail_too_many_parameters_outside_trait_impls() {
    let params = "a: u8, b: u8, c: u8, d: u8, e: u8, f: u8";
    let source =
        format!("fn wide({params}) {{}}\nstruct S;\nimpl Tr for S {{ fn wide({params}) {{}} }}\n");
    let report = found(&source, &Baseline::new());
    assert!(
        report.contains("parameters app/src/fixture.rs::wide: 6 parameters"),
        "{report}"
    );
    assert!(!report.contains("<S as Tr>"), "{report}");
}

#[test]
fn should_fail_an_attribute_without_its_entry_and_an_entry_without_its_attribute() {
    let baseline = listed(&[("excessive_nesting", "app/src/fixture.rs::unmarked", 0)]);
    let source = format!(
        "{}{}",
        function(MARKER, "marked", 45, 0),
        function("", "unmarked", 3, 0)
    );
    let report = found(&source, &baseline);
    assert!(
        report.contains("marked: size-baseline attribute without a baseline entry"),
        "{report}"
    );
    assert!(
        report
            .contains("unmarked: listed in the size baseline without its size-baseline attribute"),
        "{report}"
    );
}

#[test]
fn should_report_a_budget_lint_silenced_outside_the_baseline() {
    let source = "#[allow(clippy::too_many_arguments)]\nfn f() {}\n#[allow(clippy::needless_return)]\nfn g() {}\n";
    let suppressions = measure_source(PATH, source).suppressions;
    assert_eq!(
        suppressions,
        vec![format!("{PATH}: clippy::too_many_arguments")]
    );
}

#[test]
fn should_count_an_async_function_with_its_braces_as_clippy_does() {
    let source = function("", "f", 39, 0).replacen("fn f", "async fn f", 1);
    let measures = measure(&source);
    assert_eq!(
        measures.units[&(Kind::FunctionLines, format!("{PATH}::f"))],
        41
    );
}
