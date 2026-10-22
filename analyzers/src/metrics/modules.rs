//! The module graph: one module per directory of `.rs` files, its edges from `use crate::…` paths, its import
//! cycles, and the `unsafe`, `.unwrap()` and `.clone()` counts of its code lines.

use std::collections::{BTreeMap, BTreeSet};

/// One source file: its `/`-separated path from the workspace root and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The counts of one module directory.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Module {
    pub unsafes: u64,
    pub unwraps: u64,
    pub clones: u64,
    pub fan_in: usize,
    pub fan_out: usize,
}

/// The module directories with their counts and fan-in/fan-out, and the import cycles between them.
pub struct Graph {
    pub modules: BTreeMap<String, Module>,
    pub cycles: Vec<Vec<String>>,
}

/// Builds the graph over the sources: one module per directory that holds a source.
pub fn graph(sources: &[Source<'_>]) -> Graph {
    let dirs: BTreeSet<String> = sources.iter().map(|s| dir_of(s.path).to_owned()).collect();
    let mut modules: BTreeMap<String, Module> = dirs
        .iter()
        .map(|d| (d.clone(), Module::default()))
        .collect();
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in sources {
        let dir = dir_of(source.path).to_owned();
        let code = code_lines(source.text);
        let module = modules.entry(dir.clone()).or_default();
        module.unsafes += occurrences(&code, "unsafe ");
        module.unwraps += occurrences(&code, ".unwrap()");
        module.clones += occurrences(&code, ".clone()");
        let Some(root) = crate_root(source.path) else {
            continue;
        };
        let targets = crate_paths(&code)
            .into_iter()
            .filter_map(|path| resolve(&dirs, root, &path));
        edges
            .entry(dir.clone())
            .or_default()
            .extend(targets.filter(|target| *target != dir));
    }
    for (from, targets) in &edges {
        modules.entry(from.clone()).or_default().fan_out = targets.len();
        for target in targets {
            modules.entry(target.clone()).or_default().fan_in += 1;
        }
    }
    Graph { modules, cycles: cycles(&edges) }
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// The `src` directory of the crate a file belongs to, e.g. `app/src`.
fn crate_root(path: &str) -> Option<&str> {
    path.find("/src/").map(|i| &path[..i + 4])
}

/// The text without line comments and without the contents of one-line string literals.
fn code_lines(text: &str) -> String {
    text.lines().map(code_of).collect::<Vec<_>>().join("\n")
}

fn code_of(line: &str) -> String {
    let mut code = String::new();
    let (mut in_string, mut escaped) = (false, false);
    for (i, c) in line.char_indices() {
        if in_string {
            in_string = escaped || c != '"';
            escaped = !escaped && c == '\\';
            continue;
        }
        if line[i..].starts_with("//") {
            break;
        }
        in_string = c == '"';
        code.push(c);
    }
    code
}

fn occurrences(code: &str, needle: &str) -> u64 {
    code.matches(needle).count() as u64
}

/// Every module path after `crate::`, with one level of `{a::b, c}` groups expanded.
pub fn crate_paths(code: &str) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    for (at, _) in code.match_indices("crate::") {
        let rest = &code[at + "crate::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            let inner = group.split('}').next().unwrap_or("");
            paths.extend(
                inner
                    .split(',')
                    .map(|item| segments(item.trim()))
                    .filter(|p| !p.is_empty()),
            );
        } else {
            let path = segments(rest);
            if !path.is_empty() {
                paths.push(path);
            }
        }
    }
    paths
}

fn segments(text: &str) -> Vec<String> {
    let end = text
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(text.len());
    text[..end]
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The deepest source directory along the path, from the crate's `src`.
fn resolve(dirs: &BTreeSet<String>, root: &str, path: &[String]) -> Option<String> {
    let mut best = dirs.contains(root).then(|| root.to_owned());
    let mut dir = root.to_owned();
    for segment in path {
        dir = format!("{dir}/{segment}");
        if dirs.contains(&dir) {
            best = Some(dir.clone());
        }
    }
    best
}

/// The strongly connected components of more than one module (Tarjan), each sorted.
fn cycles(edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    let nodes: BTreeSet<&String> = edges.keys().chain(edges.values().flatten()).collect();
    let mut found = Vec::new();
    let mut done = BTreeSet::new();
    for &start in &nodes {
        if done.contains(start) {
            continue;
        }
        let reach = reachable(edges, start);
        let mut component: Vec<String> = reach
            .iter()
            .filter(|n| reachable(edges, n).contains(start))
            .cloned()
            .collect();
        component.sort();
        done.extend(component.iter().cloned());
        if component.len() > 1 {
            found.push(component);
        }
    }
    found
}

fn reachable(edges: &BTreeMap<String, BTreeSet<String>>, start: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([start.to_owned()]);
    let mut todo = vec![start.to_owned()];
    while let Some(node) = todo.pop() {
        for next in edges.get(&node).into_iter().flatten() {
            if seen.insert(next.clone()) {
                todo.push(next.clone());
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_plain_and_grouped_crate_paths() {
        let code =
            "use crate::core::domain::Hotkey;\nuse crate::{runtime::Router, adapters::primary};";
        assert_eq!(
            crate_paths(code),
            vec![
                vec!["core", "domain", "Hotkey"],
                vec!["runtime", "Router"],
                vec!["adapters", "primary"],
            ]
        );
    }

    #[test]
    fn should_count_fan_in_fan_out_and_cycles() {
        let sources = [
            Source { path: "app/src/main.rs", text: "use crate::a::X;" },
            Source { path: "app/src/a/mod.rs", text: "use crate::b::Y; x.unwrap(); // y.unwrap()" },
            Source {
                path: "app/src/b/mod.rs",
                text: "use crate::a::Z; unsafe { } \"unsafe \\\" unsafe \"",
            },
        ];
        let graph = graph(&sources);
        assert_eq!(
            graph.cycles,
            vec![vec!["app/src/a".to_owned(), "app/src/b".to_owned()]]
        );
        let a = &graph.modules["app/src/a"];
        assert_eq!((a.fan_in, a.fan_out, a.unwraps), (2, 1, 1));
        assert_eq!(graph.modules["app/src/b"].unsafes, 1);
    }
}
