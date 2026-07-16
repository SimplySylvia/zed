//! GPUI surfaces for the Plan feature: the Plan tab, dock panel, status-bar
//! pill, and review UI. Everything user-visible lives here.
//!
//! M0 ships only a placeholder dock [`PlanPanel`], registered behind the
//! `ZED_PLAN` environment flag, to prove the fork's panel-registration diff and
//! the build loop. Real surfaces arrive in M3+ (see docs/milestones/).
//!
//! Fork discipline (PRD Part II §2): `plan_ui` may depend on
//! workspace/agent_ui/editor/theme; nothing may depend on `plan_ui`.

use std::time::Duration;

use acp_thread::{AgentThreadEntry, ToolCallStatus};
use agent_ui::AgentPanelEvent;
use anyhow::Result;
use gpui::{
    Animation, AnimationExt, AnyElement, App, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, Hsla, IntoElement, Pixels, Render, SharedString, Subscription,
    WeakEntity, Window, actions, px, pulsating_between,
};
use plan_core::{Plan, Status, Task, TaskStatus, exec, store};
use settings::Settings;
use ui::prelude::*;
use ui::{
    Button, ButtonStyle, CommonAnimationExt, Icon, IconButton, IconSize, Indicator, TintColor,
    Tooltip,
};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

pub mod following;
pub mod plan_pill;
pub mod plan_settings;
pub mod plan_view;

use crate::following::PlanFollower;

actions!(plan_panel, [ToggleFocus]);

const PLAN_PANEL_KEY: &str = "PlanPanel";

/// Whether the Plan feature is enabled: the durable `"plan".enabled` setting
/// (F12.1) **or** the `ZED_PLAN` env flag (kept for dev/CI, and because a personal
/// fork can't use Zed's server-driven feature flags). Toggling the setting takes
/// effect on the next launch (registration happens once at startup).
pub fn plan_enabled(cx: &App) -> bool {
    if matches!(std::env::var("ZED_PLAN").as_deref(), Ok("1") | Ok("true")) {
        return true;
    }
    plan_settings::PlanSettings::get_global(cx).enabled
}

