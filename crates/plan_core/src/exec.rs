//! Execution lifecycle (F4.1, F11.3): launch a plan into `executing` and manage the
//! single executor lease. Pure functions on `&mut Plan`, shared by the UI (Launch
//! button) and the server (`plan_launch`); mutations rev-bump + stamp history.
//!
//! Git branch/worktree setup (F10.2) is **M7** — launch here only transitions plan
//! state + lease. The agent's ACP ExitPlanMode ↔ launch reconciliation is a seam
//! handled by the skill (see docs/milestones/M6a.md).

use anyhow::{Result, bail};

use crate::Plan;
use crate::schema::{Executor, HistoryEntry, Status};

fn bump(plan: &mut Plan, by: &str, kind: &str, summary: String) {
    plan.rev += 1;
    plan.history.push(HistoryEntry {
        rev: Some(plan.rev),
        by: Some(by.to_string()),
        kind: Some(kind.to_string()),
        summary: Some(summary),
        at: None,
        extra: Default::default(),
    });
}

/// Launch the plan (F4.1): transition `approved → executing` and take the executor
/// lease for `thread` (F11.3 — one thread executes). Re-launching while already
/// executing under the same thread is an idempotent no-op. Errors if another thread
/// holds the lease, or if the plan isn't `approved`. `taken_at` stays `None` here
/// (time-free); a caller with a clock may stamp it.
pub fn launch(plan: &mut Plan, thread: &str) -> Result<()> {
    if let Some(executor) = &plan.executor {
        if executor.thread != thread {
            bail!(
                "plan {} is leased by another thread ({}) — reclaim before launching",
                plan.id,
                executor.thread
            );
        }
        if plan.status == Status::Executing {
            return Ok(());
        }
    }
    if plan.status != Status::Approved {
        bail!(
            "plan {} is {:?}, can only launch from approved",
            plan.id,
            plan.status
        );
    }
    plan.status = Status::Executing;
    plan.executor = Some(Executor {
        thread: thread.to_string(),
        taken_at: None,
        extra: Default::default(),
    });
    bump(plan, thread, "launch", format!("launched — {thread} executing"));
    Ok(())
}

/// Release the executor lease. Paired with pause/stop transitions in M6c; here it
/// just clears the lease.
pub fn release_lease(plan: &mut Plan) {
    plan.executor = None;
}
