//! Execution lifecycle: launch (approved → executing) + the executor lease (F4.1, F11.3).

use plan_core::exec;
use plan_core::{Plan, Status};

fn plan(status: &str) -> Plan {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": status, "rev": 1,
        "thread": "acp-1", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "status": "pending" }]
    }))
    .unwrap()
}

#[test]
fn launch_from_approved_sets_executing_and_lease() {
    let mut plan = plan("approved");
    exec::launch(&mut plan, "acp-1").unwrap();
    assert_eq!(plan.status, Status::Executing);
    let executor = plan.executor.as_ref().expect("lease taken");
    assert_eq!(executor.thread, "acp-1");
    assert_eq!(plan.rev, 2);
    assert!(plan.history.iter().any(|h| h.kind.as_deref() == Some("launch")));
}

#[test]
fn launch_requires_approved() {
    for status in ["in_review", "drafting", "revising"] {
        let mut plan = plan(status);
        assert!(exec::launch(&mut plan, "acp-1").is_err());
        assert_ne!(plan.status, Status::Executing);
        assert!(plan.executor.is_none());
        assert_eq!(plan.rev, 1);
    }
}

#[test]
fn relaunch_by_same_thread_is_idempotent() {
    let mut plan = plan("approved");
    exec::launch(&mut plan, "acp-1").unwrap();
    let rev_after_launch = plan.rev;
    // Already executing + leased by this thread → no error, no rev churn.
    exec::launch(&mut plan, "acp-1").unwrap();
    assert_eq!(plan.status, Status::Executing);
    assert_eq!(plan.rev, rev_after_launch);
}

#[test]
fn launch_blocked_when_another_thread_holds_the_lease() {
    let mut plan = plan("approved");
    plan.executor = Some(serde_json::from_value(serde_json::json!({ "thread": "acp-other" })).unwrap());
    assert!(exec::launch(&mut plan, "acp-1").is_err());
    // The foreign lease is untouched.
    assert_eq!(plan.executor.as_ref().unwrap().thread, "acp-other");
    assert_ne!(plan.status, Status::Executing);
}

#[test]
fn release_lease_clears_the_executor() {
    let mut plan = plan("approved");
    exec::launch(&mut plan, "acp-1").unwrap();
    exec::release_lease(&mut plan);
    assert!(plan.executor.is_none());
}
