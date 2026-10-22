//! Code lines as clippy's `too_many_lines` counts them: a physical line that is neither blank nor comment-only.

use proc_macro2::LineColumn;

/// Counts the code lines of `text`, skipping blank lines, `//` comment lines and lines inside `/* */` comments.
pub fn code_lines(text: &str) -> usize {
    let mut in_comment = false;
    text.lines()
        .filter(|line| line_has_code(line, &mut in_comment))
        .count()
}

/// The port of clippy's per-line scan: a line has code when something other than a comment starts on it.
fn line_has_code(mut line: &str, in_comment: &mut bool) -> bool {
    let mut code = false;
    loop {
        line = line.trim_start();
        if line.is_empty() {
            return code;
        }
        if *in_comment {
            let Some(end) = line.find("*/") else {
                return code;
            };
            line = &line[end + 2..];
            *in_comment = false;
            continue;
        }
        let multi = line.find("/*").unwrap_or(line.len());
        let single = line.find("//").unwrap_or(line.len());
        code |= multi > 0 && single > 0;
        if multi >= single {
            return code;
        }
        line = &line[multi + 2..];
        *in_comment = true;
    }
}

/// The text of a block from its opening to its closing brace, the braces themselves only when `braces` is set:
/// clippy measures a function's body inside its braces, and an `async fn`'s body with them (its desugared span).
pub fn block_text(source: &str, open: LineColumn, close: LineColumn, braces: bool) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let (from, to) = if braces {
        (open.column, close.column + 1)
    } else {
        (open.column + 1, close.column)
    };
    let (first, last) = (open.line - 1, close.line - 1);
    if first == last {
        return chars(lines[first], from, to);
    }
    let mut text = chars(lines[first], from, usize::MAX);
    for line in &lines[first + 1..last] {
        text.push('\n');
        text.push_str(line);
    }
    text.push('\n');
    text.push_str(&chars(lines[last], 0, to));
    text
}

fn chars(line: &str, from: usize, to: usize) -> String {
    line.chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}
