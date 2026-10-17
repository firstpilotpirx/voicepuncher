//! Which scenarios a Gherkin harness runs: those its command line selects (`--name`, `--tags` or
//! `CUCUMBER_FILTER_TAGS`) that every gate of the harness admits. A gate holds a tag back: `@wip`
//! always, a tag that needs something on the machine while that is missing.

use cucumber::cli::{Args, Opts};
use cucumber::gherkin::tagexpr::TagOperation;
use cucumber::gherkin::{Feature, Rule, Scenario};
use cucumber::tag::Ext as _;
use regex::Regex;

const VOICE_SAMPLES_VARIABLE: &str = "VOICEPUNCHER_VOICE_SAMPLES";
const VOICE_SAMPLES_TAG: &str = "voice_samples";
const REAL_KEYBOARD_VARIABLE: &str = "VOICEPUNCHER_REAL_KEYBOARD_IT";
const REAL_KEYBOARD_TAG: &str = "real_keyboard";
const WIP_TAG: &str = "wip";

/// A tag whose scenarios run only while the gate is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    tag: &'static str,
    open: bool,
    line: Option<String>,
}

impl Gate {
    /// `@wip`: never runs.
    pub fn wip() -> Self {
        Self {
            tag: WIP_TAG,
            open: false,
            line: None,
        }
    }

    /// `@voice_samples`: runs where the user's recordings exist, from the value of
    /// `VOICEPUNCHER_VOICE_SAMPLES` (`std::env::var`).
    /// Unset and empty both close it, each with its own line.
    pub fn voice_samples(value: Result<String, std::env::VarError>) -> Self {
        let state = match value {
            Ok(dir) if !dir.is_empty() => None,
            Ok(_) => Some("empty"),
            Err(_) => Some("not set"),
        };
        Self::closed_by_variable(VOICE_SAMPLES_TAG, VOICE_SAMPLES_VARIABLE, state)
    }

    /// `@real_keyboard`: runs with `VOICEPUNCHER_REAL_KEYBOARD_IT` set to any value.
    pub fn real_keyboard(value: Result<String, std::env::VarError>) -> Self {
        let state = value.is_err().then_some("not set");
        Self::closed_by_variable(REAL_KEYBOARD_TAG, REAL_KEYBOARD_VARIABLE, state)
    }

    /// `closed_because`: why the variable closes the gate ("not set", "empty"), `None` when open.
    fn closed_by_variable(tag: &'static str, variable: &str, closed_because: Option<&str>) -> Self {
        let line = closed_because
            .map(|reason| format!("GATED: {variable} {reason}, skipping @{tag} scenarios"));
        Self {
            tag,
            open: line.is_none(),
            line,
        }
    }

    /// Whether the gate is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The `GATED:` line that says why the gate is closed, if a variable closes it.
    pub fn line(&self) -> Option<&str> {
        self.line.as_deref()
    }

    fn admits(&self, tags: &[&str]) -> bool {
        self.open || !tags.contains(&self.tag)
    }
}

/// The scenarios a command line selects, as cucumber reads its options.
#[derive(Debug)]
pub enum Selection {
    /// No `--name` and no `--tags`.
    Everything,
    /// `--name`: the scenario name matches the regex.
    Name(Regex),
    /// `--tags` or `CUCUMBER_FILTER_TAGS`: the merged tags satisfy the expression.
    Tags(TagOperation),
}

impl Selection {
    fn selects(&self, scenario: &Scenario, tags: &[&str]) -> bool {
        match self {
            Self::Everything => true,
            Self::Name(name) => name.is_match(&scenario.name),
            Self::Tags(expression) => expression.eval(tags.iter().copied()),
        }
    }
}

/// Decides, per scenario, whether it runs.
#[derive(Debug)]
pub struct ScenarioFilter {
    selection: Selection,
    gates: Vec<Gate>,
}

impl ScenarioFilter {
    /// Whether the scenario runs: its selection picks it and every gate admits it.
    pub fn runs(&self, feature: &Feature, rule: Option<&Rule>, scenario: &Scenario) -> bool {
        let tags = merged_tags(feature, rule, scenario);
        self.selection.selects(scenario, &tags) && self.gates.iter().all(|g| g.admits(&tags))
    }

    /// The `GATED:` lines of the closed gates, for the harness to print once.
    pub fn gated_lines(&self) -> impl Iterator<Item = &str> {
        self.gates.iter().filter_map(Gate::line)
    }
}

/// Splits cucumber's options into the options to hand cucumber, without `--name` and `--tags`, and
/// the filter that requires that selection and every gate.
///
/// cucumber applies a selection it is given instead of the harness's filter, not beside it, so a
/// gate would not hold under `--tags`, `--name` or `CUCUMBER_FILTER_TAGS`. Every field is named:
/// a cucumber release that adds one, a new kind of filter for instance, fails to compile here.
pub fn split<P, R, W, C>(
    opts: Opts<P, R, W, C>,
    gates: Vec<Gate>,
) -> (Opts<P, R, W, C>, ScenarioFilter)
where
    P: Args,
    R: Args,
    W: Args,
    C: Args,
{
    let Opts {
        re_filter,
        tags_filter,
        parser,
        runner,
        writer,
        custom,
    } = opts;
    // The precedence of cucumber's `filter_run`: the name, then the tags, then everything.
    let selection = match (re_filter, tags_filter) {
        (Some(name), _) => Selection::Name(name),
        (None, Some(expression)) => Selection::Tags(expression),
        (None, None) => Selection::Everything,
    };
    let opts = Opts {
        re_filter: None,
        tags_filter: None,
        parser,
        runner,
        writer,
        custom,
    };
    (opts, ScenarioFilter { selection, gates })
}

/// Whether the scenario carries `tag` (without `@`) on itself, its rule or its feature, Examples
/// tags included: the tags a gate and a `--tags` selection read.
pub fn has_tag(feature: &Feature, rule: Option<&Rule>, scenario: &Scenario, tag: &str) -> bool {
    merged_tags(feature, rule, scenario).contains(&tag)
}

fn merged_tags<'a>(
    feature: &'a Feature,
    rule: Option<&'a Rule>,
    scenario: &'a Scenario,
) -> Vec<&'a str> {
    feature
        .tags
        .iter()
        .chain(rule.iter().flat_map(|r| &r.tags))
        .chain(&scenario.tags)
        .map(String::as_str)
        .collect()
}

#[cfg(test)]
mod tests;
