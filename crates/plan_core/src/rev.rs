//! Staged revisions (F9.3): the agent proposes a plan change as a *pending diff*
//! rather than mutating the plan; the user applies or rejects it per hunk in the
//! UI (mirroring Zed's Keep/Reject). Staging is inert — no block is touched and
//! `rev` is not bumped until the revision resolves. This is the same "proposed
//! diff" primitive that execution-time amendments (F4.7) reuse.
//!
//! Pure functions on `&mut Plan`, shared by `plan_ui` (which applies/rejects) and
//! `plan_server` (which stages via `plan_propose_revision`). Block addressing
//! reuses [`crate::anchor::set_block_text`].

use serde::Deserialize;

use crate::Plan;
use crate::schema::{Hunk, HistoryEntry, PendingRevision};

/// A hunk as proposed by the agent, before ids/state are assigned. Deserializable
/// so `plan_server` can accept it straight from the tool's JSON arguments.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HunkSpec {
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub old: Option<String>,
    #[serde(default)]
    pub new: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
}

/// Stage a pending revision from `specs`, assigning hunk ids (`h1`..) and the
/// `pending` state. Records the rev the revision *will* become. Inert: no block
/// is changed and `rev` is unchanged until [`resolve_revision`].
pub fn stage_revision(plan: &mut Plan, specs: Vec<HunkSpec>) {
    let hunks = specs
        .into_iter()
        .enumerate()
        .map(|(index, spec)| Hunk {
            id: format!("h{}", index + 1),
            target: spec.target,
            old: spec.old,
            new: spec.new,
            from: spec.from,
            state: Some("pending".to_string()),
            extra: Default::default(),
        })
        .collect();
    plan.pending_revision = Some(PendingRevision {
        rev: Some(plan.rev + 1),
        hunks,
        extra: Default::default(),
    });
}

/// Apply one pending hunk: write its `new` text to the target block and mark it
/// `applied`. Returns false if there is no pending revision, the id is unknown,
/// the hunk has no target/new text, or the target block doesn't resolve.
pub fn apply_hunk(plan: &mut Plan, hunk_id: &str) -> bool {
    let Some(pending) = plan.pending_revision.as_ref() else {
        return false;
    };
    let target = pending.hunks.iter().find(|hunk| hunk.id == hunk_id).and_then(|hunk| {
        Some((hunk.target.clone()?, hunk.new.clone()?))
    });
    let Some((block, new_text)) = target else {
        return false;
    };
    if !crate::anchor::set_block_text(plan, &block, &new_text) {
        return false;
    }
    set_hunk_state(plan, hunk_id, "applied")
}

/// Reject one pending hunk: mark it `rejected`, leaving its block untouched.
/// Returns false if there is no pending revision or the id is unknown.
pub fn reject_hunk(plan: &mut Plan, hunk_id: &str) -> bool {
    set_hunk_state(plan, hunk_id, "rejected")
}

fn set_hunk_state(plan: &mut Plan, hunk_id: &str, state: &str) -> bool {
    let Some(pending) = plan.pending_revision.as_mut() else {
        return false;
    };
    match pending.hunks.iter_mut().find(|hunk| hunk.id == hunk_id) {
        Some(hunk) => {
            hunk.state = Some(state.to_string());
            true
        }
        None => false,
    }
}

/// Commit or discard the pending revision once no hunk is still `pending`. If any
/// hunk was applied, bump `rev`, stamp history, and record provenance (each
/// applied hunk's target lands in its source comment's `caused_changes`, marking
/// that comment `addressed`, per F3.5). Clears the pending revision. Returns the
/// committed rev, or `None` if nothing was applied or a hunk is still pending.
pub fn resolve_revision(plan: &mut Plan) -> Option<u32> {
    let pending = plan.pending_revision.as_ref()?;
    if pending
        .hunks
        .iter()
        .any(|hunk| hunk.state.as_deref() == Some("pending"))
    {
        return None;
    }
    let target_rev = pending.rev.unwrap_or(plan.rev + 1);
    let applied: Vec<(Option<String>, Option<String>)> = pending
        .hunks
        .iter()
        .filter(|hunk| hunk.state.as_deref() == Some("applied"))
        .map(|hunk| (hunk.from.clone(), hunk.target.clone()))
        .collect();

    plan.pending_revision = None;
    if applied.is_empty() {
        return None;
    }

    plan.rev = target_rev;
    plan.history.push(HistoryEntry {
        rev: Some(plan.rev),
        by: Some("agent".to_string()),
        kind: Some("revision".to_string()),
        summary: Some(format!("applied {} hunk(s)", applied.len())),
        at: None,
        extra: Default::default(),
    });

    for (from, target) in applied {
        let (Some(from), Some(target)) = (from, target) else {
            continue;
        };
        if let Some(comment) = plan.comments.iter_mut().find(|comment| comment.id == from) {
            if !comment.caused_changes.iter().any(|change| change == &target) {
                comment.caused_changes.push(target);
            }
            comment.state = Some("addressed".to_string());
        }
    }
    Some(plan.rev)
}

/// Apply every pending hunk, then resolve. Returns the committed rev.
pub fn apply_all(plan: &mut Plan) -> Option<u32> {
    let ids: Vec<String> = plan
        .pending_revision
        .as_ref()?
        .hunks
        .iter()
        .filter(|hunk| hunk.state.as_deref() == Some("pending"))
        .map(|hunk| hunk.id.clone())
        .collect();
    for id in ids {
        apply_hunk(plan, &id);
    }
    resolve_revision(plan)
}

/// Reject every pending hunk, then resolve (clears with no rev bump).
pub fn reject_all(plan: &mut Plan) -> Option<u32> {
    let ids: Vec<String> = plan
        .pending_revision
        .as_ref()?
        .hunks
        .iter()
        .filter(|hunk| hunk.state.as_deref() == Some("pending"))
        .map(|hunk| hunk.id.clone())
        .collect();
    for id in ids {
        reject_hunk(plan, &id);
    }
    resolve_revision(plan)
}