/// Initializes the Plan UI. A no-op unless [`plan_enabled`] is true, so a
/// disabled build behaves exactly like upstream.
pub fn init(cx: &mut App) {
    if !plan_enabled(cx) {
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

    fn position(&self, _window: &Window, cx: &App) -> DockPosition {
        crate::plan_settings::PlanSettings::get_global(cx).dock
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Bottom | DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Persist to the "plan".dock setting so right-click "move" survives restarts.
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let fs = workspace.read(cx).app_state().fs.clone();
        settings::update_settings_file(fs, cx, move |settings, _| {
            settings.plan.get_or_insert_default().dock = Some(position.into());
        });
        cx.notify();
    }

    fn starts_open(&self, _window: &Window, cx: &App) -> bool {
        crate::plan_settings::PlanSettings::get_global(cx).auto_open
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
        // Must be unique across all panels (the dock enforces this for a
        // deterministic status-bar order). 6 collided with OutlinePanel; a high,
        // fork-distinctive value sorts the Plan toggle last and avoids upstream
        // collisions. Re-homing the panel (settings-driven dock change) runs the
        // uniqueness check, so this must stay unique across upstream merges.
        100
    }
}

impl PlanPanel {
    /// Load the plan fresh, apply `mutate`, save, and refresh — the panel's direct
    /// write path for its contextual actions (F5.1/F4.5).
    fn mutate(&mut self, cx: &mut Context<Self>, mutate: impl FnOnce(&mut Plan) -> Result<()>) {
        let Some(id) = self.follower.plan().map(|plan| plan.id.clone()) else {
            return;
        };
        let dir = self.follower.plans_dir().to_path_buf();
        let Ok(mut fresh) = store::load(&dir, &id) else {
            return;
        };
        if mutate(&mut fresh).is_ok() && store::save(&dir, &fresh).is_ok() {
            self.refresh(cx);
        }
    }

    fn pause(&mut self, cx: &mut Context<Self>) {
        self.mutate(cx, exec::pause);
    }

    fn resume(&mut self, cx: &mut Context<Self>) {
        self.mutate(cx, exec::resume);
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.mutate(cx, exec::stop);
    }

    fn approve_gate(&mut self, task: String, cx: &mut Context<Self>) {
        self.mutate(cx, move |plan| exec::approve_gate(plan, &task));
    }

    fn clear_guard(&mut self, task: String, step: String, cx: &mut Context<Self>) {
        self.mutate(cx, move |plan| exec::clear_guard(plan, &task, &step, None));
    }

    /// Contextual header actions (§4/§10): matches the state — Approve gate / Record
    /// input while holding, else Pause/Resume, plus Stop while running.
    fn panel_actions(&self, plan: &Plan, cx: &Context<Self>) -> Vec<AnyElement> {
        let hold = exec::current_hold(plan);
        let running = matches!(plan.status, Status::Executing | Status::Paused);
        let mut actions: Vec<AnyElement> = Vec::new();
        match &hold {
            Some(hold) if hold.kind == "gate" => {
                let task = hold.task.clone();
                actions.push(
                    Button::new("panel-approve-gate", "✓ Approve gate")
                        .style(ButtonStyle::Tinted(TintColor::Accent))
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.approve_gate(task.clone(), cx)
                        }))
                        .into_any_element(),
                );
            }
            Some(hold) => {
                let task = hold.task.clone();
                let step = hold.step.clone().unwrap_or_default();
                let label = if hold.kind == "input" {
                    "Record input"
                } else {
                    "Approve to run"
                };
                actions.push(
                    Button::new("panel-clear-guard", label)
                        .style(ButtonStyle::Tinted(TintColor::Accent))
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.clear_guard(task.clone(), step.clone(), cx)
                        }))
                        .into_any_element(),
                );
            }
            None if plan.status == Status::Executing => actions.push(
                Button::new("panel-pause", "⏸ Pause")
                    .on_click(cx.listener(|this, _, _window, cx| this.pause(cx)))
                    .into_any_element(),
            ),
            None if plan.status == Status::Paused => actions.push(
                Button::new("panel-resume", "▶ Resume")
                    .on_click(cx.listener(|this, _, _window, cx| this.resume(cx)))
                    .into_any_element(),
            ),
            None => {}
        }
        if running || hold.is_some() {
            actions.push(
                Button::new("panel-stop", "⏹ Stop")
                    .on_click(cx.listener(|this, _, _window, cx| this.stop(cx)))
                    .into_any_element(),
            );
        }
        actions
    }

    /// Open the full Plan document tab (F1.3 "Open as tab").
    fn open_plan_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                plan_view::PlanView::open_tab(workspace, window, cx);
            });
        }
    }

    fn render_plan(&self, plan: &Plan, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().colors().border_variant;
        // Left/right docks are narrow ("mobile") — stack the pipeline + live columns
        // vertically instead of side-by-side (there isn't room for two columns).
        let stacked = matches!(
            crate::plan_settings::PlanSettings::get_global(cx).dock,
            DockPosition::Left | DockPosition::Right
        );
        let pipeline = v_flex()
            .flex_1()
            .min_h_0()
            .px_3()
            .py_2()
            .child(panel_pipeline(plan, cx));
        let activity = v_flex()
            .flex_1()
            .min_h_0()
            .px_3()
            .py_2()
            .child(panel_activity(&self.activity, cx));
        let body = if stacked {
            v_flex()
                .flex_1()
                .min_h_0()
                .child(pipeline.border_b_1().border_color(border))
                .child(activity)
                .into_any_element()
        } else {
            h_flex()
                .flex_1()
                .min_h_0()
                .child(pipeline.border_r_1().border_color(border))
                .child(activity)
                .into_any_element()
        };
        v_flex()
            .size_full()
            .child(
                // Header (§4): dot · title · sync receipt · contextual actions.
                h_flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(border)
                    .child(status_dot(&plan.status, panel_dot_color(&plan.status), "plan-panel-dot"))
                    .child(Label::new(format!("Plan · {}", plan.id)))
                    .child({
                        let (receipt, behind) = sync_receipt(plan);
                        Label::new(receipt)
                            .buffer_font(cx)
                            .size(LabelSize::XSmall)
                            .color(if behind {
                                Color::Warning
                            } else {
                                Color::Placeholder
                            })
                    })
                    .child(div().flex_1())
                    .children(self.panel_actions(plan, cx))
                    .child(
                        // Cycle the dock position Left → Bottom → Right (persists via
                        // set_position); mirrors the dock's move-to-next-position.
                        IconButton::new("cycle-plan-dock", IconName::ArrowRightLeft)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Move panel (left / bottom / right)"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let next = match crate::plan_settings::PlanSettings::get_global(cx)
                                    .dock
                                {
                                    DockPosition::Left => DockPosition::Bottom,
                                    DockPosition::Bottom => DockPosition::Right,
                                    DockPosition::Right => DockPosition::Left,
                                };
                                this.set_position(next, window, cx);
                            })),
                    )
                    .child(
                        IconButton::new("open-plan-tab", IconName::Maximize)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Open as tab"))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.open_plan_tab(window, cx)),
                            ),
                    ),
            )
            // Body (§4): pipeline + live activity — side-by-side when docked bottom,
            // stacked when docked left/right (narrow).
            .child(body)
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
                Some(plan) => self.render_plan(plan, cx).into_any_element(),
            })
    }
}

