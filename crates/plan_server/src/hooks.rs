//! Claude Code hook decision logic, invoked via the `plan_server hook <event>`
//! subcommands. Enforcement blocks; it does not advise (PRD F6.3). The pure
//! decision functions here are unit-tested without Claude Code.

use std::path::Path;

use plan_core::{Plan, Status, TaskStatus, exec, git, store};
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

/// The full PreToolUse decision (F6.3): the no-plan/not-executing edit gate, plus —
/// while executing — a block when a guard or GATE is holding (F4.5/F4.5b), and
/// policy force-guards on matching Bash commands. `command` is the Bash command
/// (if the tool is Bash).
pub fn pretooluse_gate(
    plans_dir: &Path,
    tool_name: &str,
    command: Option<&str>,
) -> Result<(), String> {
    pretooluse_allows_edit(plans_dir, tool_name)?;
    let Some(plan) = active_plan(plans_dir) else {
        return Ok(());
    };
    let is_edit = EDIT_TOOLS.contains(&tool_name);
    let is_bash = tool_name == "Bash";
    // Destructive git is amendment-only whenever a plan governs the session
    // (F10.5c) — enforced regardless of lifecycle, so history is never rewritten.
    if is_bash {
        if let Some(command) = command {
            if git::is_destructive(command) {
                return Err(
                    "destructive git is amendment-only (F10.5c) — propose an amendment instead of rewriting history"
                        .to_string(),
                );
            }
        }
    }
    if plan.status != Status::Executing {
        return Ok(());
    }
    if !is_edit && !is_bash {
        return Ok(());
    }
    if let Some(hold) = exec::current_hold(&plan) {
        return Err(hold_reason(&plan, &hold));
    }
    if is_bash {
        if let Some(command) = command {
            // Commit-time format + trailer check (F10.1/F10.3).
            if is_git_commit(command) {
                let message = commit_message(command);
                if !message.is_empty() {
                    let policy = git::GitPolicy::load(plans_dir);
                    let ticket = git::plan_ticket_key(&plan);
                    git::commit_ok(&policy, &message, ticket)
                        .map_err(|reason| format!("commit blocked — {reason}"))?;
                }
            }
            let policy = exec::GuardPolicy::load(plans_dir);
            if let Some(pattern) = exec::matched_require_on(&policy, command) {
                if !exec::command_guard_cleared(&plan, &pattern) {
                    return Err(format!(
                        "`{pattern}` requires an approved guard — approve it in the Plan panel before running"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Whether a shell command runs `git commit`.
fn is_git_commit(command: &str) -> bool {
    let tokens = shell_tokens(command);
    tokens
        .windows(2)
        .any(|pair| pair[0] == "git" && pair[1] == "commit")
}

/// Reconstruct the commit message from `-m`/`--message` arguments (git joins
/// multiple `-m` values with a blank line — so `-m subject -m trailer` becomes
/// subject + trailer). Empty when none are present (e.g. an editor/`-F` commit,
/// which we can't validate and therefore don't block).
fn commit_message(command: &str) -> String {
    let tokens = shell_tokens(command);
    let mut paragraphs = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if token == "-m" || token == "--message" {
            if let Some(value) = tokens.get(index + 1) {
                paragraphs.push(value.clone());
                index += 2;
                continue;
            }
        } else if let Some(value) = token.strip_prefix("--message=") {
            paragraphs.push(value.to_string());
        }
        index += 1;
    }
    paragraphs.join("\n\n")
}

/// Split a command line into tokens, honoring single/double quotes and stripping
/// the surrounding quote characters. Good enough for hook inspection — not a full
/// shell parser.
fn shell_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for ch in command.chars() {
        match quote {
            Some(active) => {
                if ch == active {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    started = true;
                }
                ch if ch.is_whitespace() => {
                    if started {
                        tokens.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                _ => {
                    current.push(ch);
                    started = true;
                }
            },
        }
    }
    if started {
        tokens.push(current);
    }
    tokens
}

/// A human-facing reason for a guard/gate hold, quoting the guard prompt.
fn hold_reason(plan: &Plan, hold: &exec::Hold) -> String {
    match &hold.step {
        Some(step) => {
            let prompt = plan
                .tasks
                .iter()
                .find(|task| task.id == hold.task)
                .and_then(|task| task.steps.iter().find(|candidate| &candidate.id == step))
                .and_then(|candidate| candidate.guard.as_ref())
                .and_then(|guard| guard.prompt.clone())
                .unwrap_or_else(|| format!("guard on {}.{step}", hold.task));
            format!("held at a guard — {prompt}. Clear it in the Plan panel to continue.")
        }
        None => format!(
            "GATE task {} needs sign-off — approve the gate in the Plan panel.",
            hold.task
        ),
    }
}

/// Stop loop guard (F6.3, lean): while a plan is `executing` with unfinished,
/// unheld tasks, nudge the agent to continue rather than stop. `stop_hook_active`
/// is Claude Code's flag that the stop already resulted from a hook nudge — the
/// runaway guard, so we don't nudge again. Returns the nudge reason, or `None` to
/// allow the stop.
pub fn stop_loop_guard(plans_dir: &Path, stop_hook_active: bool) -> Option<String> {
    if stop_hook_active {
        return None;
    }
    let plan = active_plan(plans_dir)?;
    if plan.status != Status::Executing || exec::current_hold(&plan).is_some() {
        return None;
    }
    let remaining = plan
        .tasks
        .iter()
        .filter(|task| matches!(task.status, TaskStatus::Pending | TaskStatus::InProgress))
        .count();
    (remaining > 0).then(|| {
        format!("{remaining} task(s) remain in the executing plan — continue them, or pause the plan in the Plan panel")
    })
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
            let command = input
                .get("tool_input")
                .and_then(|tool_input| tool_input.get("command"))
                .and_then(Value::as_str);
            match pretooluse_gate(plans_dir, tool, command) {
                Ok(()) => (String::new(), 0),
                Err(reason) => (deny(&reason), 0),
            }
        }
        "Stop" => {
            let active = input
                .get("stop_hook_active")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match stop_loop_guard(plans_dir, active) {
                Some(reason) => (json!({ "decision": "block", "reason": reason }).to_string(), 0),
                None => (String::new(), 0),
            }
        }
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
