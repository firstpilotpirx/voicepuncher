//! The `stage_inputs` of `.config/stages.toml`: which files feed which stage. `<glob> <stages>` per line, the stages
//! comma-separated, `*` spanning `/`; `code` names no model stage, `accuracy` both accuracy stages and `none`
//! makes a file feed no stage. A file no line names feeds every stage.

/// A model stage: it runs on the local services and its key also covers what it runs on outside the
/// repository.
pub fn is_model_stage(stage: &str) -> bool {
    stage.starts_with("e2e") || stage == "refine-eval"
}

struct Rule {
    glob: String,
    stages: Vec<String>,
}

pub struct Rules(Vec<Rule>);

impl Rules {
    pub fn parse(table: &str) -> Self {
        let rules = table
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let glob = fields.next()?;
                let names = fields.next()?;
                let stages = names.split(',').flat_map(expand_alias).collect();
                Some(Rule { glob: glob.to_string(), stages })
            })
            .collect();
        Self(rules)
    }

    /// Whether `path` feeds `stage`: not when a matching line says `none`, nor for a model stage when lines
    /// name the file but not that stage.
    pub fn feeds(&self, path: &str, stage: &str) -> bool {
        let (mut named, mut none, mut feeds) = (false, false, false);
        for rule in self.0.iter().filter(|rule| glob_matches(&rule.glob, path)) {
            named = true;
            none |= rule.stages.iter().any(|name| name == "none");
            feeds |= rule.stages.iter().any(|name| name == stage);
        }
        !none && (!named || !is_model_stage(stage) || feeds)
    }
}

fn expand_alias(name: &str) -> Vec<String> {
    if name == "accuracy" {
        vec!["e2e-quick".into(), "e2e-bench".into()]
    } else {
        vec![name.to_string()]
    }
}

/// A glob whose `*` spans any characters, `/` included, matched against the whole text.
pub fn glob_matches(glob: &str, text: &str) -> bool {
    let parts: Vec<&str> = glob.split('*').collect();
    let (first, rest) = parts.split_first().unwrap_or((&"", &[]));
    let Some(mut tail) = text.strip_prefix(first) else {
        return false;
    };
    let Some((last, middle)) = rest.split_last() else {
        return tail.is_empty();
    };
    for part in middle {
        match tail.find(part) {
            Some(at) => tail = &tail[at + part.len()..],
            None => return false,
        }
    }
    tail.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_match_a_glob_whose_star_spans_folders() {
        assert!(glob_matches("docs/*", "docs/a/b.md"));
        assert!(glob_matches("*.md", "README.md"));
        assert!(glob_matches(
            "app/src/core/domain/voice_command*.rs",
            "app/src/core/domain/voice_command_recognizer.rs"
        ));
        assert!(glob_matches("AGENTS.md", "AGENTS.md"));
        assert!(!glob_matches("AGENTS.md", "docs/AGENTS.md"));
        assert!(!glob_matches("a*b*c", "abx"));
        assert!(glob_matches("a*b*b", "abb"));
        assert!(!glob_matches("a*bb", "ab"));
    }

    #[test]
    fn should_feed_the_stages_a_file_names_and_every_code_stage() {
        let rules = Rules::parse(
            "    docs/*  code\n    *.md  none\n    command-spotting/*  e2e-smoke-voice,accuracy\n",
        );
        assert!(rules.feeds("docs/a.txt", "fmt"));
        assert!(!rules.feeds("docs/a.txt", "e2e-smoke-voice"));
        assert!(!rules.feeds("docs/a.md", "fmt"));
        assert!(rules.feeds("command-spotting/x.rs", "e2e-bench"));
        assert!(!rules.feeds("command-spotting/x.rs", "refine-eval"));
        assert!(rules.feeds("app/x.rs", "refine-eval"));
    }
}
