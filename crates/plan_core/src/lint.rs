//! Policy-driven plan lint (F9.1): rule checkers over a `Plan` + a `Policy` read
//! from `.plans/policy.json` (Appendix B) atop built-in defaults. This is the
//! single source of lint truth — `plan_server` reuses it, and the UI runs it
//! directly. Findings become `plan-lint`-authored comments (see
//! [`crate::comments`]); blocker-severity findings gate Approve.
//!
//! Structural referential-integrity checks live separately in
//! [`crate::schema::Plan::validate`]; this module is the *policy* layer.

use std::collections::BTreeMap;
use std::path::Path;

use crate::Plan;
use crate::schema::{Anchor, Comment, HistoryEntry, ThreadEntry};

/// The author stamped on lint-generated comments (F9.1).
const LINT_AUTHOR: &str = "plan-lint";

/// A rule's configured severity. `Off` disables the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Blocker,
    Warn,
    Off,
}

impl Severity {
    fn parse(value: &str) -> Option<Severity> {
        match value {
            "blocker" => Some(Severity::Blocker),
            "warn" => Some(Severity::Warn),
            "off" => Some(Severity::Off),
            _ => None,
        }
    }

    /// The comment-system severity string for a finding (`blocker`/`concern`);
    /// `Off` never produces a finding so it maps to `concern` defensively.
    pub fn comment_severity(self) -> &'static str {
        match self {
            Severity::Blocker => "blocker",
            _ => "concern",
        }
    }

    /// The policy severity label, for reporting a finding as-configured.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Blocker => "blocker",
            Severity::Warn => "warn",
            Severity::Off => "off",
        }
    }
}

/// The `lint` block of `.plans/policy.json` (Appendix B), with defaults applied
/// for any rule the file omits. The file only *tightens* — an absent file uses
/// all defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub every_task_has_tests: Severity,
    pub criteria_link_tasks: Severity,
    pub files_must_exist: Severity,
    pub prod_requires_gate: Severity,
    pub max_files_per_task: (Severity, usize),
    pub ticket_coverage: Severity,
}

impl Default for Policy {
    fn default() -> Self {
        // Appendix B defaults.
        Policy {
            every_task_has_tests: Severity::Blocker,
            criteria_link_tasks: Severity::Warn,
            files_must_exist: Severity::Warn,
            prod_requires_gate: Severity::Blocker,
            max_files_per_task: (Severity::Warn, 8),
            ticket_coverage: Severity::Blocker,
        }
    }
}

impl Policy {
    /// Load `<plans_dir>/policy.json` if present, overriding defaults with any
    /// rule it specifies. A missing or unreadable file yields the defaults.
    pub fn load(plans_dir: &Path) -> Policy {
        let text = match std::fs::read_to_string(plans_dir.join("policy.json")) {
            Ok(text) => text,
            Err(_) => return Policy::default(),
        };
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => Policy::from_json(&value),
            Err(_) => Policy::default(),
        }
    }

    /// Overlay the `lint` object of a parsed policy document onto the defaults.
    pub fn from_json(value: &serde_json::Value) -> Policy {
        let mut policy = Policy::default();
        let Some(lint) = value.get("lint") else {
            return policy;
        };
        let severity = |key: &str| {
            lint.get(key)
                .and_then(|value| value.as_str())
                .and_then(Severity::parse)
        };
        if let Some(severity) = severity("every-task-has-tests") {
            policy.every_task_has_tests = severity;
        }
        if let Some(severity) = severity("criteria-link-tasks") {
            policy.criteria_link_tasks = severity;
        }
        if let Some(severity) = severity("files-must-exist") {
            policy.files_must_exist = severity;
        }
        if let Some(severity) = severity("prod-requires-gate") {
            policy.prod_requires_gate = severity;
        }
        if let Some(severity) = severity("ticket-coverage") {
            policy.ticket_coverage = severity;
        }
        if let Some(rule) = lint.get("max-files-per-task") {
            let (mut sev, mut limit) = policy.max_files_per_task;
            if let Some(parsed) = rule.get("severity").and_then(|v| v.as_str()).and_then(Severity::parse)
            {
                sev = parsed;
            }
            if let Some(parsed) = rule.get("limit").and_then(|v| v.as_u64()) {
                limit = parsed as usize;
            }
            policy.max_files_per_task = (sev, limit);
        }
        policy
    }
}

/// A single lint finding, anchored to the offending block when applicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub message: String,
    pub block: Option<String>,
}

