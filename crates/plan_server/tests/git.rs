//! Git integration (M7a): branch/commit tools, the commit-time hook check
//! (format + trailer + destructive-op block), and the launch dirty/stale guard
//! (F10.1/F10.2/F10.3/F10.5c).

// Synchronous integration test shelling `git` + driving the binary over stdio;
// the disallowed-methods guard (async-blocking process spawn) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use plan_core::{Plan, Status, store};
use plan_server::{hooks, tools};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_git_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A ticketed, executing plan with one in-progress task.
fn seed_executing(plans_dir: &Path) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "acp-1", "executor": { "thread": "acp-1" },
        "tickets": [{ "source": "jira", "key": "LED-1" }],
        "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "ticket": "LED-1", "status": "in_progress" }]
    }))
    .unwrap();
    store::save(plans_dir, &plan).unwrap();
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// A fresh git repo on `main` with one commit; returns (repo_root, plans_dir).
fn git_repo(name: &str) -> (PathBuf, PathBuf) {
    let repo = scratch(name);
    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["-c", "init.defaultBranch=main", "init"])
        .status()
        .expect("git init");
    git(&repo, &["config", "user.email", "t@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    fs::write(repo.join("README.md"), "seed\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "-m", "seed"]);
    let plans = repo.join(".plans");
    fs::create_dir_all(&plans).unwrap();
    (repo, plans)
}

fn seed_approved(plans_dir: &Path) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "LED-212", "title": "Add pagination to the invoices table",
        "status": "approved", "rev": 1, "thread": "acp-1",
        "tickets": [{ "source": "jira", "key": "LED-212" }],
        "spec": { "goal": "g" }, "tasks": [{ "id": "t1", "ticket": "LED-212" }]
    }))
    .unwrap();
    store::save(plans_dir, &plan).unwrap();
}

#[test]
fn set_branch_stamps_git_without_bumping_rev() {
    let dir = scratch("setbranch");
    seed_approved(&dir);
    let before = store::load(&dir, "LED-212").unwrap().rev;
    tools::set_branch(&dir, "LED-212", "led-212-x", "main", None).unwrap();
    let plan = store::load(&dir, "LED-212").unwrap();
    let git = plan.git.expect("git block");
    assert_eq!(git.branch.as_deref(), Some("led-212-x"));
    assert_eq!(git.base.as_deref(), Some("main"));
    assert_eq!(plan.rev, before, "recording branch is telemetry, not a plan revision");
}

#[test]
fn record_commit_stamps_task_and_git() {
    let dir = scratch("record");
    seed_executing(&dir);
    tools::record_commit(&dir, "P", "t1", "3f81c2a", Some("+42 -6")).unwrap();
    let plan = store::load(&dir, "P").unwrap();
    let task = plan.tasks.iter().find(|task| task.id == "t1").unwrap();
    assert_eq!(task.artifacts.sha.as_deref(), Some("3f81c2a"));
    assert_eq!(task.artifacts.diffstat.as_deref(), Some("+42 -6"));
    let commits = &plan.git.expect("git block").commits;
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].sha.as_deref(), Some("3f81c2a"));
}

#[test]
fn commit_hook_denies_bad_format() {
    let dir = scratch("badformat");
    seed_executing(&dir);
    let result = hooks::pretooluse_gate(&dir, "Bash", Some(r#"git commit -m "wip""#));
    assert!(result.is_err(), "malformed commit subject should be denied");
}

#[test]
fn commit_hook_denies_missing_trailer() {
    let dir = scratch("notrailer");
    seed_executing(&dir);
    let result = hooks::pretooluse_gate(
        &dir,
        "Bash",
        Some(r#"git commit -m "[LED-1]: accept page params in the loader""#),
    );
    assert!(result.is_err(), "commit without a Plan trailer should be denied");
}

#[test]
fn commit_hook_allows_well_formed_commit() {
    let dir = scratch("goodcommit");
    seed_executing(&dir);
    let result = hooks::pretooluse_gate(
        &dir,
        "Bash",
        Some(r#"git commit -m "[LED-1]: accept page params in the loader" -m "Plan: LED-1 rev1 task-t1""#),
    );
    assert!(result.is_ok(), "well-formed ticketed commit should pass: {result:?}");
}

#[test]
fn commit_hook_denies_destructive_git() {
    let dir = scratch("destructive");
    seed_executing(&dir);
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("git reset --hard HEAD~1")).is_err());
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("git push --force origin main")).is_err());
}

#[test]
fn commit_hook_allows_revert_and_non_git() {
    let dir = scratch("safe");
    seed_executing(&dir);
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("git revert 3f81c2a")).is_ok());
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("ls -la")).is_ok());
}

#[test]
fn launch_blocks_on_dirty_tree() {
    let (repo, plans) = git_repo("dirty");
    seed_approved(&plans);
    // An untracked code file (not under .plans/) makes the tree dirty.
    fs::write(repo.join("scratch.txt"), "wip\n").unwrap();
    let result = tools::launch(&plans, "LED-212", "acp-1");
    assert!(result.is_err(), "launch must refuse on a dirty tree");
}

#[test]
fn launch_stamps_branch_on_clean_repo() {
    let (_repo, plans) = git_repo("clean");
    seed_approved(&plans);
    // Only the untracked .plans/ file is present — excluded from the dirty check.
    let plan = tools::launch(&plans, "LED-212", "acp-1").expect("launch on clean repo");
    assert_eq!(plan.status, Status::Executing);
    assert_eq!(
        plan.git.expect("git block").branch.as_deref(),
        Some("led-212-add-pagination-to-the-invoices-table")
    );
}

#[test]
fn set_branch_over_mcp() {
    let dir = scratch("mcp");
    seed_approved(&dir);
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_set_branch","arguments":{"id":"LED-212","branch":"led-212-x","base":"main"}}}"#,
    ]
    .join("\n")
        + "\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_plan_server"))
        .env("PLAN_PLANS_DIR", &dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn plan_server");
    child.stdin.take().unwrap().write_all(messages.as_bytes()).unwrap();
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    assert!(child.wait().unwrap().success());
    let plan = store::load(&dir, "LED-212").unwrap();
    assert_eq!(plan.git.expect("git").branch.as_deref(), Some("led-212-x"));
}
