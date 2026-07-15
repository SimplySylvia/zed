//! Plan resolution helpers used by the tab/panel/hooks to find the plan for a
//! thread and enumerate plans in a directory.

use std::fs;
use std::path::{Path, PathBuf};

use plan_core::{Plan, store};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_core_resolve_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &Path, id: &str, thread: &str) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": id, "title": "t", "status": "drafting",
        "rev": 1, "thread": thread, "spec": { "goal": "g" }
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn list_plan_ids_returns_all_sorted() {
    let dir = scratch("list");
    seed(&dir, "B", "t2");
    seed(&dir, "A", "t1");
    assert_eq!(store::list_plan_ids(&dir), vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn list_plan_ids_ignores_non_plan_files() {
    let dir = scratch("ignore");
    seed(&dir, "A", "t1");
    fs::write(dir.join("notes.txt"), "x").unwrap();
    assert_eq!(store::list_plan_ids(&dir), vec!["A".to_string()]);
}

#[test]
fn find_by_thread_matches_the_owning_plan() {
    let dir = scratch("find");
    seed(&dir, "A", "acp-1");
    seed(&dir, "B", "acp-2");
    assert_eq!(store::find_by_thread(&dir, "acp-2").unwrap().id, "B");
    assert!(store::find_by_thread(&dir, "acp-nope").is_none());
}
