//! Shared "follow the active thread's plan" logic for the Plan tab, status pill,
//! and dock panel. Each surface owns its own `AgentPanel` subscription + reload
//! poll but delegates resolution to a [`PlanFollower`] so there is one
//! implementation of "which plan does the active thread map to".

use std::path::{Path, PathBuf};

use agent_ui::AgentPanel;
use gpui::{App, Entity, WeakEntity};
use plan_core::{Plan, store};
use workspace::Workspace;

/// Tracks the plan for the workspace's active agent thread.
pub struct PlanFollower {
    plans_dir: PathBuf,
    agent_panel: Option<WeakEntity<AgentPanel>>,
    session: Option<String>,
    plan: Option<Plan>,
}

impl PlanFollower {
    pub fn new(workspace: &Workspace, cx: &App) -> Self {
        Self {
            plans_dir: plans_dir(workspace, cx),
            agent_panel: workspace.panel::<AgentPanel>(cx).map(|panel| panel.downgrade()),
            session: None,
            plan: None,
        }
    }

    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    /// The `.plans/` directory this follower resolves against (for direct writes).
    pub fn plans_dir(&self) -> &Path {
        &self.plans_dir
    }

    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// The workspace's agent panel (used to reach the active thread's events).
    pub fn agent_panel(&self) -> Option<Entity<AgentPanel>> {
        self.agent_panel.as_ref().and_then(WeakEntity::upgrade)
    }

    /// Recompute the plan from the active thread. Returns true if it changed
    /// (so the caller can `cx.notify()` only when needed).
    pub fn refresh(&mut self, cx: &App) -> bool {
        let session = self
            .agent_panel
            .as_ref()
            .and_then(WeakEntity::upgrade)
            .and_then(|panel| active_session(cx, &panel));
        let plan = resolve_plan(&self.plans_dir, session.as_deref());
        let changed = plan != self.plan;
        self.session = session;
        self.plan = plan;
        changed
    }
}

/// `.plans/` under the first visible worktree (where the agent/server operate).
pub fn plans_dir(workspace: &Workspace, cx: &App) -> PathBuf {
    workspace
        .project()
        .read(cx)
        .visible_worktrees(cx)
        .next()
        .map(|worktree| worktree.read(cx).abs_path().join(".plans"))
        .unwrap_or_else(|| PathBuf::from(".plans"))
}

/// The active thread's ACP session id (matches `plan.thread`; see zed-notes).
pub fn active_session(cx: &App, agent_panel: &Entity<AgentPanel>) -> Option<String> {
    agent_panel
        .read(cx)
        .active_agent_thread(cx)
        .map(|thread| thread.read(cx).session_id().0.to_string())
}

/// The plan the tab/pill/panel should show: the thread's plan (by session id)
/// if one matches, else the sole plan in `.plans/` when there is exactly one
/// (convenience before the `plan.thread`↔session-id binding is stamped).
pub fn resolve_plan(plans_dir: &Path, session: Option<&str>) -> Option<Plan> {
    if let Some(session) = session {
        if let Some(plan) = store::find_by_thread(plans_dir, session) {
            return Some(plan);
        }
    }
    match store::list_plan_ids(plans_dir).as_slice() {
        // Recover from the newest `.bak` if the live file is corrupt (F11.5) — a
        // designed state rather than a blank tab on an unresolved merge conflict.
        [only] => store::load_or_recover(plans_dir, only).ok().map(|outcome| outcome.plan),
        _ => None,
    }
}
