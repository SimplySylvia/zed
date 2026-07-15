//! GPUI surfaces for the Plan feature: the Plan tab, dock panel, status-bar
//! pill, and review UI. Everything user-visible lives here.
//!
//! M0 ships only a placeholder dock [`PlanPanel`], registered behind the
//! `ZED_PLAN` environment flag, to prove the fork's panel-registration diff and
//! the build loop. Real surfaces arrive in M3+ (see docs/milestones/).
//!
//! Fork discipline (PRD Part II §2): `plan_ui` may depend on
//! workspace/agent_ui/editor/theme; nothing may depend on `plan_ui`.

use acp_thread::{AgentThreadEntry, ToolCallStatus};
use agent_ui::AgentPanelEvent;
use anyhow::Result;
use gpui::{
    App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Pixels, Render, Subscription, WeakEntity, Window, actions, px,
};
use plan_core::{Plan, Status, Task, TaskStatus};
use ui::Indicator;
use ui::prelude::*;
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

pub mod following;
pub mod plan_pill;
pub mod plan_view;

use crate::following::PlanFollower;

actions!(plan_panel, [ToggleFocus]);

const PLAN_PANEL_KEY: &str = "PlanPanel";

/// Whether the Plan feature is enabled for this session, via the `ZED_PLAN`
/// environment flag. M0 uses an env flag rather than Zed's server-driven
/// feature flags (which a personal fork can't control) or the `"plan"` setting
/// (which arrives in M9). Set `ZED_PLAN=1` (or `ZED_PLAN=true`) to enable.
pub fn plan_enabled() -> bool {
    matches!(std::env::var("ZED_PLAN").as_deref(), Ok("1") | Ok("true"))
}

/// Initializes the Plan UI. A no-op unless [`plan_enabled`] is true, so an
/// unflagged build behaves exactly like upstream.
pub fn init(cx: &mut App) {
    if !plan_enabled() {
        return;
    }
    cx.observe_new(|workspace: &mut Workspace, _window, cx| {
        register(workspace);
        plan_view::PlanView::register(workspace, cx);
    })
    .detach();
}

fn register(workspace: &mut Workspace) {
    workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
        workspace.toggle_panel_focus::<PlanPanel>(window, cx);
    });
}

/// The Plan dock panel (F1.3): a compact bottom-dock surface that follows the
/// active thread and shows the pipeline (task status) beside a live activity
/// column. Colors resolve through `cx.theme()`.
pub struct PlanPanel {
    focus_handle: FocusHandle,
    follower: PlanFollower,
    #[allow(dead_code)]
    workspace: WeakEntity<Workspace>,
    activity: Vec<ActivityRow>,
    _agent_subscription: Option<Subscription>,
    _watch_task: Option<gpui::Task<()>>,
}

impl PlanPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, _window, cx| {
            let follower = PlanFollower::new(workspace, cx);
            let handle = workspace.weak_handle();
            cx.new(|cx| PlanPanel::new(follower, handle, cx))
        })
    }

    fn new(
        follower: PlanFollower,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            follower,
            workspace,
            activity: Vec::new(),
            _agent_subscription: None,
            _watch_task: None,
        };
        if let Some(agent_panel) = panel.follower.agent_panel() {
            panel._agent_subscription =
                Some(cx.subscribe(&agent_panel, |this, _panel, event, cx| {
                    if matches!(event, AgentPanelEvent::ActiveViewChanged) {
                        this.refresh(cx);
                    }
                }));
        }
        panel.refresh(cx);
        panel._watch_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                if view.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
            }
        }));
        panel
    }

    /// Re-resolve the plan and rebuild the activity feed; notify on any change.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let plan_changed = self.follower.refresh(cx);
        let activity_changed = self.rebuild_activity(cx);
        if plan_changed || activity_changed {
            cx.notify();
        }
    }

    /// Rebuild the activity list from the active thread's entries (polled). Keeps
    /// the most recent [`MAX_ACTIVITY`] tool-call rows. Returns true if changed.
    fn rebuild_activity(&mut self, cx: &App) -> bool {
        let mut rows: Vec<ActivityRow> = self
            .follower
            .agent_panel()
            .and_then(|panel| panel.read(cx).active_agent_thread(cx))
            .map(|thread| {
                thread
                    .read(cx)
                    .entries()
                    .iter()
                    .filter_map(activity_row)
                    .collect()
            })
            .unwrap_or_default();
        if rows.len() > MAX_ACTIVITY {
            rows.drain(0..rows.len() - MAX_ACTIVITY);
        }
        let changed = rows != self.activity;
        self.activity = rows;
        changed
    }
}

const MAX_ACTIVITY: usize = 50;

impl Focusable for PlanPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for PlanPanel {}

impl Panel for PlanPanel {
    fn persistent_name() -> &'static str {
        "PlanPanel"
    }

    fn panel_key() -> &'static str {
        PLAN_PANEL_KEY
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Bottom | DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // M0 placeholder: position isn't persisted yet. Backing this with the
        // "plan" setting (so right-click "move" survives restarts) lands in M9.
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(200.)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::ListTodo)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Plan")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        6
    }
}

