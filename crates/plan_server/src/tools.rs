//! Tool logic operating on `.plans/<id>.plan.json` via `plan_core`. Kept as
//! plain functions over a plans directory so they unit-test without rmcp/async.

use std::path::Path;

use anyhow::Result;
use plan_core::rev::HunkSpec;
use plan_core::{
    HistoryEntry, Plan, Status, Task, TaskStatus, TimelineEntry, anchor, comments, exec, lint, rev,
    store,
};

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

/// Set the plan's lifecycle status.
pub fn set_status(plans_dir: &Path, id: &str, status: &str) -> Result<Plan> {
    let parsed = parse_status(status)?;
    let summary = format!("status → {status}");
    mutate(plans_dir, id, "status", summary, move |plan| {
        plan.status = parsed;
        Ok(())
    })
}

/// Append a new task in `pending`.
pub fn add_task(
    plans_dir: &Path,
    id: &str,
    task_id: &str,
    title: &str,
    system: Option<&str>,
) -> Result<Plan> {
    let task_value = serde_json::json!({
        "id": task_id, "title": title, "system": system, "status": "pending",
    });
    let task_id = task_id.to_string();
    let summary = format!("added task {task_id}");
    mutate(plans_dir, id, "add_task", summary, move |plan| {
        if plan.tasks.iter().any(|task| task.id == task_id) {
            anyhow::bail!("task {task_id} already exists");
        }
        plan.tasks.push(serde_json::from_value::<Task>(task_value)?);
        Ok(())
    })
}

/// Update a task's status and/or append a timeline entry.
pub fn task_update(
    plans_dir: &Path,
    id: &str,
    task_id: &str,
    status: Option<&str>,
    detail: Option<&str>,
) -> Result<Plan> {
    let new_status = status.map(parse_task_status).transpose()?;
    let detail = detail.map(String::from);
    let task_id = task_id.to_string();
    let summary = format!("task {task_id} updated");
    mutate(plans_dir, id, "task_update", summary, move |plan| {
        let task = plan
            .tasks
            .iter_mut()
            .find(|task| task.id == task_id)
            .ok_or_else(|| anyhow::anyhow!("no task {task_id}"))?;
        if let Some(status) = new_status {
            task.status = status;
        }
        if let Some(detail) = detail {
            task.timeline.push(TimelineEntry {
                at: None,
                kind: Some("edit".to_string()),
                detail: Some(detail),
                extra: Default::default(),
            });
        }
        Ok(())
    })
}

/// Record an answer to an open question.
pub fn answer_question(
    plans_dir: &Path,
    id: &str,
    question_id: &str,
    answer: &str,
) -> Result<Plan> {
    let question_id = question_id.to_string();
    let answer = answer.to_string();
    let summary = format!("answered {question_id}");
    mutate(plans_dir, id, "answer_question", summary, move |plan| {
        let question = plan
            .spec
            .open_questions
            .iter_mut()
            .find(|question| question.id == question_id)
            .ok_or_else(|| anyhow::anyhow!("no open question {question_id}"))?;
        question.answer = Some(serde_json::Value::String(answer));
        Ok(())
    })
}

/// Update a named spec/design section. M2 supports a small set of paths; the
/// rest of the drafting-edit surface arrives with the review loop (M5).
pub fn update_section(
    plans_dir: &Path,
    id: &str,
    section: &str,
    value: serde_json::Value,
) -> Result<Plan> {
    let section = section.to_string();
    let summary = format!("updated {section}");
    mutate(plans_dir, id, "update_section", summary, move |plan| {
        match section.as_str() {
            "spec.goal" => {
                plan.spec.goal = value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("spec.goal must be a string"))?
                    .to_string();
            }
            "spec.scope.in" => plan.spec.scope.r#in = as_str_vec(&value)?,
            "spec.scope.out" => plan.spec.scope.out = as_str_vec(&value)?,
            "design.risks" => plan.design.risks = as_str_vec(&value)?,
            other => anyhow::bail!("unsupported section: {other}"),
        }
        Ok(())
    })
}

/// Load a plan, apply `mutation`, bump `rev`, stamp `history[]` (the sync-receipt
/// trail, PRD §1), and save atomically.
fn mutate(
    plans_dir: &Path,
    id: &str,
    kind: &str,
    summary: String,
    mutation: impl FnOnce(&mut Plan) -> Result<()>,
) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    mutation(&mut plan)?;
    plan.rev += 1;
    plan.history.push(HistoryEntry {
        rev: Some(plan.rev),
        by: Some("agent".to_string()),
        kind: Some(kind.to_string()),
        summary: Some(summary),
        at: None,
        extra: Default::default(),
    });
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

fn parse_status(status: &str) -> Result<Status> {
    serde_json::from_value(serde_json::Value::String(status.to_string()))
        .map_err(|_| anyhow::anyhow!("unknown plan status: {status}"))
}

fn parse_task_status(status: &str) -> Result<TaskStatus> {
    serde_json::from_value(serde_json::Value::String(status.to_string()))
        .map_err(|_| anyhow::anyhow!("unknown task status: {status}"))
}

fn as_str_vec(value: &serde_json::Value) -> Result<Vec<String>> {
    value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("expected a JSON array of strings"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(String::from)
                .ok_or_else(|| anyhow::anyhow!("array must contain only strings"))
        })
        .collect()
}

