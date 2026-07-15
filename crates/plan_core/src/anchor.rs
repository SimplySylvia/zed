//! Fuzzy re-anchoring of comment anchors across plan revisions (F3.1, PRD §7).
//!
//! An anchor addresses a block (`t2`, `t2.s1`, `a1`, …) and preserves the quoted
//! text at anchor time. As the plan is revised, [`reanchor`] classifies whether
//! the anchor still holds, went outdated (text rewritten), moved (block renamed
//! but the quote survives), or detached (both gone).

use crate::{Plan, schema::Anchor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReanchorResult {
    /// The block exists and still contains the quoted text.
    Anchored,
    /// The block exists but the quoted text is gone (rewritten) — the original
    /// quote should be preserved and shown with an "outdated" badge.
    Outdated,
    /// The block id no longer resolves, but the quote was found elsewhere.
    Moved { block: String },
    /// Neither the block nor the quote could be found.
    Detached,
}

/// Re-anchor `anchor` against the current `plan`.
pub fn reanchor(anchor: &Anchor, plan: &Plan) -> ReanchorResult {
    let Some(block) = anchor.block.as_deref() else {
        return ReanchorResult::Detached;
    };
    let quote = anchor.quote.as_deref().unwrap_or_default();

    match block_text(plan, block) {
        Some(text) => {
            if quote.is_empty() || text.contains(quote) {
                ReanchorResult::Anchored
            } else {
                ReanchorResult::Outdated
            }
        }
        None => match (!quote.is_empty())
            .then(|| find_block_with_quote(plan, quote))
            .flatten()
        {
            Some(block) => ReanchorResult::Moved { block },
            None => ReanchorResult::Detached,
        },
    }
}

/// The current text of a block id (`t<id>` title, `t<id>.s<id>` step text, or
/// `a<id>` acceptance when/shall), or `None` if it no longer resolves.
fn block_text(plan: &Plan, block: &str) -> Option<String> {
    if let Some((task_id, step_id)) = block.split_once('.') {
        let task = plan.tasks.iter().find(|task| task.id == task_id)?;
        let step = task.steps.iter().find(|step| step.id == step_id)?;
        return step.text.clone();
    }
    if let Some(task) = plan.tasks.iter().find(|task| task.id == block) {
        return task.title.clone();
    }
    if let Some(acceptance) = plan.spec.acceptance.iter().find(|a| a.id == block) {
        return Some(acceptance_text(acceptance));
    }
    None
}

/// The block id whose current text contains `quote`, searched over tasks, steps,
/// and acceptance criteria.
fn find_block_with_quote(plan: &Plan, quote: &str) -> Option<String> {
    for task in &plan.tasks {
        if task
            .title
            .as_deref()
            .is_some_and(|title| title.contains(quote))
        {
            return Some(task.id.clone());
        }
        for step in &task.steps {
            if step.text.as_deref().is_some_and(|text| text.contains(quote)) {
                return Some(format!("{}.{}", task.id, step.id));
            }
        }
    }
    plan.spec
        .acceptance
        .iter()
        .find(|a| acceptance_text(a).contains(quote))
        .map(|a| a.id.clone())
}

fn acceptance_text(acceptance: &crate::schema::Acceptance) -> String {
    format!(
        "{} {}",
        acceptance.when.as_deref().unwrap_or_default(),
        acceptance.shall.as_deref().unwrap_or_default()
    )
    .trim()
    .to_string()
}
