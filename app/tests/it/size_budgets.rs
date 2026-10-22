//! Size and complexity budgets (AGENTS.md § Architecture rules): every unit of workspace Rust code stays within
//! its budget or is listed in `app/tests/size_baseline.toml`, and a listed unit never grows.
//! `VOICEPUNCHER_WRITE_SIZE_BASELINE=1` rewrites the baseline from the code instead (`just size-baseline`).

mod baseline;
mod budget;
mod fixtures;
mod lines;
mod measure;
mod verdict;
mod workspace;

const BASELINE: &str = "app/tests/size_baseline.toml";

#[test]
fn workspace_stays_within_size_budgets() {
    let root = workspace::root();
    let (measures, suppressions) = workspace::measure(&root);
    let path = root.join(BASELINE);
    if std::env::var("VOICEPUNCHER_WRITE_SIZE_BASELINE").as_deref() == Ok("1") {
        let text = baseline::render(&baseline::from_measures(&measures));
        std::fs::write(&path, text).expect("write the size baseline");
        return;
    }
    let listed = baseline::parse(&std::fs::read_to_string(&path).unwrap_or_default());
    let mut found = verdict::violations(&measures, &listed);
    found.extend(
        suppressions
            .iter()
            .map(|s| format!("{s} silences a budget lint outside the size baseline")),
    );
    assert!(
        found.is_empty(),
        "size budgets (AGENTS.md § Architecture rules), baseline {BASELINE}:\n{}",
        found.join("\n")
    );
}
