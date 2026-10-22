//! The functions of a `rust-code-analysis-cli -m -O json` report: one JSON object per file, one per line.

use serde::Deserialize;

/// One function and its measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub file: String,
    /// The names of its enclosing impls, traits and modules, then its own, `::`-joined.
    pub name: String,
    pub line: u64,
    pub cyclomatic: u64,
    pub cognitive: u64,
    pub lines: u64,
}

#[derive(Deserialize)]
struct Space {
    name: Option<String>,
    start_line: u64,
    kind: String,
    #[serde(default)]
    spaces: Vec<Space>,
    metrics: Metrics,
}

#[derive(Deserialize)]
struct Metrics {
    cyclomatic: Sum,
    cognitive: Sum,
    loc: Loc,
}

#[derive(Deserialize)]
struct Sum {
    sum: f64,
}

#[derive(Deserialize)]
struct Loc {
    sloc: f64,
}

/// Every function of the report, nested ones included.
pub fn parse(report: &str) -> Result<Vec<Function>, String> {
    let mut functions = Vec::new();
    for line in report.lines().filter(|l| l.starts_with('{')) {
        let unit: Space =
            serde_json::from_str(line).map_err(|e| format!("rust-code-analysis report: {e}"))?;
        let file = unit.name.clone().unwrap_or_default();
        collect(&file, &unit.spaces, &[], &mut functions);
    }
    Ok(functions)
}

fn collect(file: &str, spaces: &[Space], outer: &[&str], functions: &mut Vec<Function>) {
    for space in spaces {
        let own = space.name.as_deref().unwrap_or("?");
        let mut path = outer.to_vec();
        path.push(own);
        if space.kind == "function" {
            functions.push(Function {
                file: file.to_owned(),
                name: path.join("::"),
                line: space.start_line,
                cyclomatic: whole(space.metrics.cyclomatic.sum),
                cognitive: whole(space.metrics.cognitive.sum),
                lines: whole(space.metrics.loc.sloc),
            });
        }
        collect(file, &space.spaces, &path, functions);
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the report's counts are whole"
)]
fn whole(value: f64) -> u64 {
    value.max(0.0).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(kind: &str, name: &str, cyclomatic: f64, spaces: &str) -> String {
        format!(
            r#"{{"name":"{name}","start_line":3,"end_line":9,"kind":"{kind}","spaces":[{spaces}],"metrics":{{"cyclomatic":{{"sum":{cyclomatic}}},"cognitive":{{"sum":2.0}},"loc":{{"sloc":7.0}}}}}}"#
        )
    }

    #[test]
    fn should_read_nested_functions_with_their_qualified_names() {
        let function = space("function", "run", 12.0, "");
        let imp = space("impl", "Pool", 13.0, &function);
        let unit = space("unit", "copy-pool/src/pool.rs", 14.0, &imp);
        let functions = parse(&unit).expect("valid report");
        assert_eq!(
            functions,
            vec![Function {
                file: "copy-pool/src/pool.rs".into(),
                name: "Pool::run".into(),
                line: 3,
                cyclomatic: 12,
                cognitive: 2,
                lines: 7,
            }]
        );
    }
}
