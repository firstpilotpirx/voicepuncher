//! The workspace members (the root `Cargo.toml`'s `members`) and the scope of each one's test part
//! `test@<member>`, as Cargo builds its tests: the member, its `path = "../<member>"` dependencies and
//! dev-dependencies, and from those on their normal dependencies only (a dependency's dev-dependencies are not
//! built for its dependents); every member for one whose `Cargo.toml` sets `tests-read-every-member = true`
//! under `[package.metadata.stage-keys]`.

use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

static MEMBERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^members\s*=\s*\[([^\]]*)\]")
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static QUOTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""([^"]+)""#).unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\[([^\]]+)\]").unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static PATH_DEPENDENCY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"path\s*=\s*"\.\./([^"/]+)/?""#)
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static EVERY_MEMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*tests-read-every-member\s*=\s*true")
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});

#[derive(Default)]
struct Manifest {
    dependencies: Vec<String>,
    dev_dependencies: Vec<String>,
    every_member: bool,
}

pub struct Workspace {
    pub members: Vec<String>,
    scope: BTreeMap<String, BTreeSet<String>>,
}

impl Workspace {
    pub fn read(root: &Path) -> Self {
        let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap_or_default();
        let members: Vec<String> = MEMBERS
            .captures(&read("Cargo.toml"))
            .map(|list| {
                QUOTED
                    .captures_iter(&list[1])
                    .map(|name| name[1].to_string())
                    .collect()
            })
            .unwrap_or_default();
        let manifests: BTreeMap<String, Manifest> = members
            .iter()
            .map(|member| {
                (
                    member.clone(),
                    manifest(&read(&format!("{member}/Cargo.toml")), &members),
                )
            })
            .collect();
        let scope = members
            .iter()
            .map(|member| (member.clone(), scope_of(member, &members, &manifests)))
            .collect();
        Self { members, scope }
    }

    /// The member a path lies under, if any.
    pub fn owner(&self, path: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|member| path.starts_with(&format!("{member}/")))
            .map(String::as_str)
    }

    /// Whether the files of `owner` feed the test part of `part`.
    pub fn in_scope(&self, part: &str, owner: &str) -> bool {
        self.scope
            .get(part)
            .is_some_and(|scope| scope.contains(owner))
    }

    pub fn is_member(&self, name: &str) -> bool {
        self.members.iter().any(|member| member == name)
    }

    /// The stages with `test@members` replaced by one part per member.
    pub fn expand(&self, stages: &[String]) -> Vec<String> {
        stages
            .iter()
            .flat_map(|stage| {
                if stage == "test@members" {
                    self.members
                        .iter()
                        .map(|member| format!("test@{member}"))
                        .collect()
                } else {
                    vec![stage.clone()]
                }
            })
            .collect()
    }
}

fn manifest(text: &str, members: &[String]) -> Manifest {
    let mut found = Manifest::default();
    let mut section = String::new();
    for line in text.lines() {
        if let Some(name) = SECTION.captures(line) {
            section = name[1].to_string();
        }
        if section == "package.metadata.stage-keys" && EVERY_MEMBER.is_match(line) {
            found.every_member = true;
        }
        let Some(dependency) = PATH_DEPENDENCY.captures(line) else {
            continue;
        };
        let dependency = dependency[1].to_string();
        if !members.contains(&dependency) {
            continue;
        }
        if section.contains("dev-dependencies") {
            found.dev_dependencies.push(dependency);
        } else {
            found.dependencies.push(dependency);
        }
    }
    found
}

fn scope_of(
    member: &str,
    members: &[String],
    manifests: &BTreeMap<String, Manifest>,
) -> BTreeSet<String> {
    let own = manifests.get(member);
    let mut todo: Vec<String> = vec![member.to_string()];
    todo.extend(
        own.map(|found| found.dev_dependencies.clone())
            .unwrap_or_default(),
    );
    if own.is_some_and(|found| found.every_member) {
        todo.extend(members.iter().cloned());
    }
    let mut scope = BTreeSet::new();
    let mut next = 0;
    while let Some(name) = todo.get(next).cloned() {
        next += 1;
        if scope.insert(name.clone()) {
            todo.extend(
                manifests
                    .get(&name)
                    .map(|found| found.dependencies.clone())
                    .unwrap_or_default(),
            );
        }
    }
    scope
}
