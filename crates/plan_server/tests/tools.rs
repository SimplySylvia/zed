//! plan_get/plan_create tool logic: create then get round-trips, and get always
//! re-reads from disk (never cached) so a UI/user edit is picked up.

use std::fs;
use std::path::PathBuf;

use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_tools_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn create_then_get_round_trips() {
    let dir = scratch("create_get");
    let created = tools::create(&dir, "LED-9", "Add widget", "Ship the widget", "acp-1").unwrap();
    assert_eq!(created.id, "LED-9");
    assert_eq!(created.rev, 1);

    let loaded = tools::get(&dir, "LED-9").unwrap();
    assert_eq!(loaded.title, "Add widget");
    assert_eq!(loaded.spec.goal, "Ship the widget");
    assert_eq!(loaded.thread, "acp-1");
}

#[test]
fn get_reads_fresh_from_disk_never_cached() {
    let dir = scratch("fresh");
    let mut plan = tools::create(&dir, "LED-9", "t", "g", "acp-1").unwrap();
    assert_eq!(tools::get(&dir, "LED-9").unwrap().rev, plan.rev);

    // An external writer (the UI, or the user) bumps the plan on disk.
    plan.rev = 42;
    plan_core::store::save(&dir, &plan).unwrap();

    assert_eq!(
        tools::get(&dir, "LED-9").unwrap().rev,
        42,
        "plan_get must re-read from disk, not serve a cached copy"
    );
}
