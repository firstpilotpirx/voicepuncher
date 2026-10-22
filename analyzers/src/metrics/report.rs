//! The printed report: the most complex functions, then one row per module directory.

use super::duplicates::Duplicates;
use super::functions::Function;
use super::modules::Graph;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// How many of the most complex functions the report lists.
const WORST: usize = 25;

pub fn print(functions: &[Function], graph: &Graph, duplicates: &Duplicates) {
    print!("{}", render(functions, graph, duplicates));
}

/// The report's text.
pub fn render(functions: &[Function], graph: &Graph, duplicates: &Duplicates) -> String {
    let mut text = format!("The {WORST} most complex functions (cyclomatic, cognitive, lines):\n");
    let mut worst: Vec<&Function> = functions.iter().collect();
    worst.sort_by(|a, b| {
        b.cyclomatic
            .cmp(&a.cyclomatic)
            .then(b.cognitive.cmp(&a.cognitive))
    });
    for f in worst.iter().take(WORST) {
        let _ = writeln!(
            text,
            "  {:>4} {:>4} {:>5}  {}:{} {}",
            f.cyclomatic, f.cognitive, f.lines, f.file, f.line, f.name
        );
    }
    text += &table(functions, graph);
    for cycle in &graph.cycles {
        let _ = writeln!(text, "import cycle: {}", cycle.join(" ↔ "));
    }
    let _ = writeln!(
        text,
        "duplicated: {} lines in {} clones",
        duplicates.lines, duplicates.clones
    );
    text
}

/// One row per module directory.
fn table(functions: &[Function], graph: &Graph) -> String {
    let mut text = String::new();
    let _ = writeln!(
        text,
        "\n{:<56} {:>5} {:>6} {:>5} {:>5} {:>6} {:>6} {:>5} {:>4} {:>4}",
        "module", "fns", "median", "max", "cycl", "unsafe", "unwrap", "clone", "in", "out"
    );
    let mut by_dir: BTreeMap<&str, Vec<&Function>> = BTreeMap::new();
    for f in functions {
        by_dir
            .entry(f.file.rsplit_once('/').map_or("", |(d, _)| d))
            .or_default()
            .push(f);
    }
    for (dir, module) in &graph.modules {
        let fns = by_dir.get(dir.as_str()).map_or(&[][..], Vec::as_slice);
        let mut lengths: Vec<u64> = fns.iter().map(|f| f.lines).collect();
        lengths.sort_unstable();
        let median = lengths.get(lengths.len() / 2).copied().unwrap_or(0);
        let longest = lengths.last().copied().unwrap_or(0);
        let cyclomatic = fns.iter().map(|f| f.cyclomatic).max().unwrap_or(0);
        let _ = writeln!(
            text,
            "{dir:<56} {:>5} {median:>6} {longest:>5} {cyclomatic:>5} {:>6} {:>6} {:>5} {:>4} {:>4}",
            fns.len(),
            module.unsafes,
            module.unwraps,
            module.clones,
            module.fan_in,
            module.fan_out
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::super::modules::{graph, Source};
    use super::*;

    #[test]
    fn should_list_the_worst_functions_and_one_row_per_module() {
        let functions: Vec<Function> = (1..=30)
            .map(|i| Function {
                file: "app/src/a/mod.rs".into(),
                name: format!("f{i}"),
                line: i,
                cyclomatic: i,
                cognitive: 0,
                lines: i,
            })
            .collect();
        let sources = [
            Source { path: "app/src/a/mod.rs", text: "" },
            Source { path: "app/src/b.rs", text: "" },
        ];
        let text = render(
            &functions,
            &graph(&sources),
            &Duplicates { lines: 3, clones: 1 },
        );
        let listed = text
            .lines()
            .filter(|l| l.contains("app/src/a/mod.rs:"))
            .count();
        assert_eq!(listed, WORST);
        assert!(text
            .lines()
            .next()
            .is_some_and(|l| l.contains("most complex")));
        assert!(text.contains(" f30\n"), "the most complex first:\n{text}");
        assert!(!text.contains(" f5\n"), "only the 25 most complex:\n{text}");
        assert!(text.lines().any(|l| l.starts_with("app/src/a ")
            && l.split_whitespace()
                .take(4)
                .eq(["app/src/a", "30", "16", "30"])));
        assert!(text.lines().any(|l| l.starts_with("app/src ")));
    }
}
