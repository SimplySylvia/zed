//! Git policy (Appendix B `policy.git`) + pure branch/commit helpers (F10.1 git
//! lint, F10.2 launch guard, F10.3 task-scoped commits + trailer, F10.5c
//! destructive-op gate).
//!
//! **Facts-in, decision-out.** Nothing here shells `git`: helpers take supplied
//! facts (a commit message, a dirty flag, a behind-count) and return a decision,
//! so they're unit-testable without a repo. Git command execution lives in the
//! server (`plan_server::git`); the UI reads live state via `project::git_store`.
//!
//! Defaults are **generic** (a permissive ticketed commit format, `main` base) so
//! an absent `.plans/policy.json` behaves sanely; a real git block tightens them
//! (see docs/milestones/M7a.md for the recorded default + trailer-token decisions).

use std::path::Path;

use serde_json::Value;

use crate::Plan;
use crate::schema::Task;

/// The `git` block of `.plans/policy.json` (Appendix B), defaults applied for any
/// key the file omits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPolicy {
    pub branch_pattern: String,
    pub branch_pattern_no_ticket: String,
    pub base_feature: String,
    pub base_hotfix: String,
    pub commit_format: String,
    pub commit_format_no_ticket: String,
    pub imperative_mood: bool,
    pub one_commit_per_task: bool,
    pub require_tests_green: bool,
    pub trailer: String,
    pub destructive_ops: String,
}

impl Default for GitPolicy {
    fn default() -> Self {
        GitPolicy {
            branch_pattern: "{ticket}-{slug}".to_string(),
            branch_pattern_no_ticket: "{slug}".to_string(),
            base_feature: "main".to_string(),
            base_hotfix: "main".to_string(),
            // Generic ticketed subject: `[ABC-123]: 8..72 chars`.
            commit_format: r"^\[[A-Z]+-\d+\]: .{8,72}$".to_string(),
            commit_format_no_ticket: r"^.{8,72}$".to_string(),
            imperative_mood: true,
            one_commit_per_task: true,
            require_tests_green: true,
            trailer: "Plan: {ticket} rev{rev} task-{task}".to_string(),
            destructive_ops: "amendment_only".to_string(),
        }
    }
}

impl GitPolicy {
    /// Load the `git` block from `<plans_dir>/policy.json`, overlaying defaults.
    pub fn load(plans_dir: &Path) -> GitPolicy {
        let Ok(text) = std::fs::read_to_string(plans_dir.join("policy.json")) else {
            return GitPolicy::default();
        };
        serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|value| value.get("git").cloned())
            .map(|git| GitPolicy::from_json(&git))
            .unwrap_or_default()
    }

    /// Overlay a parsed `git` object onto the defaults.
    pub fn from_json(git: &Value) -> GitPolicy {
        let mut policy = GitPolicy::default();
        let string = |value: &Value, target: &mut String| {
            if let Some(text) = value.as_str() {
                *target = text.to_string();
            }
        };
        let boolean = |value: &Value, target: &mut bool| {
            if let Some(flag) = value.as_bool() {
                *target = flag;
            }
        };
        if let Some(branch) = git.get("branch") {
            if let Some(value) = branch.get("pattern") {
                string(value, &mut policy.branch_pattern);
            }
            if let Some(value) = branch.get("pattern_no_ticket") {
                string(value, &mut policy.branch_pattern_no_ticket);
            }
            if let Some(base) = branch.get("base") {
                if let Some(value) = base.get("feature") {
                    string(value, &mut policy.base_feature);
                }
                if let Some(value) = base.get("hotfix") {
                    string(value, &mut policy.base_hotfix);
                }
            }
        }
        if let Some(commit) = git.get("commit") {
            if let Some(value) = commit.get("format") {
                string(value, &mut policy.commit_format);
            }
            if let Some(value) = commit.get("format_no_ticket") {
                string(value, &mut policy.commit_format_no_ticket);
            }
            if let Some(value) = commit.get("imperative_mood") {
                boolean(value, &mut policy.imperative_mood);
            }
            if let Some(value) = commit.get("one_commit_per_task") {
                boolean(value, &mut policy.one_commit_per_task);
            }
            if let Some(value) = commit.get("require_tests_green") {
                boolean(value, &mut policy.require_tests_green);
            }
            if let Some(value) = commit.get("trailer") {
                string(value, &mut policy.trailer);
            }
        }
        if let Some(value) = git.get("destructive_ops") {
            string(value, &mut policy.destructive_ops);
        }
        policy
    }
}

/// The plan's ticket key (the first ticket), or `None` for a ticketless plan.
pub fn plan_ticket_key(plan: &Plan) -> Option<&str> {
    plan.tickets.first().map(|ticket| ticket.key.as_str())
}

/// A branch/PR-safe slug from arbitrary text: lowercase, non-alphanumeric runs
/// collapse to a single `-`, with no leading/trailing `-`.
pub fn slug(text: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.extend(ch.to_lowercase());
        } else {
            pending_dash = true;
        }
    }
    slug
}

