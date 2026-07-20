//! Policy-driven lint (F9.1): rule checkers over a plan + policy, with the clean
//! LED-212 fixture as the no-findings baseline and mutations tripping each rule.

use std::fs;

use plan_core::lint::{self, Policy, Severity};
use plan_core::{Plan, store};

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

fn plan(value: serde_json::Value) -> Plan {
    serde_json::from_value(value).unwrap()
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_core_lint_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn ids(findings: &[lint::Finding], rule: &str) -> Vec<String> {
    findings
        .iter()
        .filter(|finding| finding.rule_id == rule)
        .map(|finding| finding.block.clone().unwrap_or_default())
        .collect()
}

#[test]
fn clean_fixture_has_no_findings() {
    let plan: Plan = serde_json::from_str(FIXTURE).unwrap();
    // repo_root None skips files-must-exist; the fixture is otherwise clean.
    let findings = lint::lint(&plan, &Policy::default(), None);
    assert!(findings.is_empty(), "unexpected findings: {findings:?}");
}

#[test]
fn max_files_per_task_fires_over_limit() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "acceptance": ["a1"], "files": [
            { "path": "a.rs" }, { "path": "b.rs" }
        ] }]
    }));
    let policy = Policy {
        max_files_per_task: (Severity::Warn, 1),
        ..Policy::default()
    };
    assert_eq!(ids(&lint::lint(&plan, &policy, None), "max-files-per-task"), ["t1"]);
}

#[test]
fn criteria_link_tasks_fires_on_unlinked_criterion() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a",
        "spec": { "goal": "g", "acceptance": [
            { "id": "a1", "tasks": ["t1"] },
            { "id": "a2", "tasks": [] }
        ] },
        "tasks": [{ "id": "t1", "acceptance": ["a1"] }]
    }));
    assert_eq!(
        ids(&lint::lint(&plan, &Policy::default(), None), "criteria-link-tasks"),
        ["a2"]
    );
}

#[test]
fn every_task_has_tests_fires_on_unlinked_task() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "system": "backend", "acceptance": [] }]
    }));
    let findings = lint::lint(&plan, &Policy::default(), None);
    let rule: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "every-task-has-tests")
        .collect();
    assert_eq!(rule.len(), 1);
    assert_eq!(rule[0].severity, Severity::Blocker);
}

#[test]
fn testing_task_and_gate_are_exempt_from_every_task_has_tests() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [
            { "id": "t1", "system": "testing", "acceptance": [] },
            { "id": "t2", "gate": true, "acceptance": [] }
        ]
    }));
    assert!(ids(&lint::lint(&plan, &Policy::default(), None), "every-task-has-tests").is_empty());
}

#[test]
fn off_severity_suppresses_a_rule() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "system": "backend", "acceptance": [],
                    "commit_message": "implement the backend task" }]
    }));
    let policy = Policy {
        every_task_has_tests: Severity::Off,
        ..Policy::default()
    };
    assert!(lint::lint(&plan, &policy, None).is_empty());
}

#[test]
fn files_must_exist_fires_only_for_missing_files() {
    let dir = scratch("files");
    fs::write(dir.join("present.rs"), "// here").unwrap();
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "acceptance": ["a1"], "files": [
            { "path": "present.rs" }, { "path": "missing.rs" }
        ] }]
    }));
    let findings = lint::lint(&plan, &Policy::default(), Some(&dir));
    let messages: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "files-must-exist")
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("missing.rs"));
}

#[test]
fn absent_policy_falls_back_to_defaults() {
    let dir = scratch("nopolicy");
    let policy = Policy::load(&dir);
    assert_eq!(policy.every_task_has_tests, Severity::Blocker);
    assert_eq!(policy.max_files_per_task, (Severity::Warn, 8));
}

#[test]
fn policy_json_overrides_defaults() {
    let dir = scratch("policy");
    let policy_json = serde_json::json!({
        "lint": {
            "every-task-has-tests": "warn",
            "max-files-per-task": { "severity": "blocker", "limit": 3 }
        }
    });
    fs::write(
        dir.join("policy.json"),
        serde_json::to_string_pretty(&policy_json).unwrap(),
    )
    .unwrap();
    let policy = Policy::load(&dir);
    assert_eq!(policy.every_task_has_tests, Severity::Warn);
    assert_eq!(policy.max_files_per_task, (Severity::Blocker, 3));
    // Unspecified rules keep their defaults.
    assert_eq!(policy.criteria_link_tasks, Severity::Warn);
}

#[test]
fn git_commit_format_fires_on_malformed_subject() {
    // A ticketed plan whose task subject lacks the `[ABC-123]:` prefix.
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1" }],
        "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "ticket": "LED-1", "acceptance": ["a1"], "commit_message": "wip" }]
    }));
    assert_eq!(ids(&lint::lint(&plan, &Policy::default(), None), "git-commit-format"), ["t1"]);
}

