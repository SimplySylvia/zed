//! Git policy (Appendix B `policy.git`) + pure branch/commit helpers (F10.1/
//! F10.2/F10.3/F10.5c). Facts-in, decision-out — no git shelling here.

use std::fs;

use plan_core::Plan;
use plan_core::git::{self, GitPolicy};

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

fn fixture() -> Plan {
    serde_json::from_str(FIXTURE).unwrap()
}

fn plan(value: serde_json::Value) -> Plan {
    serde_json::from_value(value).unwrap()
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_core_git_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn branch_name_from_ticket_and_title_slug() {
    let plan = fixture();
    assert_eq!(
        git::branch_name(&plan, &GitPolicy::default()),
        "led-212-add-pagination-to-the-invoices-table"
    );
}

#[test]
fn branch_name_ticketless_is_slug_only() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "Tidy up the CSS", "status": "approved",
        "rev": 1, "thread": "a", "spec": { "goal": "g" }
    }));
    assert_eq!(git::branch_name(&plan, &GitPolicy::default()), "tidy-up-the-css");
}

#[test]
fn base_for_uses_the_feature_base() {
    let plan = fixture();
    let policy = GitPolicy {
        base_feature: "develop".to_string(),
        ..GitPolicy::default()
    };
    assert_eq!(git::base_for(&plan, &policy), "develop");
}

#[test]
fn trailer_for_ticketed_task() {
    let plan = fixture(); // rev 5, task t1 carries ticket LED-212
    let task = plan.tasks.iter().find(|task| task.id == "t1").unwrap();
    assert_eq!(git::trailer_for(&plan, task), "Plan: LED-212 rev5 task-t1");
}

#[test]
fn trailer_for_ticketless_task_drops_the_ticket_token() {
    let mut plan = fixture();
    plan.tickets.clear();
    for task in &mut plan.tasks {
        task.ticket = None;
    }
    let task = &plan.tasks[0];
    assert_eq!(git::trailer_for(&plan, task), format!("Plan: rev5 task-{}", task.id));
}

#[test]
fn commit_subject_ok_accepts_a_well_formed_ticketed_subject() {
    let policy = GitPolicy::default();
    assert!(git::commit_subject_ok(&policy, "[LED-212]: accept page params in invoices loader", true));
    // No ticket prefix -> rejected under the ticketed format.
    assert!(!git::commit_subject_ok(&policy, "accept page params in invoices loader", true));
    // Subject body under 8 chars -> rejected.
    assert!(!git::commit_subject_ok(&policy, "[LED-212]: short", true));
}

#[test]
fn commit_subject_ok_ticketless_uses_the_no_ticket_format() {
    let policy = GitPolicy::default();
    assert!(git::commit_subject_ok(&policy, "accept page params in invoices loader", false));
    assert!(!git::commit_subject_ok(&policy, "short", false));
}

#[test]
fn commit_ok_requires_subject_and_trailer() {
    let policy = GitPolicy::default();
    let good = "[LED-212]: accept page params in invoices loader\n\nPlan: LED-212 rev5 task-t1";
    assert!(git::commit_ok(&policy, good, Some("LED-212")).is_ok());

    // Missing trailer.
    let no_trailer = "[LED-212]: accept page params in invoices loader";
    assert!(git::commit_ok(&policy, no_trailer, Some("LED-212")).is_err());

    // Bad subject.
    let bad_subject = "wip\n\nPlan: LED-212 rev5 task-t1";
    assert!(git::commit_ok(&policy, bad_subject, Some("LED-212")).is_err());
}

#[test]
fn launch_block_flags_dirty_and_stale() {
    let policy = GitPolicy::default();
    assert!(git::launch_block(&policy, false, 0).is_none());
    assert!(git::launch_block(&policy, true, 0).is_some());
    assert!(git::launch_block(&policy, false, 2).is_some());
}

#[test]
fn is_destructive_blocks_history_rewrites_not_safe_ops() {
    assert!(git::is_destructive("git reset --hard HEAD~1"));
    assert!(git::is_destructive("git push --force origin main"));
    assert!(git::is_destructive("git push -f origin main"));
    assert!(git::is_destructive("git branch -D feature"));
    assert!(git::is_destructive("git clean -fd"));

    assert!(!git::is_destructive("git commit -m 'x'"));
    assert!(!git::is_destructive("git status"));
    assert!(!git::is_destructive("git revert 3f81c2a")); // revert is a new commit (F10.5c safe)
}

#[test]
fn git_policy_absent_falls_back_to_defaults() {
    let dir = scratch("nopolicy");
    let policy = GitPolicy::load(&dir);
    assert_eq!(policy.destructive_ops, "amendment_only");
    assert!(policy.one_commit_per_task);
}

#[test]
fn git_policy_json_overrides_defaults() {
    let dir = scratch("policy");
    let policy_json = serde_json::json!({
        "git": {
            "branch": { "pattern": "{ticket}-{slug}", "base": { "feature": "develop", "hotfix": "main" } },
            "commit": { "format": "^\\[X-\\d+\\]: .{4,50}$", "trailer": "Ref: {ticket}" },
            "destructive_ops": "amendment_only"
        }
    });
    fs::write(dir.join("policy.json"), serde_json::to_string_pretty(&policy_json).unwrap()).unwrap();
    let policy = GitPolicy::load(&dir);
    assert_eq!(policy.base_feature, "develop");
    assert!(git::commit_subject_ok(&policy, "[X-9]: tidy", true));
    assert!(!git::commit_subject_ok(&policy, "[LED-1]: tidy up", true));
}
