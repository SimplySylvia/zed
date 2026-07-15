//! Execution lifecycle: launch (approved → executing) + the executor lease (F4.1,
//! F11.3), plus step guards + GATE holds (F4.5/F4.5b).

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

/// An executing plan with a guarded task (approve + input guards) and a GATE task.
fn guarded_plan() -> Plan {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "acp-1",
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "tasks": ["t1"] }] },
        "tasks": [
            { "id": "t1", "status": "in_progress", "acceptance": ["a1"], "steps": [
                { "id": "s1", "text": "Run the DB migration",
                  "guard": { "type": "approve", "prompt": "about to run prisma migrate — approve to run", "state": "pending" } },
                { "id": "s2", "text": "Verify the export",
                  "guard": { "type": "input", "prompt": "record what you saw", "state": "pending", "evidence_for": ["a1"] } }
            ] },
            { "id": "t2", "title": "Sign-off", "gate": true, "status": "pending" }
        ]
    }))
    .unwrap()
}

#[test]
fn hold_guard_sets_holding() {
    let mut plan = guarded_plan();
    exec::hold_guard(&mut plan, "t1", "s1").unwrap();
    assert_eq!(
        plan.tasks[0].steps[0].guard.as_ref().unwrap().state.as_deref(),
        Some("holding")
    );
    assert!(exec::hold_guard(&mut plan, "t1", "nope").is_err());
}

#[test]
fn clear_approve_guard_leaves_a_cleared_receipt() {
    let mut plan = guarded_plan();
    exec::hold_guard(&mut plan, "t1", "s1").unwrap();
    let before = plan.rev;
    exec::clear_guard(&mut plan, "t1", "s1", None).unwrap();
    assert_eq!(
        plan.tasks[0].steps[0].guard.as_ref().unwrap().state.as_deref(),
        Some("cleared")
    );
    assert!(plan.rev > before);
    assert!(plan.history.iter().any(|h| h.kind.as_deref() == Some("guard_cleared")));
}

#[test]
fn clear_input_guard_stores_response_and_attaches_evidence() {
    let mut plan = guarded_plan();
    exec::clear_guard(&mut plan, "t1", "s2", Some(serde_json::json!("saw 0 rows"))).unwrap();
    let guard = plan.tasks[0].steps[1].guard.as_ref().unwrap();
    assert_eq!(guard.state.as_deref(), Some("cleared"));
    assert_eq!(guard.response.as_ref().unwrap(), &serde_json::json!("saw 0 rows"));
    // evidence_for a1 → an evidence entry landed on the criterion.
    assert!(!plan.spec.acceptance[0].evidence.is_empty());
}

#[test]
fn approve_gate_marks_the_gate_task_done() {
    let mut plan = guarded_plan();
    exec::approve_gate(&mut plan, "t2").unwrap();
    let gate = plan.tasks.iter().find(|t| t.id == "t2").unwrap();
    assert_eq!(gate.status, plan_core::TaskStatus::Done);
    assert!(plan.history.iter().any(|h| h.kind.as_deref() == Some("gate_approved")));
    // Not a gate task → error.
    assert!(exec::approve_gate(&mut plan, "t1").is_err());
}

#[test]
fn current_hold_finds_holding_guard_then_pending_gate() {
    let mut plan = guarded_plan();
    // A holding guard is the active hold.
    exec::hold_guard(&mut plan, "t1", "s1").unwrap();
    let hold = exec::current_hold(&plan).unwrap();
    assert_eq!(hold.task, "t1");
    assert_eq!(hold.step.as_deref(), Some("s1"));

    // With no holding guard but a gate task in progress, the gate is the hold.
    exec::clear_guard(&mut plan, "t1", "s1", None).unwrap();
    if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == "t2") {
        task.status = plan_core::TaskStatus::InProgress;
    }
    let hold = exec::current_hold(&plan).unwrap();
    assert_eq!(hold.task, "t2");
    assert_eq!(hold.kind, "gate");
}

#[test]
fn require_on_matches_command_and_checks_coverage() {
    let policy = exec::GuardPolicy {
        require_on: vec!["prisma migrate".to_string()],
        default_type: "approve".to_string(),
    };
    assert_eq!(
        exec::matched_require_on(&policy, "npx prisma migrate dev").as_deref(),
        Some("prisma migrate")
    );
    assert!(exec::matched_require_on(&policy, "ls -la").is_none());

    // Coverage: the in-progress task must carry a cleared guard whose prompt covers the pattern.
    let mut plan = guarded_plan();
    assert!(!exec::command_guard_cleared(&plan, "prisma migrate"));
    exec::clear_guard(&mut plan, "t1", "s1", None).unwrap(); // s1 prompt mentions "prisma migrate"
    assert!(exec::command_guard_cleared(&plan, "prisma migrate"));
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
