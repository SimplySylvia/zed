//! Execution lifecycle (F4.1, F11.3): launch a plan into `executing` and manage the
//! single executor lease. Pure functions on `&mut Plan`, shared by the UI (Launch
//! button) and the server (`plan_launch`); mutations rev-bump + stamp history.
//!
//! Git branch/worktree setup (F10.2) is **M7** — launch here only transitions plan
//! state + lease. The agent's ACP ExitPlanMode ↔ launch reconciliation is a seam
//! handled by the skill (see docs/milestones/M6a.md).

use std::path::Path;

use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::Plan;
use crate::schema::{Evidence, Executor, Guard, HistoryEntry, Status, TaskStatus};

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

// ── Gates + step guards (F4.5 / F4.5b) ───────────────────────────────────────

/// The active hold blocking execution — a holding step guard or a pending GATE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub task: String,
    pub step: Option<String>,
    /// "approve" | "input" (step guard) or "gate".
    pub kind: String,
}

fn guard_mut<'plan>(plan: &'plan mut Plan, task: &str, step: &str) -> Option<&'plan mut Guard> {
    plan.tasks
        .iter_mut()
        .find(|candidate| candidate.id == task)?
        .steps
        .iter_mut()
        .find(|candidate| candidate.id == step)?
        .guard
        .as_mut()
}

/// Mark a step guard as `holding` (F4.5b) — the agent calls this when it reaches a
/// guarded step, so the hook blocks until the user clears it.
pub fn hold_guard(plan: &mut Plan, task: &str, step: &str) -> Result<()> {
    match guard_mut(plan, task, step) {
        Some(guard) => {
            guard.state = Some("holding".to_string());
            Ok(())
        }
        None => bail!("no guard on {task}.{step}"),
    }
}

/// Clear a step guard (F4.5b): record the optional ✋ input `response`, mark it
/// `cleared`, and attach a `guard` evidence entry to each acceptance criterion in
/// the guard's `evidence_for`. Leaves a receipt in `history`.
pub fn clear_guard(plan: &mut Plan, task: &str, step: &str, response: Option<Value>) -> Result<()> {
    let evidence_for = match guard_mut(plan, task, step) {
        Some(guard) => {
            guard.state = Some("cleared".to_string());
            guard.response = response;
            guard.cleared_at = None;
            guard.evidence_for.clone()
        }
        None => bail!("no guard on {task}.{step}"),
    };
    let reference = format!("{task}.{step}");
    for criterion_id in &evidence_for {
        if let Some(criterion) = plan
            .spec
            .acceptance
            .iter_mut()
            .find(|criterion| &criterion.id == criterion_id)
        {
            criterion.evidence.push(Evidence {
                evidence_type: Some("guard".to_string()),
                reference: Some(reference.clone()),
                captured_at: None,
                stale: false,
                extra: Default::default(),
            });
        }
    }
    bump(plan, "user", "guard_cleared", format!("cleared guard {reference}"));
    Ok(())
}

/// Approve a GATE task (F4.5): mark it `done` so execution proceeds. Errors if the
/// task isn't a gate.
pub fn approve_gate(plan: &mut Plan, task: &str) -> Result<()> {
    let is_gate = plan
        .tasks
        .iter()
        .find(|candidate| candidate.id == task)
        .map(|candidate| candidate.gate)
        .unwrap_or(false);
    if !is_gate {
        bail!("task {task} is not a GATE task");
    }
    if let Some(gate) = plan.tasks.iter_mut().find(|candidate| candidate.id == task) {
        gate.status = TaskStatus::Done;
    }
    bump(plan, "user", "gate_approved", format!("approved gate {task}"));
    Ok(())
}

/// The current hold blocking execution: a `holding` step guard, else a GATE task
/// that's in progress (F4.5/F4.5b). Drives the needs-you chrome and the hook.
pub fn current_hold(plan: &Plan) -> Option<Hold> {
    for task in &plan.tasks {
        for step in &task.steps {
            if let Some(guard) = &step.guard {
                if guard.state.as_deref() == Some("holding") {
                    return Some(Hold {
                        task: task.id.clone(),
                        step: Some(step.id.clone()),
                        kind: guard.guard_type.clone().unwrap_or_else(|| "approve".to_string()),
                    });
                }
            }
        }
    }
    plan.tasks
        .iter()
        .find(|task| task.gate && task.status == TaskStatus::InProgress)
        .map(|task| Hold {
            task: task.id.clone(),
            step: None,
            kind: "gate".to_string(),
        })
}

/// The `guards` block of `.plans/policy.json` (Appendix B): commands whose Bash
/// invocation must be guarded, and the default guard kind.
#[derive(Debug, Clone, Deserialize)]
pub struct GuardPolicy {
    #[serde(default)]
    pub require_on: Vec<String>,
    #[serde(default = "default_guard_type")]
    pub default_type: String,
}

fn default_guard_type() -> String {
    "approve".to_string()
}

impl Default for GuardPolicy {
    fn default() -> Self {
        GuardPolicy {
            require_on: Vec::new(),
            default_type: default_guard_type(),
        }
    }
}

impl GuardPolicy {
    /// Load the `guards` block from `<plans_dir>/policy.json`, or defaults if absent.
    pub fn load(plans_dir: &Path) -> GuardPolicy {
        let Ok(text) = std::fs::read_to_string(plans_dir.join("policy.json")) else {
            return GuardPolicy::default();
        };
        serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|value| value.get("guards").cloned())
            .and_then(|guards| serde_json::from_value(guards).ok())
            .unwrap_or_default()
    }
}

/// The `require_on` pattern (substring) matching `command`, if any (F4.5b policy
/// force-guards).
pub fn matched_require_on(policy: &GuardPolicy, command: &str) -> Option<String> {
    policy
        .require_on
        .iter()
        .find(|pattern| command.contains(pattern.as_str()))
        .cloned()
}

/// Whether an in-progress task carries a `cleared` guard whose prompt covers
/// `pattern` — the coverage rule that lets a `require_on` command through (M6b q3).
pub fn command_guard_cleared(plan: &Plan, pattern: &str) -> bool {
    plan.tasks
        .iter()
        .filter(|task| task.status == TaskStatus::InProgress)
        .flat_map(|task| task.steps.iter())
        .filter_map(|step| step.guard.as_ref())
        .any(|guard| {
            guard.state.as_deref() == Some("cleared")
                && guard
                    .prompt
                    .as_deref()
                    .is_some_and(|prompt| prompt.contains(pattern))
        })
}
