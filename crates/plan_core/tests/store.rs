//! Atomic, crash-safe persistence: save/load round-trips, writes leave no temp
//! residue, overwrites back up the prior revision, backups are capped, and a
//! corrupt file errors instead of panicking.

use std::fs;
use std::path::PathBuf;

use plan_core::{Plan, store};

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_core_store_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample() -> Plan {
    serde_json::from_str(FIXTURE).unwrap()
}

#[test]
fn save_then_load_round_trips() {
    let dir = scratch("roundtrip");
    let plan = sample();
    store::save(&dir, &plan).unwrap();
    let loaded = store::load(&dir, &plan.id).unwrap();
    assert_eq!(loaded, plan);
}

#[test]
fn save_leaves_no_temp_file() {
    let dir = scratch("atomic");
    let plan = sample();
    store::save(&dir, &plan).unwrap();
    assert!(dir.join("LED-212.plan.json").exists());
    store::load(&dir, "LED-212").expect("saved file must be readable");

    let leftovers: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
        .map(|e| e.file_name())
        .collect();
    assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
}

#[test]
fn overwrite_backs_up_prior_revision() {
    let dir = scratch("backup");
    let mut plan = sample();
    store::save(&dir, &plan).unwrap(); // rev 5, first write, no backup
    plan.rev = 6;
    store::save(&dir, &plan).unwrap(); // overwrites; backs up rev 5
    assert!(
        dir.join(".bak").join("LED-212.rev5.plan.json").exists(),
        "expected a backup of the overwritten rev 5"
    );
}

#[test]
fn backups_are_capped_at_ten() {
    let dir = scratch("cap");
    let mut plan = sample();
    for rev in 1..=12 {
        plan.rev = rev;
        store::save(&dir, &plan).unwrap();
    }
    let count = fs::read_dir(dir.join(".bak")).unwrap().count();
    assert_eq!(count, 10, "backups should be capped at 10");
}

#[test]
fn malformed_json_errors_without_panicking() {
    let dir = scratch("malformed");
    fs::write(dir.join("LED-212.plan.json"), "{ not valid json").unwrap();
    let err = store::load(&dir, "LED-212").unwrap_err();
    assert!(!err.to_string().is_empty());
}
