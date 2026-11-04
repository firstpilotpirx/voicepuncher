//! The smoke set of the services and speech capabilities: at most one `@smoke` scenario per rule, the rest
//! `@regression`, so the check runs each rule once and `full-check` the whole set.

use cucumber::{given, then, when};

use super::repo_file;
use crate::WorkflowWorld;

const FEATURES: [&str; 2] = [
    "e2e-test/tests/features/local_services.feature",
    "e2e-test/tests/features/speech_recognition.feature",
];

enum Line<'a> {
    Tags(&'a str),
    Feature,
    Rule(&'a str),
    Scenario,
    Comment,
    Other,
}

fn classify(line: &str) -> Line<'_> {
    if let Some(tags) = line.strip_prefix('@') {
        Line::Tags(tags)
    } else if line.starts_with("Feature:") {
        Line::Feature
    } else if let Some(name) = line.strip_prefix("Rule:") {
        Line::Rule(name.trim())
    } else if line.starts_with("Scenario") {
        Line::Scenario
    } else if line.starts_with('#') {
        Line::Comment
    } else {
        Line::Other
    }
}

/// The scenarios of each rule and how many of them are in the smoke set.
#[derive(Default)]
struct Counter {
    rules: Vec<(String, usize, usize)>,
    pending: bool,
    feature: bool,
    rule: bool,
}

impl Counter {
    fn tags(&mut self, tags: &str) {
        self.pending |= tags.split_whitespace().any(|tag| tag == "smoke");
    }

    fn rule(&mut self, file: &str, name: &str) {
        self.rule = std::mem::take(&mut self.pending);
        self.rules.push((format!("{file}: {name}"), 0, 0));
    }

    /// A scenario is in the smoke set when it, its rule or its feature carries `@smoke`.
    fn scenario(&mut self) {
        let smoke = self.feature || self.rule || std::mem::take(&mut self.pending);
        if let Some(last) = self.rules.last_mut() {
            last.1 += 1;
            last.2 += usize::from(smoke);
        }
    }

    /// `<file>: <rule>: <smoke> of <scenarios>` per rule.
    fn lines(self) -> Vec<String> {
        self.rules
            .into_iter()
            .map(|(rule, scenarios, smoke)| format!("{rule}: {smoke} of {scenarios}"))
            .collect()
    }
}

fn rules_of(file: &str, text: &str) -> Vec<String> {
    let mut counter = Counter::default();
    for line in text.lines().map(str::trim) {
        match classify(line) {
            Line::Tags(tags) => counter.tags(tags),
            Line::Feature => counter.feature = std::mem::take(&mut counter.pending),
            Line::Rule(name) => counter.rule(file, name),
            Line::Scenario => counter.scenario(),
            Line::Comment => {}
            Line::Other => counter.pending = false,
        }
    }
    counter.lines()
}

/// Whether a counted rule has several scenarios and more than one of them in the smoke set.
fn over(line: &str) -> bool {
    let (head, scenarios) = line.rsplit_once(" of ").expect("a scenario count");
    let (_, smoke) = head.rsplit_once(": ").expect("a smoke count");
    scenarios.parse::<usize>().expect("scenarios") > 1 && smoke.parse::<usize>().expect("smoke") > 1
}

#[given("the feature files of the local-services and speech-recognition capabilities")]
fn feature_files(world: &mut WorkflowWorld) {
    world.text = FEATURES.join("\n");
}

#[when("the smoke scenarios of each rule are counted")]
fn count_smoke(world: &mut WorkflowWorld) {
    let counted: Vec<String> = world
        .text
        .lines()
        .flat_map(|file| rules_of(file, &repo_file(file)))
        .collect();
    world.text = counted.join("\n");
}

#[then("every rule with several scenarios has one smoke scenario")]
fn one_smoke_per_rule(world: &mut WorkflowWorld) {
    let over: Vec<&str> = world.text.lines().filter(|line| over(line)).collect();
    assert!(
        over.is_empty(),
        "rules with several smoke scenarios:\n{}",
        over.join("\n")
    );
}