/// Run every enabled rule over `plan`. `repo_root` (the worktree root that
/// `.plans/` lives under) enables `files-must-exist`; pass `None` to skip it
/// (e.g. fixture tests).
pub fn lint(plan: &Plan, policy: &Policy, repo_root: Option<&Path>) -> Vec<Finding> {
    let mut findings = Vec::new();

    let (files_severity, limit) = policy.max_files_per_task;
    if files_severity != Severity::Off {
        for task in &plan.tasks {
            if task.files.len() > limit {
                findings.push(Finding {
                    rule_id: "max-files-per-task",
                    severity: files_severity,
                    message: format!(
                        "task {} touches {} files (limit {limit})",
                        task.id,
                        task.files.len()
                    ),
                    block: Some(task.id.clone()),
                });
            }
        }
    }

    if policy.criteria_link_tasks != Severity::Off {
        for acceptance in &plan.spec.acceptance {
            if acceptance.tasks.is_empty() {
                findings.push(Finding {
                    rule_id: "criteria-link-tasks",
                    severity: policy.criteria_link_tasks,
                    message: format!("criterion {} links no tasks", acceptance.id),
                    block: Some(acceptance.id.clone()),
                });
            }
        }
    }

    if policy.every_task_has_tests != Severity::Off {
        for task in &plan.tasks {
            let is_testing = task.system.as_deref() == Some("testing");
            if !task.gate && !is_testing && task.acceptance.is_empty() {
                findings.push(Finding {
                    rule_id: "every-task-has-tests",
                    severity: policy.every_task_has_tests,
                    message: format!(
                        "task {} links no acceptance criteria (no test coverage)",
                        task.id
                    ),
                    block: Some(task.id.clone()),
                });
            }
        }
    }

    if policy.files_must_exist != Severity::Off {
        if let Some(root) = repo_root {
            for task in &plan.tasks {
                for file in &task.files {
                    if !root.join(&file.path).exists() {
                        findings.push(Finding {
                            rule_id: "files-must-exist",
                            severity: policy.files_must_exist,
                            message: format!(
                                "task {} references missing file {}",
                                task.id, file.path
                            ),
                            block: Some(task.id.clone()),
                        });
                    }
                }
            }
        }
    }

    // Deferred (recorded no-ops — the engine hosts them cheaply when inputs land):
    // - `prod-requires-gate`: the schema carries no "prod" signal on a task yet
    //   (M5c open q3); revisit with guards/git in M6/M7.
    // - `ticket-coverage`: needs `tickets[]`/`ticket_ac` mapping (M8).
    // - git-format rules (F10.1): enforced at the commit-time hook (M7).

    findings
}

/// Deterministic id for the comment a finding produces, so re-runs update in
/// place instead of duplicating.
fn finding_comment_id(finding: &Finding) -> String {
    format!(
        "lint-{}-{}",
        finding.rule_id,
        finding.block.as_deref().unwrap_or("_")
    )
}

fn lint_message(text: &str) -> ThreadEntry {
    ThreadEntry {
        author: Some(LINT_AUTHOR.to_string()),
        text: Some(text.to_string()),
        action: None,
        rev: None,
        cites: vec![],
        pushback: None,
        extra: Default::default(),
    }
}

/// Idempotently sync `plan-lint`-authored comments to `findings` (F9.1): add a
/// comment per new finding, refresh one whose message/severity drifted, and clear
/// plan-lint comments whose finding no longer fires (so fixing an issue removes
/// its flag). `user`/`agent` comments are never touched. Rev-bumps + stamps
/// history only when something changed; returns whether it did.
pub fn reconcile(plan: &mut Plan, findings: &[Finding]) -> bool {
    let desired: BTreeMap<String, &Finding> = findings
        .iter()
        .map(|finding| (finding_comment_id(finding), finding))
        .collect();
    let mut changed = false;

    let before = plan.comments.len();
    plan.comments.retain(|comment| {
        comment.author.as_deref() != Some(LINT_AUTHOR) || desired.contains_key(&comment.id)
    });
    if plan.comments.len() != before {
        changed = true;
    }

    for (id, finding) in &desired {
        let severity = finding.severity.comment_severity();
        let existing = plan
            .comments
            .iter_mut()
            .find(|comment| &comment.id == id && comment.author.as_deref() == Some(LINT_AUTHOR));
        match existing {
            Some(comment) => {
                let current_text = comment.thread.first().and_then(|message| message.text.clone());
                if comment.severity.as_deref() != Some(severity)
                    || current_text.as_deref() != Some(finding.message.as_str())
                {
                    comment.severity = Some(severity.to_string());
                    comment.thread = vec![lint_message(&finding.message)];
                    changed = true;
                }
            }
            None => {
                plan.comments.push(Comment {
                    id: id.clone(),
                    kind: Some("flag".to_string()),
                    author: Some(LINT_AUTHOR.to_string()),
                    severity: Some(severity.to_string()),
                    anchor: finding.block.as_ref().map(|block| Anchor {
                        lens: Some("tasks".to_string()),
                        block: Some(block.clone()),
                        range: None,
                        quote: None,
                        code_refs: vec![],
                        extra: Default::default(),
                    }),
                    outdated: false,
                    suggestion: None,
                    alternatives: None,
                    thread: vec![lint_message(&finding.message)],
                    state: Some("open".to_string()),
                    caused_changes: vec![],
                    extra: Default::default(),
                });
                changed = true;
            }
        }
    }

    if changed {
        plan.rev += 1;
        plan.history.push(HistoryEntry {
            rev: Some(plan.rev),
            by: Some(LINT_AUTHOR.to_string()),
            kind: Some("lint".to_string()),
            summary: Some(format!("{} lint finding(s)", desired.len())),
            at: None,
            extra: Default::default(),
        });
    }
    changed
}
