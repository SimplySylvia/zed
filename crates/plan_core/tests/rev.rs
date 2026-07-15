//! Pending-revision (staged diff) apply/reject/resolve round-trips (F9.3, F3.5).

use plan_core::rev::{self, HunkSpec};
use plan_core::{Plan, store};

fn plan() -> Plan {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "steps": [
            { "id": "s1", "text": "old text" },
            { "id": "s2", "text": "second" }
        ] }],
        "comments": [{ "id": "c1", "kind": "flag", "author": "user", "state": "open",
            "anchor": { "lens": "tasks", "block": "t1.s1", "quote": "old text" } }]
    }))
    .unwrap()
}

fn spec(target: &str, new: &str, from: Option<&str>) -> HunkSpec {
    HunkSpec {
        target: Some(target.to_string()),
        old: None,
        new: Some(new.to_string()),
        from: from.map(String::from),
    }
}

#[test]
fn stage_revision_is_inert() {
    let mut plan = plan();
    rev::stage_revision(
        &mut plan,
        vec![spec("t1.s1", "new s1", None), spec("t1.s2", "new s2", None)],
    );
    let pending = plan.pending_revision.as_ref().expect("staged");
    assert_eq!(pending.hunks.len(), 2);
    assert_eq!(pending.rev, Some(2));
    assert!(pending.hunks.iter().all(|h| h.state.as_deref() == Some("pending")));
    // Staging must not touch blocks or bump rev.
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("old text"));
    assert_eq!(plan.rev, 1);
}

#[test]
fn apply_and_reject_then_resolve_bumps_once() {
    let mut plan = plan();
    rev::stage_revision(
        &mut plan,
        vec![spec("t1.s1", "new s1", None), spec("t1.s2", "new s2", None)],
    );
    assert!(rev::apply_hunk(&mut plan, "h1"));
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("new s1"));
    assert!(rev::reject_hunk(&mut plan, "h2"));
    assert_eq!(rev::resolve_revision(&mut plan), Some(2));
    assert_eq!(plan.rev, 2);
    // Rejected hunk left its block untouched.
    assert_eq!(plan.tasks[0].steps[1].text.as_deref(), Some("second"));
    assert!(plan.pending_revision.is_none());
    assert!(plan.history.iter().any(|h| h.kind.as_deref() == Some("revision")));
}

#[test]
fn resolve_records_provenance_on_source_comment() {
    let mut plan = plan();
    rev::stage_revision(&mut plan, vec![spec("t1.s1", "new s1", Some("c1"))]);
    assert!(rev::apply_hunk(&mut plan, "h1"));
    assert_eq!(rev::resolve_revision(&mut plan), Some(2));
    let comment = &plan.comments[0];
    assert!(comment.caused_changes.iter().any(|c| c == "t1.s1"));
    assert_eq!(comment.state.as_deref(), Some("addressed"));
}

#[test]
fn apply_all_commits_every_hunk() {
    let mut plan = plan();
    rev::stage_revision(
        &mut plan,
        vec![spec("t1.s1", "new s1", None), spec("t1.s2", "new s2", None)],
    );
    assert_eq!(rev::apply_all(&mut plan), Some(2));
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("new s1"));
    assert_eq!(plan.tasks[0].steps[1].text.as_deref(), Some("new s2"));
    assert_eq!(plan.rev, 2);
    assert!(plan.pending_revision.is_none());
}

#[test]
fn reject_all_clears_without_bump() {
    let mut plan = plan();
    rev::stage_revision(
        &mut plan,
        vec![spec("t1.s1", "new s1", None), spec("t1.s2", "new s2", None)],
    );
    assert_eq!(rev::reject_all(&mut plan), None);
    assert_eq!(plan.rev, 1);
    assert!(plan.pending_revision.is_none());
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("old text"));
}

#[test]
fn resolve_noops_while_a_hunk_is_still_pending() {
    let mut plan = plan();
    rev::stage_revision(
        &mut plan,
        vec![spec("t1.s1", "new s1", None), spec("t1.s2", "new s2", None)],
    );
    assert!(rev::apply_hunk(&mut plan, "h1"));
    // One hunk is still pending -> resolve is a no-op.
    assert_eq!(rev::resolve_revision(&mut plan), None);
    assert!(plan.pending_revision.is_some());
    assert_eq!(plan.rev, 1);
}

#[test]
fn apply_hunk_unknown_id_returns_false() {
    let mut plan = plan();
    rev::stage_revision(&mut plan, vec![spec("t1.s1", "new s1", None)]);
    assert!(!rev::apply_hunk(&mut plan, "nope"));
    assert!(!rev::apply_hunk(&mut {
        let mut p = plan.clone();
        p.pending_revision = None;
        p
    }, "h1"));
}

#[test]
fn round_trips_through_disk() {
    let dir = std::env::temp_dir().join("plan_core_rev_roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut plan = plan();
    rev::stage_revision(&mut plan, vec![spec("t1.s1", "new s1", Some("c1"))]);
    store::save(&dir, &plan).unwrap();
    let mut reloaded = store::load(&dir, "P").unwrap();
    assert_eq!(rev::apply_all(&mut reloaded), Some(2));
    assert_eq!(reloaded.tasks[0].steps[0].text.as_deref(), Some("new s1"));
}