/// One row of the live activity column, kept theme-agnostic (color at render).
#[derive(Clone, PartialEq)]
struct ActivityRow {
    verb: String,
    detail: String,
    kind: ActivityKind,
}

#[derive(Clone, PartialEq)]
enum ActivityKind {
    Running,
    Done,
    Failed,
    Other,
}

/// Map a thread entry to an activity row (tool calls only). The verb column is the
/// tool kind (edit/execute/read/…); the detail is the tool name (§4 live column).
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
    let verb = format!("{:?}", call.kind).to_lowercase();
    let detail = call
        .tool_name
        .as_ref()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "tool".to_string());
    Some(ActivityRow { verb, detail, kind })
}

/// Pipeline column (§4): task rows with a status glyph/spinner, a label, a
/// right-aligned mono note (sha / running / GATE / guard), and a state tint.
fn panel_pipeline(plan: &Plan, cx: &App) -> impl IntoElement {
    let status = cx.theme().status();
    v_flex().gap_0p5().children(plan.tasks.iter().enumerate().map(move |(index, task)| {
        let glyph: AnyElement = if task.status == TaskStatus::InProgress {
            Icon::new(IconName::ArrowCircle)
                .size(IconSize::XSmall)
                .color(Color::Accent)
                .with_rotate_animation(2)
                .into_any_element()
        } else {
            let (glyph, color) = panel_task_glyph(task);
            Label::new(glyph).size(LabelSize::Small).color(color).into_any_element()
        };
        let guarded = task.steps.iter().any(|step| {
            step.guard.as_ref().and_then(|guard| guard.state.as_deref()) == Some("holding")
        });
        let note = task
            .artifacts
            .sha
            .as_ref()
            .map(|sha| format!("⌥ {}", sha.chars().take(7).collect::<String>()))
            .or_else(|| task.gate.then(|| "GATE".to_string()))
            .or_else(|| guarded.then(|| "✋ guard".to_string()))
            .or_else(|| (task.status == TaskStatus::InProgress).then(|| "running".to_string()));
        // Row tint by state (§4): active info · failed error · gate/guard amber.
        let tint = if task.status == TaskStatus::InProgress {
            Some((status.info_background, status.info))
        } else if task.status == TaskStatus::Failed {
            Some((status.deleted_background, status.deleted))
        } else if task.gate || guarded {
            Some((status.modified_background, status.modified))
        } else {
            None
        };
        h_flex()
            .gap_2()
            .items_center()
            .px_1()
            .py_0p5()
            .rounded_md()
            .when_some(tint, |row, (bg, border)| row.bg(bg).border_1().border_color(border))
            .child(glyph)
            .child(
                div().flex_1().min_w_0().child(
                    Label::new(format!(
                        "{} · {}",
                        index + 1,
                        task.title.clone().unwrap_or_else(|| task.id.clone())
                    ))
                    .size(LabelSize::Small)
                    .color(if task.status == TaskStatus::Done {
                        Color::Muted
                    } else {
                        Color::Default
                    }),
                ),
            )
            .when_some(note, |row, note| {
                row.child(
                    Label::new(note)
                        .buffer_font(cx)
                        .size(LabelSize::XSmall)
                        .color(Color::Placeholder),
                )
            })
    }))
}

