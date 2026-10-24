//! The stage keys: per stage a sha256 over what it reads, so a stage whose key has a pass record need not run
//! again (AGENTS.md § How to verify a change, passed stages are reused). Over the stage's name; the path and
//! content of every file it reads (`stage_inputs`), tracked or untracked but not ignored, as the working tree
//! holds it, so uncommitted edits count and a deleted file changes the key; the toolchain; the input
//! variables but the volatile ones. A model stage also covers what it runs on outside the repository, and for
//! it a Cargo file's `version = "…"` lines of the workspace's own packages do not count. The test stage's
//! parts `test@<member>` cover the code inputs but the files of the members its tests do not build
//! (`test@members` names one per member). `VOICEPUNCHER_STAGE_ROOT` names another repository.

mod files;
mod inputs;
mod members;
pub mod rules;
#[cfg(test)]
mod tests;

pub use inputs::Inputs;

use crate::context::Context;
use crate::Outcome;
use members::Workspace;
use rules::{is_model_stage, Rules};
use sha2::{Digest, Sha256};
use std::path::Path;

/// `dev-tools keys <stage>…`: prints `<stage> <key>` per stage.
pub fn command(stages: &[String]) -> Outcome {
    let context = Context::from_env();
    for (stage, key) in of_this_run(&context, stages)? {
        println!("{stage} {key}");
    }
    Ok(0)
}

/// The keys of the stages in this process's environment.
pub fn of_this_run(context: &Context, stages: &[String]) -> Result<Vec<(String, String)>, String> {
    let inputs = Inputs::gather(&context.just, context.table("stage_inputs")?);
    compute(&context.stage_root(), stages, &inputs)
}

/// The key of each stage of the repository at `root`.
pub fn compute(
    root: &Path,
    stages: &[String],
    inputs: &Inputs,
) -> Result<Vec<(String, String)>, String> {
    let listed = files::listed(root)?;
    let tree = Tree {
        root,
        hashes: files::hashed(root, &listed)?,
        listed,
        rules: Rules::parse(&inputs.table),
        workspace: Workspace::read(root),
        inputs,
    };
    tree.workspace
        .expand(stages)
        .into_iter()
        .map(|stage| tree.key(stage))
        .collect()
}

/// What the keys of one repository are computed over.
struct Tree<'a> {
    root: &'a Path,
    listed: Vec<String>,
    hashes: std::collections::BTreeMap<String, String>,
    rules: Rules,
    workspace: Workspace,
    inputs: &'a Inputs,
}

impl Tree<'_> {
    fn key(&self, stage: String) -> Result<(String, String), String> {
        let part = stage.strip_prefix("test@").map(str::to_string);
        if let Some(part) = part
            .as_deref()
            .filter(|part| !self.workspace.is_member(part))
        {
            return Err(format!("{stage}: no workspace member {part}"));
        }
        let model = is_model_stage(&stage);
        let mut sha = Sha256::new();
        sha.update(format!("stage {stage}\n"));
        sha.update(&self.inputs.toolchain);
        sha.update(&self.inputs.variables);
        if model {
            sha.update(&self.inputs.runtime);
        }
        for path in self
            .listed
            .iter()
            .filter(|path| self.rules.feeds(path, &stage) && self.in_part(part.as_deref(), path))
        {
            let content = if model && is_cargo_file(path) {
                files::unversioned(self.root, path)?
            } else {
                self.hashes.get(path).cloned().unwrap_or_default()
            };
            sha.update(format!("{path} {content}\n"));
        }
        Ok((stage, files::hex(&sha.finalize())))
    }

    /// Whether a path feeds a test part: a file outside the members feeds every part.
    fn in_part(&self, part: Option<&str>, path: &str) -> bool {
        match (part, self.workspace.owner(path)) {
            (Some(part), Some(owner)) => self.workspace.in_scope(part, owner),
            _ => true,
        }
    }
}

fn is_cargo_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == "Cargo.toml" || name == "Cargo.lock"
}
