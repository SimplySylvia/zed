//! Referential-integrity validation: the clean fixture reports nothing, and
//! dangling task/acceptance references are surfaced (as warnings, not fatal —
//! a broken plan stays loadable and repairable).

use plan_core::Plan;

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

fn sample() -> Plan {
    serde_json::from_str(FIXTURE).unwrap()
}

#[test]
fn clean_fixture_has_no_issues() {
    let issues = sample().validate();
    assert!(issues.is_empty(), "expected clean fixture, got: {issues:?}");
}

#[test]
fn dangling_acceptance_task_is_flagged() {
    let mut plan = sample();
    plan.spec.acceptance[0].tasks = vec!["t99".into()];
    assert!(
        plan.validate().iter().any(|m| m.contains("t99")),
        "acceptance referencing a missing task should be flagged"
    );
}

#[test]
fn dangling_depends_on_is_flagged() {
    let mut plan = sample();
    plan.tasks[1].depends_on = vec!["t404".into()];
    assert!(
        plan.validate().iter().any(|m| m.contains("t404")),
        "depends_on referencing a missing task should be flagged"
    );
}

#[test]
fn dangling_task_acceptance_is_flagged() {
    let mut plan = sample();
    plan.tasks[0].acceptance = vec!["a404".into()];
    assert!(
        plan.validate().iter().any(|m| m.contains("a404")),
        "task referencing a missing acceptance id should be flagged"
    );
}