/// Live activity column (§4): mono rows with a teal verb column + detail; failures
/// red, completions green, live rows default.
fn panel_activity(rows: &[ActivityRow], cx: &App) -> impl IntoElement {
    let verb_color = cx
        .theme()
        .syntax()
        .style_for_name("type")
        .and_then(|style| style.color)
        .unwrap_or(cx.theme().colors().text_accent);
    v_flex().gap_0p5().children(rows.iter().map(move |row| {
        let color = match row.kind {
            ActivityKind::Done => Color::Created,
            ActivityKind::Failed => Color::Error,
            ActivityKind::Running => Color::Default,
            ActivityKind::Other => Color::Custom(verb_color),
        };
        h_flex()
            .gap_2()
            .child(
                div()
                    .min_w(px(34.))
                    .child(Label::new(row.verb.clone()).buffer_font(cx).size(LabelSize::XSmall).color(color)),
            )
            .child(
                Label::new(row.detail.clone())
                    .buffer_font(cx)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
    }))
}


/// The sync receipt (F5.3b): the plan's rev + latest history timestamp.
/// The panel header sync receipt (§10). Returns the receipt text and whether the
/// agent is a rev behind — when the latest agent-authored revision trails the plan's
/// current rev the caller renders the amber "syncs before next task" variant.
fn sync_receipt(plan: &Plan) -> (String, bool) {
    let at = plan.history.iter().rev().find_map(|entry| entry.at.clone());
    // The rev the executing agent last synced to: its most recent revision entry.
    let agent_rev = plan
        .history
        .iter()
        .rev()
        .find(|entry| {
            entry.by.as_deref() == Some("agent") && entry.kind.as_deref() == Some("revision")
        })
        .and_then(|entry| entry.rev);
    let behind = agent_rev.is_some_and(|rev| rev < plan.rev);
    let synced_rev = if behind {
        agent_rev.unwrap_or(plan.rev)
    } else {
        plan.rev
    };
    let text = match (behind, at) {
        (true, Some(at)) => format!(
            "agent synced rev {synced_rev} · {at} — rev {} syncs before next task",
            plan.rev
        ),
        (true, None) => {
            format!("agent synced rev {synced_rev} — rev {} syncs before next task", plan.rev)
        }
        (false, Some(at)) => format!("agent synced rev {synced_rev} · {at}"),
        (false, None) => format!("agent synced rev {synced_rev}"),
    };
    (text, behind)
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

// ── Shared visual helpers (compliance §0) ───────────────────────────────────

/// A filled, full-radius tinted mono pill for metadata (compliance G7 + G9):
/// SHAs, rule ids, anchor labels, provenance chips. The inner label carries the
/// buffer (mono) font; the fill is the role color at 10% opacity.
pub(crate) fn mono_chip(text: impl Into<SharedString>, color: Hsla, cx: &App) -> impl IntoElement {
    mono_chip_chassis(text, color, cx, true)
}

/// The ticket-reference variant of [`mono_chip`] (compliance G9 exception, mockup
/// `.tkchip2`): a bordered 4px-radius mono chip rather than a full pill. Used for
/// ticket keys and ticket AC references, some of which carry no `⛓` glyph.
pub(crate) fn mono_chip_ticket(
    text: impl Into<SharedString>,
    color: Hsla,
    cx: &App,
) -> impl IntoElement {
    mono_chip_chassis(text, color, cx, false)
}

fn mono_chip_chassis(
    text: impl Into<SharedString>,
    color: Hsla,
    cx: &App,
    pill: bool,
) -> impl IntoElement {
    div()
        .border_1()
        .border_color(color)
        .when_else(
            pill,
            |chip| chip.px_1p5().rounded_full().bg(color.opacity(0.1)),
            |chip| chip.px_1().rounded_sm(),
        )
        .child(
            Label::new(text)
                .buffer_font(cx)
                .size(LabelSize::XSmall)
                .color(Color::Custom(color)),
        )
}

/// Lifecycle states whose status dot / pill pulses ("needs you now" / live —
/// compliance G4, design-spec §9): drafting, executing, gate (and guard-hold,
/// which is a runtime facet of those).
pub(crate) fn status_pulses(status: &Status) -> bool {
    matches!(status, Status::Drafting | Status::Executing | Status::Gate)
}

/// Wrap an element in the slow opacity pulse (compliance G4). `id` must be unique
/// within the window. Honors reduced-motion via GPUI's animation system.
pub(crate) fn pulse(element: impl IntoElement, id: &'static str) -> AnyElement {
    div()
        .child(element)
        .with_animation(
            id,
            Animation::new(Duration::from_secs(2))
                .repeat()
                .with_easing(pulsating_between(0.4, 0.95)),
            |element, delta| element.opacity(delta),
        )
        .into_any_element()
}

/// The status dot, pulsing for needs-you/live states (compliance §2/§10 + G4).
pub(crate) fn status_dot(status: &Status, color: Color, id: &'static str) -> AnyElement {
    let dot = Indicator::dot().color(color);
    if status_pulses(status) {
        pulse(dot, id)
    } else {
        dot.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_pulses_matches_needs_you_and_live_states() {
        assert!(status_pulses(&Status::Drafting));
        assert!(status_pulses(&Status::Executing));
        assert!(status_pulses(&Status::Gate));
        assert!(!status_pulses(&Status::Approved));
        assert!(!status_pulses(&Status::InReview));
    }

    #[test]
    fn sync_receipt_includes_rev_and_timestamp() {
        let plan: Plan = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing", "rev": 5,
            "thread": "a", "spec": { "goal": "g" },
            "history": [{ "kind": "revision", "at": "2026-07-11T09:00:00Z" }]
        }))
        .unwrap();
        assert_eq!(
            sync_receipt(&plan),
            ("agent synced rev 5 · 2026-07-11T09:00:00Z".to_string(), false)
        );
    }

    #[test]
    fn sync_receipt_flags_the_agent_a_rev_behind() {
        let plan: Plan = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing", "rev": 6,
            "thread": "a", "spec": { "goal": "g" },
            "history": [
                { "by": "agent", "kind": "revision", "rev": 4, "at": "2026-07-11T09:00:00Z" }
            ]
        }))
        .unwrap();
        let (text, behind) = sync_receipt(&plan);
        assert!(behind);
        assert_eq!(
            text,
            "agent synced rev 4 · 2026-07-11T09:00:00Z — rev 6 syncs before next task"
        );
    }
}
