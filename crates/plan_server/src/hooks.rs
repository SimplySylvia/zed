//! Claude Code hook decision logic, invoked via the `plan_server hook <event>`
//! subcommands. Enforcement blocks; it does not advise (PRD F6.3). The pure
//! decision functions here are unit-tested without Claude Code.

use std::path::Path;

use plan_core::{Plan, Status, TaskStatus, store};
use serde_json::{Value, json};

/// File-writing tools gated by the executing-plan rule.
const EDIT_TOOLS: &[&str] = &["Edit", "Write", "MultiEdit", "NotebookEdit"];

/// The plan a session should act on: prefer an `executing` plan, else the sole
/// plan in `.plans/`. Returns `None` if there are zero, or several non-executing,
/// plans (ambiguous — nothing to enforce against).
pub fn active_plan(plans_dir: &Path) -> Option<Plan> {
    let mut plans: Vec<Plan> = match std::fs::read_dir(plans_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let id = name.strip_suffix(".plan.json")?;
                store::load(plans_dir, id).ok()
            })
            .collect(),
        Err(_) => return None,
    };
    if let Some(executing) = plans.iter().find(|plan| plan.status == Status::Executing) {
        return Some(executing.clone());
    }
    if plans.len() == 1 {
        return plans.pop();
    }
    None
}

/// A one-line resume briefing for SessionStart / prompt re-injection.
pub fn resume_briefing(plans_dir: &Path) -> Option<String> {
    let plan = active_plan(plans_dir)?;
    let done = plan
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Done)
        .count();
    Some(format!(
        "Active plan {} — status {:?}, rev {}. Tasks {}/{} done. \
         Re-read with plan_get before each task; reply 'resume' to continue.",
        plan.id,
        plan.status,
        plan.rev,
        done,
        plan.tasks.len()
    ))
}

/// PreToolUse gate: file-editing tools are denied unless a plan is executing.
/// Non-edit tools are always allowed.
pub fn pretooluse_allows_edit(plans_dir: &Path, tool_name: &str) -> Result<(), String> {
    if !EDIT_TOOLS.contains(&tool_name) {
        return Ok(());
    }
    match active_plan(plans_dir) {
        Some(plan) if plan.status == Status::Executing => Ok(()),
        Some(plan) => Err(format!(
            "plan {} is {:?}, not executing — draft and launch it before editing code",
            plan.id, plan.status
        )),
        None => Err(
            "no executing plan in .plans/ — draft and launch a plan before editing code"
                .to_string(),
        ),
    }
}

/// Dispatch a hook event to its decision: returns `(stdout JSON, exit code)`.
pub fn run(event: &str, input: &Value, plans_dir: &Path) -> (String, i32) {
    match event {
        "SessionStart" | "UserPromptSubmit" | "PreCompact" => match resume_briefing(plans_dir) {
            Some(context) => (inject(event, &context), 0),
            None => (String::new(), 0),
        },
        "PreToolUse" => {
            let tool = input
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match pretooluse_allows_edit(plans_dir, tool) {
                Ok(()) => (String::new(), 0),
                Err(reason) => (deny(&reason), 0),
            }
        }
        // Stop: M2 placeholder — allow the agent to stop. A real loop guard
        // (nudge-to-continue with runaway protection) arrives with execution (M6).
        "Stop" => (String::new(), 0),
        _ => (String::new(), 0),
    }
}

fn inject(event: &str, context: &str) -> String {
    json!({ "hookSpecificOutput": { "hookEventName": event, "additionalContext": context } })
        .to_string()
}

fn deny(reason: &str) -> String {
    json!({ "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }})
    .to_string()
}
