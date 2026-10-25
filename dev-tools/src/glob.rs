//! The globs of the stage tables: a `*` spans any characters, `/` included.

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
            "app/src/core/domain/voice_command_recognizer/mod.rs"
        ));
        assert!(glob_matches("AGENTS.md", "AGENTS.md"));
        assert!(!glob_matches("AGENTS.md", "docs/AGENTS.md"));
        assert!(!glob_matches("a*b*c", "abx"));
        assert!(glob_matches("a*b*b", "abb"));
        assert!(!glob_matches("a*bb", "ab"));
    }
}