impl Render for PlanPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel_background = cx.theme().colors().panel_background;
        v_flex()
            .size_full()
            .bg(panel_background)
            .child(match self.follower.plan() {
                None => v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child(
                        Label::new("no plan — ask the agent to draft one").color(Color::Muted),
                    )
                    .into_any_element(),
                Some(plan) => panel_body(plan, &self.activity).into_any_element(),
            })
    }
}

/// One row of the live activity column, kept theme-agnostic (color at render).
#[derive(Clone, PartialEq)]
struct ActivityRow {
    text: String,
    kind: ActivityKind,
}

#[derive(Clone, PartialEq)]
enum ActivityKind {
    Running,
    Done,
    Failed,
    Other,
}

/// Map a thread entry to an activity row (tool calls only, for M4).
fn activity_row(entry: &AgentThreadEntry) -> Option<ActivityRow> {
    let AgentThreadEntry::ToolCall(call) = entry else {
        return None;
    };
    let kind = match &call.status {
        ToolCallStatus::Completed => ActivityKind::Done,
        ToolCallStatus::Failed => ActivityKind::Failed,
        ToolCallStatus::InProgress => ActivityKind::Running,
        _ => ActivityKind::Other,
    };
    let text = call
        .tool_name
        .as_ref()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "tool".to_string());
    Some(ActivityRow { text, kind })
}

fn panel_body(plan: &Plan, activity: &[ActivityRow]) -> impl IntoElement {
    v_flex()
        .size_full()
        .gap_1()
        .p_2()
        .child(panel_header(plan))
        .child(
            h_flex()
                .flex_1()
                .gap_4()
                .child(
                    v_flex()
                        .flex_1()
                        .gap_1()
                        .child(section_label("PIPELINE"))
                        .child(panel_pipeline(plan)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .gap_1()
                        .child(section_label("ACTIVITY"))
                        .child(panel_activity(activity)),
                ),
        )
}

fn panel_header(plan: &Plan) -> impl IntoElement {
    let done = plan
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Done)
        .count();
    h_flex()
        .gap_2()
        .items_center()
        .child(Indicator::dot().color(panel_dot_color(&plan.status)))
        .child(Label::new(format!("Plan · {}", plan.id)))
        .child(
            Label::new(format!("{done}/{}", plan.tasks.len()))
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
        .child(
            Label::new(sync_receipt(plan))
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
}

fn panel_pipeline(plan: &Plan) -> impl IntoElement {
    v_flex().gap_0p5().children(plan.tasks.iter().map(|task| {
        let (glyph, color) = panel_task_glyph(task);
        h_flex()
            .gap_2()
            .child(Label::new(glyph).color(color))
            .child(
                Label::new(task.title.clone().unwrap_or_else(|| task.id.clone()))
                    .size(LabelSize::Small)
                    .color(if task.status == TaskStatus::Done {
                        Color::Muted
                    } else {
                        Color::Default
                    }),
            )
    }))
}

fn panel_activity(rows: &[ActivityRow]) -> impl IntoElement {
    v_flex().gap_0p5().children(rows.iter().map(|row| {
        let color = match row.kind {
            ActivityKind::Done => Color::Created,
            ActivityKind::Failed => Color::Error,
            ActivityKind::Running => Color::Accent,
            ActivityKind::Other => Color::Muted,
        };
        Label::new(row.text.clone())
            .size(LabelSize::Small)
            .color(color)
    }))
}

fn section_label(title: &'static str) -> impl IntoElement {
    Label::new(title).size(LabelSize::Small).color(Color::Muted)
}

/// The sync receipt (F5.3b): the plan's rev + latest history timestamp.
fn sync_receipt(plan: &Plan) -> String {
    match plan.history.iter().rev().find_map(|entry| entry.at.clone()) {
        Some(at) => format!("synced rev {} · {}", plan.rev, at),
        None => format!("synced rev {}", plan.rev),
    }
}

fn panel_dot_color(status: &Status) -> Color {
    match status {
        Status::Executing => Color::Accent,
        Status::Approved | Status::Done => Color::Created,
        Status::InReview | Status::Revising | Status::Gate | Status::Amending => Color::Modified,
        Status::Drafting => Color::Placeholder,
        _ => Color::Muted,
    }
}

fn panel_task_glyph(task: &Task) -> (&'static str, Color) {
    if task.gate && task.status == TaskStatus::Pending {
        return ("⏸", Color::Modified);
    }
    match task.status {
        TaskStatus::Pending => ("○", Color::Placeholder),
        TaskStatus::InProgress => ("◐", Color::Accent),
        TaskStatus::Done => ("✓", Color::Created),
        TaskStatus::Failed => ("✕", Color::Error),
        TaskStatus::Skipped => ("–", Color::Muted),
        TaskStatus::Interrupted => ("⚠", Color::Warning),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_receipt_includes_rev_and_timestamp() {
        let plan: Plan = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing", "rev": 5,
            "thread": "a", "spec": { "goal": "g" },
            "history": [{ "kind": "revision", "at": "2026-07-11T09:00:00Z" }]
        }))
        .unwrap();
        assert_eq!(sync_receipt(&plan), "synced rev 5 · 2026-07-11T09:00:00Z");
    }
}
