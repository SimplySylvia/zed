//! Thin `git` shell wrappers for the server side (F10.2 launch guard, commit
//! recording). These are the only place the Plan feature shells `git`; the
//! decision logic they feed is pure in `plan_core::git`, and the UI reads live
//! state via `project::git_store` instead. Never linked into Zed.

// The server is a standalone binary; a blocking `git` call inside a synchronous
// tool function is fine (the disallowed-methods lint targets async contexts).
#![allow(clippy::disallowed_methods)]

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

fn run(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").arg("-C").arg(repo).args(args).output()?;
    if !output.status.success() {
        bail!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Whether the working tree has uncommitted changes **outside `.plans/`** (F10.2).
/// The plan file is git-versioned and the UI rewrites it at launch, so `.plans/`
/// churn must not count as a dirty tree.
pub fn is_dirty(repo: &Path) -> Result<bool> {
    let porcelain = run(repo, &["status", "--porcelain"])?;
    Ok(porcelain.lines().any(|line| {
        // Porcelain line: `XY <path>` (path starts at column 3).
        let path = line.get(3..).unwrap_or("").trim();
        !path.is_empty() && !path.starts_with(".plans/")
    }))
}

/// How many commits `base` is ahead of `HEAD` — the stale-base count (F10.2).
/// Returns 0 when the base can't be resolved (degrade, don't block).
pub fn behind_count(repo: &Path, base: &str) -> u32 {
    let range = format!("HEAD..{base}");
    run(repo, &["rev-list", "--count", &range])
        .ok()
        .and_then(|out| out.trim().parse().ok())
        .unwrap_or(0)
}