#[test]
fn git_commit_format_fires_on_missing_commit_message() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1" }],
        "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "ticket": "LED-1", "acceptance": ["a1"] }]
    }));
    assert_eq!(ids(&lint::lint(&plan, &Policy::default(), None), "git-commit-format"), ["t1"]);
}

#[test]
fn git_commit_format_exempts_gate_tasks() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1" }],
        "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "gate": true }]
    }));
    assert!(ids(&lint::lint(&plan, &Policy::default(), None), "git-commit-format").is_empty());
}

#[test]
fn git_commit_format_off_suppresses_the_rule() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1" }],
        "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "ticket": "LED-1", "acceptance": ["a1"], "commit_message": "wip" }]
    }));
    let policy = Policy {
        git_commit_format: Severity::Off,
        ..Policy::default()
    };
    assert!(lint::lint(&plan, &policy, None).is_empty());
}

#[test]
fn ticket_coverage_fires_on_uncovered_ticket_ac() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }],
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "ticket_ac": "LED-1#1" }] }
    }));
    let findings = lint::lint(&plan, &Policy::default(), None);
    let coverage: Vec<_> = findings
        .iter()
        .filter(|finding| finding.rule_id == "ticket-coverage")
        .collect();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0].severity, Severity::Blocker);
    assert_eq!(coverage[0].block.as_deref(), Some("LED-1#2"));
}

fn plan_lint_comments(plan: &Plan) -> Vec<&plan_core::Comment> {
    plan.comments
        .iter()
        .filter(|comment| comment.author.as_deref() == Some("plan-lint"))
        .collect()
}

fn unlinked_task_plan() -> Plan {
    // Trips only every-task-has-tests (acceptance empty); a valid commit_message
    // keeps git-commit-format quiet so the reconcile tests see exactly one finding.
    plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "system": "backend", "acceptance": [],
                    "commit_message": "implement the backend task" }]
    }))
}

#[test]
fn reconcile_adds_a_plan_lint_comment_per_finding() {
    let mut plan = unlinked_task_plan();
    let findings = lint::lint(&plan, &Policy::default(), None);
    let before = plan.rev;
    assert!(lint::reconcile(&mut plan, &findings));

    let lint_comments = plan_lint_comments(&plan);
    assert_eq!(lint_comments.len(), 1);
    let comment = lint_comments[0];
    assert_eq!(comment.severity.as_deref(), Some("blocker"));
    assert_eq!(comment.state.as_deref(), Some("open"));
    assert_eq!(
        comment.anchor.as_ref().and_then(|a| a.block.as_deref()),
        Some("t1")
    );
    assert!(plan.rev > before);
}

#[test]
fn reconcile_clears_findings_that_no_longer_fire() {
    let mut plan = unlinked_task_plan();
    let findings = lint::lint(&plan, &Policy::default(), None);
    lint::reconcile(&mut plan, &findings);
    assert_eq!(plan_lint_comments(&plan).len(), 1);

    // The issue is fixed → no findings → the plan-lint comment is cleared.
    assert!(lint::reconcile(&mut plan, &[]));
    assert!(plan_lint_comments(&plan).is_empty());
}

#[test]
fn reconcile_never_touches_user_or_agent_comments() {
    let mut plan = unlinked_task_plan();
    plan.comments.push(
        serde_json::from_value(serde_json::json!({
            "id": "c1", "author": "user", "severity": "concern", "state": "open",
            "anchor": { "block": "t1" }, "thread": [{ "author": "user", "text": "hmm" }]
        }))
        .unwrap(),
    );
    let findings = lint::lint(&plan, &Policy::default(), None);
    lint::reconcile(&mut plan, &findings);
    // Clearing lint findings must leave the user comment intact.
    lint::reconcile(&mut plan, &[]);
    assert_eq!(plan.comments.len(), 1);
    assert_eq!(plan.comments[0].author.as_deref(), Some("user"));
}

#[test]
fn reconcile_is_idempotent() {
    let mut plan = unlinked_task_plan();
    let findings = lint::lint(&plan, &Policy::default(), None);
    assert!(lint::reconcile(&mut plan, &findings));
    let rev_after_first = plan.rev;
    // Re-running with the same findings changes nothing (no rev churn).
    assert!(!lint::reconcile(&mut plan, &findings));
    assert_eq!(plan.rev, rev_after_first);
    assert_eq!(plan_lint_comments(&plan).len(), 1);
}

#[test]
fn store_helpers_are_reachable() {
    // Sanity: the fixture round-trips (guards against schema drift in lint fixtures).
    let dir = scratch("roundtrip");
    let plan: Plan = serde_json::from_str(FIXTURE).unwrap();
    store::save(&dir, &plan).unwrap();
    assert!(store::load(&dir, &plan.id).is_ok());
}
