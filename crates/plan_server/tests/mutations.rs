//! Mutation tools: each loads fresh, changes the typed model, bumps `rev`, and
//! stamps `history[]` (the sync-receipt trail), then saves atomically.

use std::fs;
use std::path::{Path, PathBuf};

use plan_core::{Status, TaskStatus, store};
use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_mut_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &Path) {
    tools::create(dir, "P", "Title", "Goal", "acp-1").unwrap();
}

#[test]
fn set_status_updates_and_bumps_rev_and_history() {
    let dir = scratch("status");
    seed(&dir);
    let before = store::load(&dir, "P").unwrap();
    let after = tools::set_status(&dir, "P", "approved").unwrap();
    assert_eq!(after.status, Status::Approved);
    assert_eq!(after.rev, before.rev + 1);
    assert_eq!(after.history.len(), before.history.len() + 1);
}

#[test]
fn set_status_rejects_unknown_status() {
    let dir = scratch("bad_status");
    seed(&dir);
    assert!(tools::set_status(&dir, "P", "not_a_status").is_err());
}

#[test]
fn add_task_appends_a_task() {
    let dir = scratch("add");
    seed(&dir);
    let after = tools::add_task(&dir, "P", "t1", "Do the thing", Some("backend")).unwrap();
    assert_eq!(after.tasks.len(), 1);
    assert_eq!(after.tasks[0].id, "t1");
    assert_eq!(after.tasks[0].status, TaskStatus::Pending);
}

#[test]
fn task_update_sets_status_and_appends_timeline() {
    let dir = scratch("taskupd");
    seed(&dir);
    tools::add_task(&dir, "P", "t1", "Do the thing", None).unwrap();
    let after = tools::task_update(&dir, "P", "t1", Some("done"), Some("finished it")).unwrap();
    assert_eq!(after.tasks[0].status, TaskStatus::Done);
    assert!(after.tasks[0].timeline.iter().any(|e| e.detail.as_deref() == Some("finished it")));
}

#[test]
fn task_update_errors_on_unknown_task() {
    let dir = scratch("taskbad");
    seed(&dir);
    assert!(tools::task_update(&dir, "P", "nope", Some("done"), None).is_err());
}

#[test]
fn answer_question_records_the_answer() {
    let dir = scratch("answer");
    // Seed a plan carrying an open question.
    let plan: plan_core::Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "drafting", "rev": 1,
        "thread": "acp-1",
        "spec": { "goal": "g", "open_questions": [{ "id": "q1", "text": "Offset or cursor?", "answer": null }] }
    })).unwrap();
    store::save(&dir, &plan).unwrap();

    let after = tools::answer_question(&dir, "P", "q1", "offset").unwrap();
    assert_eq!(
        after.spec.open_questions[0].answer,
        Some(serde_json::json!("offset"))
    );
}

#[test]
fn update_section_sets_spec_goal() {
    let dir = scratch("section");
    seed(&dir);
    let after =
        tools::update_section(&dir, "P", "spec.goal", serde_json::json!("A sharper goal")).unwrap();
    assert_eq!(after.spec.goal, "A sharper goal");
}
