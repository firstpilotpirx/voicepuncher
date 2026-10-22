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
    pub const ALL: [Self; 8] = [
        Self::FunctionLines,
        Self::Parameters,
        Self::FileLines,
        Self::StructFields,
        Self::PublicMembers,
        Self::DirectoryLines,
        Self::Nesting,
        Self::Complexity,
    ];

    /// The largest measure within budget, or `None` for a kind only clippy measures.
    pub fn budget(self) -> Option<usize> {
        match self {
            Self::FunctionLines => Some(40),
            Self::Parameters => Some(5),
            Self::FileLines => Some(300),
            Self::StructFields => Some(12),
            Self::PublicMembers => Some(35),
            Self::DirectoryLines => Some(2000),
            Self::Nesting | Self::Complexity => None,
        }
    }

    /// The table of the baseline file that lists this kind's units.
    pub fn key(self) -> &'static str {
        match self {
            Self::FunctionLines => "function_lines",
            Self::Parameters => "parameters",
            Self::FileLines => "file_lines",
            Self::StructFields => "struct_fields",
            Self::PublicMembers => "public_members",
            Self::DirectoryLines => "directory_lines",
            Self::Nesting => "excessive_nesting",
            Self::Complexity => "cognitive_complexity",
        }
    }

    /// The clippy lint a size-baseline attribute names for this kind, when clippy measures it.
    pub fn from_lint(lint: &str) -> Option<Self> {
        match lint {
            "too_many_lines" => Some(Self::FunctionLines),
            "too_many_arguments" => Some(Self::Parameters),
            "excessive_nesting" => Some(Self::Nesting),
            "cognitive_complexity" => Some(Self::Complexity),
            _ => None,
        }
    }

    pub fn has_lint(self) -> bool {
        matches!(
            self,
            Self::FunctionLines | Self::Parameters | Self::Nesting | Self::Complexity
        )
    }

    pub fn unit_name(self) -> &'static str {
        match self {
            Self::FunctionLines => "code lines",
            Self::Parameters => "parameters",
            Self::FileLines => "code lines",
            Self::StructFields => "fields",
            Self::PublicMembers => "public members",
            Self::DirectoryLines => "code lines",
            Self::Nesting | Self::Complexity => "",
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

    pub fn merge(&mut self, other: Self) {
        for ((kind, id), measure) in other.units {
            self.record(kind, id, measure);
        }
        self.markers.extend(other.markers);
    }
}
