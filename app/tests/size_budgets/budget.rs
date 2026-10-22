//! The budgets (AGENTS.md § Architecture rules) and the measures one run collects.

use std::collections::{BTreeMap, BTreeSet};

/// What a budget limits. The first six are measured here; the last two only clippy measures, so the baseline
/// lists their units without a measure and the size-baseline attribute is their whole ratchet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    FunctionLines,
    Parameters,
    FileLines,
    StructFields,
    PublicMembers,
    DirectoryLines,
    Nesting,
    Complexity,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::FunctionLines,
        Kind::Parameters,
        Kind::FileLines,
        Kind::StructFields,
        Kind::PublicMembers,
        Kind::DirectoryLines,
        Kind::Nesting,
        Kind::Complexity,
    ];

    /// The largest measure within budget, or `None` for a kind only clippy measures.
    pub fn budget(self) -> Option<usize> {
        match self {
            Kind::FunctionLines => Some(40),
            Kind::Parameters => Some(5),
            Kind::FileLines => Some(300),
            Kind::StructFields => Some(12),
            Kind::PublicMembers => Some(35),
            Kind::DirectoryLines => Some(2000),
            Kind::Nesting | Kind::Complexity => None,
        }
    }

    /// The table of the baseline file that lists this kind's units.
    pub fn key(self) -> &'static str {
        match self {
            Kind::FunctionLines => "function_lines",
            Kind::Parameters => "parameters",
            Kind::FileLines => "file_lines",
            Kind::StructFields => "struct_fields",
            Kind::PublicMembers => "public_members",
            Kind::DirectoryLines => "directory_lines",
            Kind::Nesting => "excessive_nesting",
            Kind::Complexity => "cognitive_complexity",
        }
    }

    /// The clippy lint a size-baseline attribute names for this kind, when clippy measures it.
    pub fn from_lint(lint: &str) -> Option<Kind> {
        match lint {
            "too_many_lines" => Some(Kind::FunctionLines),
            "too_many_arguments" => Some(Kind::Parameters),
            "excessive_nesting" => Some(Kind::Nesting),
            "cognitive_complexity" => Some(Kind::Complexity),
            _ => None,
        }
    }

    pub fn has_lint(self) -> bool {
        matches!(
            self,
            Kind::FunctionLines | Kind::Parameters | Kind::Nesting | Kind::Complexity
        )
    }

    pub fn unit_name(self) -> &'static str {
        match self {
            Kind::FunctionLines => "code lines",
            Kind::Parameters => "parameters",
            Kind::FileLines => "code lines",
            Kind::StructFields => "fields",
            Kind::PublicMembers => "public members",
            Kind::DirectoryLines => "code lines",
            Kind::Nesting | Kind::Complexity => "",
        }
    }
}

/// Everything one run found: each measured unit with its measure, and each size-baseline attribute.
#[derive(Debug, Default)]
pub struct Measures {
    pub units: BTreeMap<(Kind, String), usize>,
    pub markers: BTreeSet<(Kind, String)>,
}

impl Measures {
    /// Records a measure; two units under one identifier keep the larger.
    pub fn record(&mut self, kind: Kind, id: String, measure: usize) {
        let entry = self.units.entry((kind, id)).or_insert(0);
        *entry = (*entry).max(measure);
    }

    pub fn merge(&mut self, other: Measures) {
        for ((kind, id), measure) in other.units {
            self.record(kind, id, measure);
        }
        self.markers.extend(other.markers);
    }
}