/// The branch name for the plan (F10.2): `{ticket}-{slug}` (ticket lowercased),
/// or the ticketless pattern (`{slug}` by default).
pub fn branch_name(plan: &Plan, policy: &GitPolicy) -> String {
    let title_slug = slug(&plan.title);
    match plan_ticket_key(plan) {
        Some(ticket) => policy
            .branch_pattern
            .replace("{ticket}", &ticket.to_lowercase())
            .replace("{slug}", &title_slug),
        None => policy.branch_pattern_no_ticket.replace("{slug}", &title_slug),
    }
}

/// The base branch to branch from (F10.2). MVP: always the feature base; hotfix
/// detection is deferred (there is no plan-`type` signal yet — M7a q4).
pub fn base_for(_plan: &Plan, policy: &GitPolicy) -> String {
    policy.base_feature.clone()
}

/// The commit trailer for a task (F10.3): `Plan: {ticket} rev{rev} task-{task}`.
/// The task's own ticket wins over the plan-level ticket (F2.4c); a ticketless
/// commit drops the `{ticket}` token cleanly.
pub fn trailer_for(plan: &Plan, task: &Task) -> String {
    trailer_with(&GitPolicy::default(), plan, task)
}

/// [`trailer_for`] against an explicit policy template (honors a custom
/// `policy.trailer`).
pub fn trailer_with(policy: &GitPolicy, plan: &Plan, task: &Task) -> String {
    let ticket = task.ticket.as_deref().or_else(|| plan_ticket_key(plan));
    let trailer = policy
        .trailer
        .replace("{rev}", &plan.rev.to_string())
        .replace("{task}", &task.id);
    match ticket {
        Some(key) => trailer.replace("{ticket}", key),
        None => trailer.replace("{ticket} ", "").replace("{ticket}", ""),
    }
}

/// Whether a commit *subject* line satisfies the configured format (F10.1). Used
/// by draft-time lint (subject only) and by [`commit_ok`] (subject + trailer).
pub fn commit_subject_ok(policy: &GitPolicy, subject: &str, ticketed: bool) -> bool {
    let pattern = if ticketed {
        &policy.commit_format
    } else {
        &policy.commit_format_no_ticket
    };
    match regex::Regex::new(pattern) {
        Ok(regex) => regex.is_match(subject),
        // A malformed policy regex must not silently pass everything.
        Err(_) => false,
    }
}

/// Whether a full commit message carries a `Plan:` trailer line (F10.3), and — when
/// ticketed — names the ticket. Shape-only: the exact rev/task aren't known here.
pub fn trailer_present(message: &str, ticket: Option<&str>) -> bool {
    message.lines().any(|line| {
        let line = line.trim();
        line.starts_with("Plan:")
            && line.contains("task-")
            && ticket.is_none_or(|key| line.contains(key))
    })
}

/// Validate a full commit message (F10.1/F10.3) for the commit-time hook: the
/// first line matches the format and a `Plan:` trailer is present. Returns a
/// human-facing reason on failure.
pub fn commit_ok(policy: &GitPolicy, message: &str, ticket: Option<&str>) -> Result<(), String> {
    let subject = message.lines().next().unwrap_or_default();
    if !commit_subject_ok(policy, subject, ticket.is_some()) {
        return Err(format!(
            "commit subject `{subject}` does not match the required format — {}",
            if ticket.is_some() {
                policy.commit_format.as_str()
            } else {
                policy.commit_format_no_ticket.as_str()
            }
        ));
    }
    if !trailer_present(message, ticket) {
        return Err("commit is missing the `Plan: {ticket} rev{rev} task-{task}` trailer".to_string());
    }
    Ok(())
}

/// The launch guard (F10.2): refuse launch on a dirty tree or a stale base.
/// Pure over supplied facts; the caller sources them (server shells `git`, UI
/// reads `git_store`).
pub fn launch_block(_policy: &GitPolicy, dirty: bool, behind: u32) -> Option<String> {
    if dirty {
        return Some(
            "working tree has uncommitted changes — commit or stash them before launching".to_string(),
        );
    }
    if behind > 0 {
        return Some(format!(
            "branch is {behind} commit(s) behind its base — update it before launching"
        ));
    }
    None
}

/// Whether a shell command is a history-rewriting / destructive git op that
/// `destructive_ops: amendment_only` forbids (F10.5c). `git revert` is a *new*
/// commit and is intentionally not destructive.
pub fn is_destructive(command: &str) -> bool {
    // Collapse whitespace so `reset   --hard` matches too.
    let normalized = command.split_whitespace().collect::<Vec<_>>().join(" ");
    const PATTERNS: &[&str] = &[
        "reset --hard",
        "push --force",
        "push -f ",
        "push --force-with-lease",
        "branch -D",
        "branch --delete --force",
        "clean -f",
        "checkout --",
    ];
    let padded = format!("{normalized} ");
    PATTERNS.iter().any(|pattern| padded.contains(pattern))
}
