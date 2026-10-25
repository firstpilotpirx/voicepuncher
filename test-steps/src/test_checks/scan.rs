//! The brace scan of `test_checks`: finds the test functions of a source and what their bodies
//! check, on rustfmt-formatted code.

/// A test function of a source: its name and its body without strings and comments.
pub struct TestFn {
    pub name: String,
    pub body: String,
}

/// The source with every string, char literal and comment blanked to `""`, `' '` and nothing,
/// keeping its lines, so braces and words inside them do not count.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let Some(skipped) = skipped_end(&chars, at) else {
            out.push(c);
            at += 1;
            continue;
        };
        out.push_str(if c == '/' { "" } else { "\"\"" });
        out.extend(chars[at..skipped].iter().filter(|c| **c == '\n'));
        at = skipped;
    }
    out
}

/// Where the comment, string or char literal starting at `at` ends, or `None` for code.
fn skipped_end(chars: &[char], at: usize) -> Option<usize> {
    match chars[at] {
        '/' if chars.get(at + 1) == Some(&'/') => Some(line_end(chars, at)),
        '\'' if chars.get(at + 2) == Some(&'\'') => Some(at + 3),
        '\'' if chars.get(at + 1) == Some(&'\\') => Some(closing(chars, at + 1, "'", true)),
        '"' => Some(closing(chars, at + 1, "\"", true)),
        'r' => raw_string_end(chars, at),
        _ => None,
    }
}

/// Where the raw string starting at `at` ends, or `None` when the `r` ends a word or starts one.
fn raw_string_end(chars: &[char], at: usize) -> Option<usize> {
    let after_word = at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '_');
    if after_word || !matches!(chars.get(at + 1), Some('#' | '"')) {
        return None;
    }
    let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
    let close = format!("\"{}", "#".repeat(hashes));
    Some(closing(chars, at + 2 + hashes, &close, false))
}

fn line_end(chars: &[char], from: usize) -> usize {
    chars[from..]
        .iter()
        .position(|c| *c == '\n')
        .map_or(chars.len(), |end| from + end)
}

/// Past the first `close` from `from` on, skipping escaped characters when the literal has
/// `escapes` (a raw string has none).
fn closing(chars: &[char], from: usize, close: &str, escapes: bool) -> usize {
    let close: Vec<char> = close.chars().collect();
    let mut at = from;
    while at < chars.len() {
        if escapes && chars[at] == '\\' {
            at += 2;
            continue;
        }
        if chars[at..].starts_with(&close) {
            return at + close.len();
        }
        at += 1;
    }
    chars.len()
}

/// The test functions of `source`: those under `#[test]` or `#[tokio::test]`, and with
/// `main_thread` every `fn should_…` (the main-thread harness lists them).
pub fn test_fns(source: &str, main_thread: bool) -> Vec<TestFn> {
    let code = code_only(source);
    let lines: Vec<&str> = code.lines().collect();
    let mut found = Vec::new();
    let mut attributed = false;
    let mut should_panic = false;
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test") {
            attributed = true;
        }
        should_panic |= trimmed.starts_with("#[should_panic");
        let Some(name) = fn_name(trimmed) else {
            continue;
        };
        if attributed || (main_thread && name.starts_with("should_") && !line.starts_with(' ')) {
            let body = body_from(&lines[at..]);
            let body = if should_panic {
                format!("panic!{body}")
            } else {
                body
            };
            found.push(TestFn { name, body });
        }
        attributed = false;
        should_panic = false;
    }
    found
}

fn fn_name(line: &str) -> Option<String> {
    // A restricted visibility (`pub(super)`, `pub(crate)`) is dropped like `pub`
    let line = line
        .strip_prefix("pub(")
        .and_then(|rest| rest.split_once(") "))
        .map_or(line, |(_, rest)| rest);
    let rest = line
        .strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("async fn "))
        .or_else(|| line.strip_prefix("fn "))?;
    Some(
        rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect(),
    )
}

/// The body of the function whose signature starts `lines`, from its opening brace to the
/// matching one.
fn body_from(lines: &[&str]) -> String {
    let text = lines.join("\n");
    let Some(open) = text.find('{') else {
        return String::new();
    };
    let mut depth = 0usize;
    for (at, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => continue,
        }
        if depth == 0 {
            return text[open..=open + at].to_owned();
        }
    }
    text[open..].to_owned()
}

/// Where the first check of `body` stands: an assertion, a failure, an unwrap or `?`, a mock
/// expectation, or a call of a helper named `assert_…`, `expect_…` or `check_…`.
pub fn first_check(body: &str) -> Option<usize> {
    const MARKERS: [&str; 12] = [
        "assert",
        "bail!",
        "ensure!",
        "panic!",
        "unreachable!",
        ".unwrap()",
        ".expect(",
        "unwrap_err",
        "?;",
        "?.",
        "?)",
        "?\n",
    ];
    let words = ["expect_", "check_"];
    let marked = MARKERS.iter().filter_map(|m| body.find(m));
    let called = words.iter().filter_map(|w| {
        body.match_indices(w)
            .find(|(at, _)| !body[..*at].ends_with(|c: char| c.is_alphanumeric() || c == '_'))
            .map(|(at, _)| at)
    });
    marked.chain(called).min()
}

/// Why `test` breaks the rule, if it does: no check, or a `return` before its first check.
pub fn offence(test: &TestFn) -> Option<&'static str> {
    let Some(check) = first_check(&test.body) else {
        return Some("checks nothing: add an assertion that can fail, or delete it");
    };
    let returns = test.body.match_indices("return").any(|(at, _)| {
        at < check && !test.body[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
    });
    returns.then_some("returns before its first check: a skipped test must not pass")
}
