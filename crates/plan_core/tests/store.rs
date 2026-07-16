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
fn corrupt_file_recovers_from_newest_backup() {
    let dir = scratch("recover");
    let plan = sample();
    // Two saves → the first revision is backed up.
    store::save(&dir, &plan).unwrap();
    let mut next = plan;
    next.rev += 1;
    store::save(&dir, &next).unwrap();

    // Corrupt the live file (e.g. an unresolved merge conflict).
    fs::write(dir.join("LED-212.plan.json"), "<<<<<<< HEAD\n{ not json").unwrap();
    assert!(store::load(&dir, "LED-212").is_err(), "corrupt file must not load");

    // Recovery falls back to the newest good backup (a designed state, not a crash).
    let outcome = store::load_or_recover(&dir, "LED-212").unwrap();
    assert!(outcome.recovered_from_backup, "should report recovery");
    assert_eq!(outcome.plan.id, "LED-212");
}

#[test]
fn load_or_recover_returns_the_live_file_when_valid() {
    let dir = scratch("recover_clean");
    let plan = sample();
    store::save(&dir, &plan).unwrap();
    let outcome = store::load_or_recover(&dir, "LED-212").unwrap();
    assert!(!outcome.recovered_from_backup);
    assert_eq!(outcome.plan, plan);
}

#[test]
fn corrupt_file_with_no_backup_errors() {
    let dir = scratch("recover_none");
    fs::write(dir.join("X.plan.json"), "{ not json").unwrap();
    assert!(store::load_or_recover(&dir, "X").is_err());
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
