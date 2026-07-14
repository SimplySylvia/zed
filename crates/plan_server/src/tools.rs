//! Tool logic operating on `.plans/<id>.plan.json` via `plan_core`. Kept as
//! plain functions over a plans directory so they unit-test without rmcp/async.

use std::path::Path;

use anyhow::Result;
use plan_core::{Plan, store};

/// Create a new draft plan and persist it atomically. The plan starts in
/// `drafting` at rev 1 with an empty spec/design/tasks; the agent fills it in
/// through the other tools. Building it via a minimal JSON value reuses the
/// schema's own `#[serde(default)]`s instead of hand-listing every field.
pub fn create(plans_dir: &Path, id: &str, title: &str, goal: &str, thread: &str) -> Result<Plan> {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": plan_core::SCHEMA_VERSION,
        "id": id,
        "title": title,
        "status": "drafting",
        "rev": 1,
        "thread": thread,
        "spec": { "goal": goal },
    }))?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Load a plan fresh from disk. Never cached (PRD §1: `plan_get` re-reads on
/// every call so UI/user edits are always reflected).
pub fn get(plans_dir: &Path, id: &str) -> Result<Plan> {
    store::load(plans_dir, id)
}