/// Open/sent comments as a summary array (with re-anchor status) for the agent.
pub fn list_comments(plans_dir: &Path, id: &str) -> Result<serde_json::Value> {
    let plan = store::load(plans_dir, id)?;
    let items: Vec<serde_json::Value> = plan
        .comments
        .iter()
        .filter(|comment| matches!(comment.state.as_deref(), Some("open") | Some("sent")))
        .map(|comment| {
            let reanchor = comment
                .anchor
                .as_ref()
                .map(|a| format!("{:?}", anchor::reanchor(a, &plan)));
            serde_json::json!({
                "id": comment.id,
                "kind": comment.kind,
                "severity": comment.severity,
                "state": comment.state,
                "block": comment.anchor.as_ref().and_then(|a| a.block.clone()),
                "reanchor": reanchor,
                "text": comment.thread.first().and_then(|message| message.text.clone()),
            })
        })
        .collect();
    Ok(serde_json::Value::Array(items))
}

/// Append an agent reply to a comment's thread.
pub fn reply_comment(
    plans_dir: &Path,
    id: &str,
    comment_id: &str,
    text: &str,
    action: Option<&str>,
) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    if !comments::reply(&mut plan, comment_id, "agent", text, action) {
        anyhow::bail!("no comment {comment_id}");
    }
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Mark a comment addressed.
pub fn mark_addressed(plans_dir: &Path, id: &str, comment_id: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    if !comments::set_comment_state(&mut plan, comment_id, "addressed") {
        anyhow::bail!("no comment {comment_id}");
    }
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Apply a suggestion's replacement to its anchored block.
pub fn apply_suggestion(plans_dir: &Path, id: &str, comment_id: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    if !comments::apply_suggestion(&mut plan, comment_id) {
        anyhow::bail!("could not apply suggestion {comment_id}");
    }
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Stage an agent-proposed revision as a pending diff (F9.3). Apply/Reject is the
/// user's action in the UI; this tool only stages and never mutates blocks or
/// bumps `rev`. The same primitive backs execution-time amendments (F4.7).
pub fn propose_revision(plans_dir: &Path, id: &str, hunks: Vec<HunkSpec>) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    rev::stage_revision(&mut plan, hunks);
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Launch the plan (F4.1): transition `approved → executing` and take the executor
/// lease for `thread` (F11.3). Git branch/worktree setup (F10.2) is M7; this only
/// moves plan state. The per-task loop then uses `task_update`.
pub fn launch(plans_dir: &Path, id: &str, thread: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::launch(&mut plan, thread)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Pause execution (F5.1).
pub fn pause(plans_dir: &Path, id: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::pause(&mut plan)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Resume a paused plan (F5.1).
pub fn resume(plans_dir: &Path, id: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::resume(&mut plan)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Stop / kill execution (F5.1/F5.6): halt + release the lease.
pub fn stop(plans_dir: &Path, id: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::stop(&mut plan)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Propose an amendment (F4.7): a staged plan change during execution, recorded
/// against `task` for the failure ladder.
pub fn propose_amendment(
    plans_dir: &Path,
    id: &str,
    task: &str,
    hunks: Vec<HunkSpec>,
) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::propose_amendment(&mut plan, task, hunks)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Recover an interrupted task (F11.1): choice ∈ resume|redo|manual.
pub fn recover_task(plans_dir: &Path, id: &str, task: &str, choice: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::recover_task(&mut plan, task, choice)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Mark a step guard as holding (F4.5b) — the agent calls this when it reaches a
/// guarded step, so the PreToolUse hook blocks until the user clears it.
pub fn hold_guard(plans_dir: &Path, id: &str, task: &str, step: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::hold_guard(&mut plan, task, step)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Clear a step guard (F4.5b): record the optional ✋ input response + evidence.
pub fn clear_guard(
    plans_dir: &Path,
    id: &str,
    task: &str,
    step: &str,
    response: Option<serde_json::Value>,
) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::clear_guard(&mut plan, task, step, response)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Approve a GATE task (F4.5) so execution proceeds.
pub fn approve_gate(plans_dir: &Path, id: &str, task: &str) -> Result<Plan> {
    let mut plan = store::load(plans_dir, id)?;
    exec::approve_gate(&mut plan, task)?;
    store::save(plans_dir, &plan)?;
    Ok(plan)
}

/// Run policy lint over the plan (F9.1): reconcile findings into `plan-lint`
/// comments and return them so the agent can auto-fix and re-run. The worktree
/// root (parent of `.plans/`) enables `files-must-exist`. Auto-fix is the agent's
/// job; this tool only flags.
pub fn lint(plans_dir: &Path, id: &str) -> Result<serde_json::Value> {
    let mut plan = store::load(plans_dir, id)?;
    let policy = lint::Policy::load(plans_dir);
    let repo_root = plans_dir.parent();
    let findings = lint::lint(&plan, &policy, repo_root);
    if lint::reconcile(&mut plan, &findings) {
        store::save(plans_dir, &plan)?;
    }
    let reported: Vec<serde_json::Value> = findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "rule": finding.rule_id,
                "severity": finding.severity.label(),
                "block": finding.block,
                "message": finding.message,
            })
        })
        .collect();
    Ok(serde_json::json!({ "findings": reported }))
}
