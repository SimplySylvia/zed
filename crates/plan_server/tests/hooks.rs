//! Hook decision logic (the enforcement core, exercised by the Claude Code
//! hook subcommands): edits are blocked unless a plan is executing, and the
//! resume briefing reflects the active plan.

use std::fs;
use std::path::PathBuf;

use plan_server::{hooks, tools};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_hooks_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn non_edit_tools_are_always_allowed() {
    let dir = scratch("read"); // no plan at all
    assert!(hooks::pretooluse_allows_edit(&dir, "Read").is_ok());
    assert!(hooks::pretooluse_allows_edit(&dir, "Grep").is_ok());
}

#[test]
fn edits_blocked_when_no_plan() {
    let dir = scratch("noplan");
    assert!(hooks::pretooluse_allows_edit(&dir, "Edit").is_err());
    assert!(hooks::pretooluse_allows_edit(&dir, "Write").is_err());
}

#[test]
fn edits_blocked_when_plan_not_executing() {
    let dir = scratch("drafting");
    tools::create(&dir, "P", "t", "g", "acp-1").unwrap(); // status: drafting
    let err = hooks::pretooluse_allows_edit(&dir, "Edit").unwrap_err();
    assert!(err.contains("P"), "reason should name the plan: {err}");
}

#[test]
fn edits_allowed_when_plan_executing() {
    let dir = scratch("executing");
    tools::create(&dir, "P", "t", "g", "acp-1").unwrap();
    tools::set_status(&dir, "P", "executing").unwrap();
    assert!(hooks::pretooluse_allows_edit(&dir, "Edit").is_ok());
}

#[test]
fn resume_briefing_reflects_active_plan() {
    let dir = scratch("briefing");
    assert!(hooks::resume_briefing(&dir).is_none(), "no plan → no briefing");
    tools::create(&dir, "LED-5", "t", "g", "acp-1").unwrap();
    let briefing = hooks::resume_briefing(&dir).expect("briefing for the single plan");
    assert!(briefing.contains("LED-5"), "briefing should name the plan: {briefing}");
}

#[test]
fn active_plan_prefers_the_executing_one() {
    let dir = scratch("active");
    tools::create(&dir, "A", "t", "g", "acp-1").unwrap();
    tools::create(&dir, "B", "t", "g", "acp-2").unwrap();
    tools::set_status(&dir, "B", "executing").unwrap();
    assert_eq!(hooks::active_plan(&dir).unwrap().id, "B");
}
