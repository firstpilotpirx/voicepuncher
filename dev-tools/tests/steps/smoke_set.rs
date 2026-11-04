//! The smoke set of the services and speech capabilities: at most one `@smoke` scenario per rule with
//! several, the rest in a set of the full check (`@regression`, or a stage of their own: `@seams`,
//! `@resilience`, `@cues`), so the check runs each rule at most once and `full-check` the whole set.

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

/// The tags of the sets only the full check runs.
const FULL_CHECK_TAGS: [&str; 4] = ["regression", "seams", "resilience", "cues"];

/// The scenarios of one rule: how many, how many in the smoke set and how many in a set of the full check.
struct Rule {
    name: String,
    scenarios: usize,
    smoke: usize,
    full: usize,
}

/// The sets a tag line puts a scenario, rule or feature into.
#[derive(Default, Clone, Copy)]
struct Sets {
    smoke: bool,
    full: bool,
}

/// The scenarios of each rule and how many of them are in the smoke set and in a set of the full check.
#[derive(Default)]
struct Counter {
    rules: Vec<Rule>,
    pending: Sets,
    feature: Sets,
    rule: Sets,
}

impl Counter {
    fn tags(&mut self, tags: &str) {
        for tag in tags
            .split_whitespace()
            .map(|tag| tag.trim_start_matches('@'))
        {
            self.pending.smoke |= tag == "smoke";
            self.pending.full |= FULL_CHECK_TAGS.contains(&tag);
        }
    }

    fn rule(&mut self, file: &str, name: &str) {
        self.rule = std::mem::take(&mut self.pending);
        self.rules
            .push(Rule { name: format!("{file}: {name}"), scenarios: 0, smoke: 0, full: 0 });
    }

    /// A scenario is in a set when it, its rule or its feature carries the set's tag.
    fn scenario(&mut self) {
        let own = std::mem::take(&mut self.pending);
        let smoke = self.feature.smoke || self.rule.smoke || own.smoke;
        let full = self.feature.full || self.rule.full || own.full;
        if let Some(last) = self.rules.last_mut() {
            last.scenarios += 1;
            last.smoke += usize::from(smoke);
            last.full += usize::from(full);
        }
    }

    /// `<file>: <rule>: <smoke> smoke, <full> full of <scenarios>` per rule.
    fn lines(self) -> Vec<String> {
        self.rules
            .into_iter()
            .map(|rule| {
                format!(
                    "{}: {} smoke, {} full of {}",
                    rule.name, rule.smoke, rule.full, rule.scenarios
                )
            })
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
            Line::Other => counter.pending = Sets::default(),
        }
    }
    counter.lines()
}

/// Whether a counted rule has several scenarios and more than one in the smoke set, or one in no
/// set at all.
fn over(line: &str) -> bool {
    let count = |text: &str, what: &str| -> usize {
        text.trim()
            .parse()
            .unwrap_or_else(|_| panic!("{what} in {line}"))
    };
    let (head, scenarios) = line.rsplit_once(" of ").expect("a scenario count");
    let (head, full) = head.rsplit_once(", ").expect("a full check count");
    let (_, smoke) = head.rsplit_once(": ").expect("a smoke count");
    let smoke = count(smoke.trim_end_matches(" smoke"), "smoke");
    let full = count(full.trim_end_matches(" full"), "full");
    let scenarios = count(scenarios, "scenarios");
    scenarios > 1 && (smoke > 1 || smoke + full != scenarios)
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

#[then(
    "every rule with several scenarios has at most one smoke scenario and the rest in the full check"
)]
fn one_smoke_per_rule(world: &mut WorkflowWorld) {
    let over: Vec<&str> = world.text.lines().filter(|line| over(line)).collect();
    assert!(
        over.is_empty(),
        "rules with several smoke scenarios or a scenario in no set:\n{}",
        over.join("\n")
    );
}
