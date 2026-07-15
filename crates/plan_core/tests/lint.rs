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
        "tasks": [{ "id": "t1", "system": "backend", "acceptance": [] }]
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
fn store_helpers_are_reachable() {
    // Sanity: the fixture round-trips (guards against schema drift in lint fixtures).
    let dir = scratch("roundtrip");
    let plan: Plan = serde_json::from_str(FIXTURE).unwrap();
    store::save(&dir, &plan).unwrap();
    assert!(store::load(&dir, &plan.id).is_ok());
}
