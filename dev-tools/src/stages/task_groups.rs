//! The warning of a full suite (`test`, `check`, `full-check`) started on a branch `<type>/<name>` whose change
//! (`openspec/changes/<name>/tasks.md`) has an open task group — a `##` heading with an unchecked `- [ ]` task —
//! that is not the last: while optimizing one area, `just test-area <path>` runs only its tests (AGENTS.md § TDD).
//! A branch without a change, or a change without open groups before its last, warns of nothing.

use crate::context::{var, Context};
use crate::process::git_line;

const FULL_SUITES: [&str; 3] = ["test", "check", "full-check"];

/// Prints the warning when the run is a full suite on such a branch.
pub fn warn(context: &Context) {
    let recipe = var("VOICEPUNCHER_STAGES_RECIPE");
    if !recipe
        .as_deref()
        .is_some_and(|recipe| FULL_SUITES.contains(&recipe))
    {
        return;
    }
    let Some(branch) = git_line(&context.root, &["rev-parse", "--abbrev-ref", "HEAD"]) else {
        return;
    };
    let Some((_, name)) = branch.split_once('/') else {
        return;
    };
    let tasks = context
        .root
        .join("openspec/changes")
        .join(name)
        .join("tasks.md");
    let Some(group) = open_group_not_last(&std::fs::read_to_string(tasks).unwrap_or_default())
    else {
        return;
    };
    println!(
        "● task group \"{group}\" of {name} is still open and not the last: while optimizing one area, run `just test-area <path>` (`just needed-tests` maps the areas) instead of the full suite"
    );
}

/// The first open task group, when it is not the last group.
pub fn open_group_not_last(tasks: &str) -> Option<String> {
    let mut groups: Vec<(String, bool)> = Vec::new();
    for line in tasks.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            groups.push((heading.trim().to_string(), false));
        } else if line.trim_start().starts_with("- [ ]") {
            if let Some(group) = groups.last_mut() {
                group.1 = true;
            }
        }
    }
    let first_open = groups.iter().position(|(_, open)| *open)?;
    (first_open + 1 < groups.len()).then(|| groups[first_open].0.clone())
}

#[cfg(test)]
mod tests {
    use super::open_group_not_last;

    const TASKS: &str = "## 1. First\n\n- [x] 1.1 done\n\n## 2. Second\n\n- [ ] 2.1 open\n\n## 3. Last\n\n- [ ] 3.1 open\n";

    #[test]
    fn should_name_the_first_open_group_before_the_last() {
        assert_eq!(open_group_not_last(TASKS).as_deref(), Some("2. Second"));
    }

    #[test]
    fn should_warn_of_nothing_when_only_the_last_group_is_open() {
        let last_only = TASKS.replace("- [ ] 2.1", "- [x] 2.1");
        assert_eq!(open_group_not_last(&last_only), None);
        assert_eq!(open_group_not_last(""), None);
        assert_eq!(open_group_not_last(&TASKS.replace("- [ ]", "- [x]")), None);
    }
}
