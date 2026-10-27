//! The `justfile` stays thin: recipes over `dev-tools`, no Perl, under 800 lines, and every `|| true` says
//! why the failure it hides does not matter.

const JUSTFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../justfile"));

#[test]
fn should_hold_no_perl() {
    let perl: Vec<&str> = JUSTFILE
        .lines()
        .filter(|line| {
            line.contains("#!/usr/bin/env perl")
                || line.split_whitespace().any(|word| word == "perl")
        })
        .collect();
    assert!(perl.is_empty(), "Perl in the justfile: {perl:#?}");
}

#[test]
fn should_stay_under_800_lines() {
    let lines = JUSTFILE.lines().count();
    assert!(lines < 800, "the justfile has {lines} lines");
}

#[test]
fn should_update_the_checkout_for_the_app_without_git_pull() {
    let recipe: Vec<&str> = JUSTFILE
        .lines()
        .skip_while(|line| !line.starts_with("update-app:"))
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .collect();
    assert!(
        recipe
            .first()
            .is_some_and(|line| line.contains("{{dev_tools}} update-checkout")),
        "update-app: {recipe:#?}"
    );
    assert!(
        !recipe.iter().any(|line| line.contains("git pull")),
        "update-app: {recipe:#?}"
    );
}

/// The lines of a justfile that end a command in `|| true` with no comment on them or on the line above.
fn uncommented_ignores(justfile: &str) -> Vec<String> {
    let lines: Vec<&str> = justfile.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("|| true"))
        .filter(|(at, line)| {
            let above = at.checked_sub(1).map_or("", |above| lines[above]);
            let after = line.split("|| true").last().unwrap_or("");
            !line.trim_start().starts_with('#')
                && !after.contains('#')
                && !above.trim_start().starts_with('#')
        })
        .map(|(at, line)| format!("{}: {line}", at + 1))
        .collect()
}

#[test]
fn should_say_why_each_ignored_failure_does_not_matter() {
    let silent = uncommented_ignores(JUSTFILE);
    assert!(
        silent.is_empty(),
        "`|| true` without a comment on it or above it: {silent:#?}"
    );
}

#[test]
fn should_take_a_comment_on_the_line_or_above_it_and_name_a_line_without_one() {
    let justfile = "a:\n    # the folder may be gone\n    rm -r x || true\n    rm y || true  # y is optional\n    rm z || true\n";
    assert_eq!(uncommented_ignores(justfile), ["5:     rm z || true"]);
}

/// The recipe each `just(&[…])` call of a step file names first: a string literal, or a `const` of the file.
fn called_recipes(source: &str) -> Vec<String> {
    let constant = |name: &str| {
        let start = format!("const {name}: &str = \"");
        source
            .split(&start)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .map(str::to_string)
    };
    source
        .split("just(")
        .skip(1)
        .filter_map(|rest| rest.trim_start().strip_prefix("&["))
        .filter_map(|rest| {
            let first = rest.trim_start();
            first.strip_prefix('"').map_or_else(
                || constant(first.split([',', ']']).next().unwrap_or("").trim()),
                |literal| literal.split('"').next().map(str::to_string),
            )
        })
        .filter(|recipe| !recipe.starts_with('-'))
        .collect()
}

#[test]
fn should_have_every_recipe_the_test_steps_call() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let recipes: Vec<&str> = JUSTFILE
        .lines()
        .filter(|line| {
            !line.starts_with([' ', '#', '\t']) && line.contains(':') && !line.contains(":=")
        })
        .filter_map(|line| line.trim_start_matches('@').split([' ', ':']).next())
        .collect();
    let mut missing = Vec::new();
    let mut files = rust_files(&root.join("e2e-test/tests"));
    files.extend(rust_files(&root.join("app/tests")));
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        for recipe in called_recipes(&source) {
            if !recipes.contains(&recipe.as_str()) {
                missing.push(format!("{}: {recipe}", path.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "recipes the steps call but the justfile lacks: {missing:#?}"
    );
}

#[test]
fn should_read_the_recipe_of_a_literal_and_of_a_constant() {
    let source = "const GUARD: &str = \"_guard\";\nfn f() { just(&[\"_engines\", a], None); x.just(\n    &[GUARD, b]); just(&[\"--show\", c]); }";
    assert_eq!(called_recipes(source), ["_engines", "_guard"]);
}

/// Every `.rs` file under a folder.
fn rust_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().map(Result::unwrap) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}
