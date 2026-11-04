//! The budget of the fixed context (`specs/sync.md`, SYN-38, SYN-39): what every session of the project pays before
//! any work, measured on the work tree as `fixed-context` measures a revision (REP-2), against the
//! `fixed_context_budget` of the project's configuration (`config_of`).

use std::fs;
use std::path::Path;

use crate::memory::{measure, WorkTree};
use crate::tools::description_bytes;
use crate::{read_config, CONFIG};

/// The key of the `[project]` table that holds the budget, in bytes.
const KEY: &str = "fixed_context_budget";

/// The warnings on the budget: none without the key, or when the configuration cannot be read (the command says so).
#[must_use]
pub fn warnings(root: &Path) -> Vec<String> {
    let Ok(config) = read_config(root) else {
        return Vec::new();
    };
    let Some(value) = config.get("project").and_then(|p| p.get(KEY)) else {
        return Vec::new();
    };
    let Some(budget) = value.as_integer().and_then(|b| u64::try_from(b).ok()) else {
        return vec![format!(
            "warning: {KEY} in {CONFIG} is not a whole number of bytes"
        )];
    };
    let parts = parts(root);
    let total: u64 = parts.iter().map(|(_, bytes)| bytes).sum();
    if total <= budget {
        return Vec::new();
    }
    // the first of the largest parts, in the order of `parts`
    let [first, rest @ ..] = &parts;
    let (part, bytes) = rest.iter().fold(
        first,
        |largest, part| if part.1 > largest.1 { part } else { largest },
    );
    vec![format!(
        "warning: the fixed context is {total} bytes, over the budget of {budget} ({KEY} in {CONFIG}); the largest part is {part} ({bytes} bytes)"
    )]
}

/// The parts of the fixed context of the work tree at `root`, with their bytes: the memory files as REP-2 counts
/// them (`CLAUDE.md` for all but the `AGENTS.md` files), then the skill descriptions.
fn parts(root: &Path) -> [(&'static str, u64); 3] {
    let memory = measure(&WorkTree(root));
    let descriptions = fs::read_dir(root.join(".claude/skills"))
        .into_iter()
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.ok()?.path().join("SKILL.md")).ok())
        .map(|skill| description_bytes(&skill))
        .sum();
    [
        ("CLAUDE.md", memory.claude),
        ("AGENTS.md", memory.agents),
        ("skill descriptions", descriptions),
    ]
}
