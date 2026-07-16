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
    Animation, AnimationExt, AnyElement, App, AsyncWindowContext, Context, ElementId, Entity,
    EventEmitter, FocusHandle, Focusable, FontWeight, Hsla, IntoElement, Pixels, Render,
    SharedString, Subscription, WeakEntity, Window, actions, px, pulsating_between,
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
        // Caps section header (§4): the plan's uppercase caps state, tinted by role.
        let state = crate::display_state(plan);
        let role = state.role().color(cx);
        let activity = v_flex()
            .flex_1()
            .min_h_0()
            .px_3()
            .py_2()
            .child(
                div()
                    .pb_1()
                    .mb_1()
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(role)
                    .child(SharedString::from(state.caps_label())),
            )
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
    verb: &'static str,
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

/// Curated verb column (§10) for an ACP tool kind, keyed on the protocol's own
/// canonical snake_case name (from `ToolKind`'s `serde(rename_all = "snake_case")`)
/// rather than its `Debug` impl. The §10 lifecycle verbs (guard/hook/ev/drift/plan)
/// are not derivable from the tool-call feed, so they're intentionally absent here.
fn tool_verb(name: &str) -> &'static str {
    match name {
        "read" | "search" => "read",
        "edit" | "delete" | "move" => "edit",
        "execute" => "run",
        "think" => "plan",
        "fetch" => "fetch",
        _ => "tool",
    }
}

/// Map a thread entry to an activity row (tool calls only). The verb column is a
/// curated verb for the tool kind (edit/run/read/…); the detail is the tool name
/// (§4 live column).
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
    // Serialize the ACP `ToolKind` to its canonical snake_case name and map that to a
    // curated verb; fall back to "tool" if serialization ever fails to yield a string.
    let verb = serde_json::to_value(call.kind)
        .ok()
        .and_then(|value| value.as_str().map(tool_verb))
        .unwrap_or("tool");
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
        let running = row.kind == ActivityKind::Running;
        let color = match row.kind {
            ActivityKind::Done => Color::Created,
            ActivityKind::Failed => Color::Error,
            // Live rows read as accent (the pulsing dot picks up the same tint).
            ActivityKind::Running => Color::Accent,
            ActivityKind::Other => Color::Custom(verb_color),
        };
        // Fixed-width leading slot: the pulsing accent dot marks the live row; other
        // rows leave the slot empty so the verb column stays aligned across rows.
        let lead: AnyElement = if running {
            crate::pulse(
                Indicator::dot().color(Color::Accent).into_any_element(),
                "plan-live-dot",
            )
        } else {
            div().into_any_element()
        };
        h_flex()
            .gap_2()
            .child(div().w(px(8.)).flex_none().child(lead))
            .child(
                div()
                    .min_w(px(34.))
                    .child(Label::new(row.verb).buffer_font(cx).size(LabelSize::XSmall).color(color)),
            )
            .child(
                Label::new(row.detail.clone())
                    .buffer_font(cx)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
    }))
}


/// The panel header sync receipt (§10, F5.3b). Returns the receipt text and whether the
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

/// Wrap `chip` so hovering it surfaces a small floating "peek card" (F0.5b) of the
/// referenced target's current text. The `peek` string is resolved at the call site
/// (where the plan + the reference are in scope) and this closure only displays it.
/// `id` must be unique per instance — these chips repeat across cards, so a shared
/// id would make hover ambiguous. Instant (no entrance animation) and reduced-motion
/// safe (adds no animation). The chip's own visual output is unchanged; the peek is
/// purely additive (an id'd wrapper + hover tooltip).
pub(crate) fn peek_chip(
    id: impl Into<ElementId>,
    chip: impl IntoElement,
    peek: SharedString,
    cx: &App,
) -> impl IntoElement {
    let editor_background = cx.theme().colors().editor_background;
    div()
        .id(id)
        .child(chip)
        .hoverable_tooltip(Tooltip::element(move |_window, _cx| {
            v_flex()
                .max_w(px(360.))
                .p_2()
                .rounded_md()
                .bg(editor_background)
                .child(
                    Label::new(peek.clone())
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element()
        }))
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

// ── Display state (design-spec §9) ───────────────────────────────────────────

/// The fine-grained lifecycle surface for a plan, derived from its contents — the
/// single source for the status-bar pill's fragment/color (and the needs-you
/// pulse). The coarse [`Status`] enum can't express sub-states like a failed task,
/// staged revisions, open lint, or review comment/blocker counts; this can. See
/// [`display_state`] for the derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DisplayState {
    Intake { questions: usize },
    Drafting,
    Lint { open: usize },
    InReview { comments: usize, blockers: usize },
    RevStaged,
    Approved,
    GuardHold,
    Executing { done: usize, total: usize, acc_done: usize, acc_total: usize },
    TaskFailed { task: String },
    Gate { drift: bool },
    Done { pr: Option<u64> },
    Paused,
    Abandoned,
}

impl DisplayState {
    /// The semantic family color for this state (design-spec §9) — the single source
    /// for the pill fragment color, the toolbar caps pill, and matching banners.
    pub(crate) fn role(&self) -> Color {
        match self {
            // Answered intake ("intake ✓ — drafting") is muted, not a needs-you amber.
            DisplayState::Intake { questions } if *questions == 0 => Color::Muted,
            DisplayState::Intake { .. } => Color::Warning,
            DisplayState::Drafting => Color::Muted,
            DisplayState::Lint { .. } => Color::Warning,
            DisplayState::InReview { .. } => Color::Warning,
            DisplayState::RevStaged => Color::Warning,
            DisplayState::Approved => Color::Created,
            DisplayState::GuardHold => Color::Warning,
            DisplayState::Executing { .. } => Color::Info,
            DisplayState::TaskFailed { .. } => Color::Error,
            DisplayState::Gate { .. } => Color::Warning,
            DisplayState::Done { .. } => Color::Created,
            DisplayState::Paused => Color::Muted,
            DisplayState::Abandoned => Color::Muted,
        }
    }

    /// The caps state name for the toolbar status pill (design-spec §3.1).
    pub(crate) fn caps_label(&self) -> &'static str {
        match self {
            DisplayState::Intake { .. } => "INTAKE",
            DisplayState::Drafting => "DRAFTING",
            DisplayState::Lint { .. } => "LINT",
            DisplayState::InReview { .. } => "IN REVIEW",
            DisplayState::RevStaged => "REV STAGED",
            DisplayState::Approved => "APPROVED",
            DisplayState::GuardHold => "GUARD HOLD",
            DisplayState::Executing { .. } => "EXECUTING",
            DisplayState::TaskFailed { .. } => "FAILED",
            DisplayState::Gate { .. } => "GATE",
            DisplayState::Done { .. } => "DONE",
            DisplayState::Paused => "PAUSED",
            DisplayState::Abandoned => "ABANDONED",
        }
    }

    /// Whether this state is a "needs you now" hold that pulses (contract §1) — only
    /// a GATE or a guard-hold, unlike the status dots which also pulse for live states.
    pub(crate) fn pulses(&self) -> bool {
        matches!(self, DisplayState::Gate { .. } | DisplayState::GuardHold)
    }
}

/// Any ticket carries stamped drift (F2.4g) — decorates the GATE surface with a
/// "+ drift" note so a mid-execution ticket change is visible at the pill.
fn any_ticket_drift(plan: &Plan) -> bool {
    plan.tickets
        .iter()
        .any(|ticket| plan_core::tickets::stamped_drift(ticket).is_some())
}

/// Derive the [`DisplayState`] from a plan's contents (design-spec §9 matrix).
///
/// Precedence (a runtime hold overrides the lifecycle status):
/// 1. A GATE hold ([`exec::current_hold`] with `kind == "gate"`) → `Gate { drift }`,
///    where `drift` is set if any ticket has stamped drift.
/// 2. Any other hold (a holding step guard) → `GuardHold`.
/// 3. Otherwise, by `plan.status`:
///    - `Amending` → `TaskFailed { task }` — the first `Failed` task's id (falling
///      back to the first task id, else `"task"`).
///    - `Done` → `Done { pr }` — the recorded PR number from `plan.git.pr.number`.
///    - `Executing` → `Executing { done, total, acc_done, acc_total }`.
///    - `Paused` / `Abandoned` / `Approved` → the matching leaf.
///    - `Gate` (without a live hold) → `Gate { drift }` (defensive; normally a
///      GATE status coincides with a hold and is caught above).
///    - `InReview` / `Revising`: a staged `pending_revision` → `RevStaged`; else
///      count non-lint, non-resolved comments + open blockers → `InReview` when
///      either is non-zero; else open lint findings → `Lint`; else empty `InReview`.
///    - `Drafting`: open lint findings → `Lint`, else `Drafting`.
///    - `Intake` → `Intake { questions }` — unanswered open questions.
pub(crate) fn display_state(plan: &Plan) -> DisplayState {
    if let Some(hold) = exec::current_hold(plan) {
        if hold.kind == "gate" {
            return DisplayState::Gate { drift: any_ticket_drift(plan) };
        }
        return DisplayState::GuardHold;
    }
    match plan.status {
        Status::Amending => {
            let task = plan
                .tasks
                .iter()
                .find(|task| task.status == TaskStatus::Failed)
                .or_else(|| plan.tasks.first())
                .map(|task| task.id.clone())
                .unwrap_or_else(|| "task".to_string());
            DisplayState::TaskFailed { task }
        }
        Status::Done => {
            let pr = plan
                .git
                .as_ref()
                .and_then(|git| git.pr.as_ref())
                .and_then(|pr| pr.get("number"))
                .and_then(|number| number.as_u64());
            DisplayState::Done { pr }
        }
        Status::Executing => DisplayState::Executing {
            done: plan
                .tasks
                .iter()
                .filter(|task| task.status == TaskStatus::Done)
                .count(),
            total: plan.tasks.len(),
            acc_done: plan.spec.acceptance.iter().filter(|criterion| criterion.done).count(),
            acc_total: plan.spec.acceptance.len(),
        },
        Status::Paused => DisplayState::Paused,
        Status::Abandoned => DisplayState::Abandoned,
        Status::Approved => DisplayState::Approved,
        Status::Gate => DisplayState::Gate { drift: any_ticket_drift(plan) },
        Status::InReview | Status::Revising => {
            if plan.pending_revision.is_some() {
                return DisplayState::RevStaged;
            }
            // `open_blocker_count` is author-agnostic, so a lint comment carrying
            // blocker severity would count toward `blockers` and short-circuit the
            // `Lint` branch below (and the lint card would still render its ⚑ glyph).
            // Acceptable because lint findings are authored at concern severity in
            // practice; this only matters if that ever changes.
            let blockers = plan_view::open_blocker_count(plan);
            let comments = plan
                .comments
                .iter()
                .filter(|comment| {
                    comment.author.as_deref() != Some(plan_core::lint::LINT_AUTHOR)
                        && comment.state.as_deref() != Some("resolved")
                })
                .count();
            if comments > 0 || blockers > 0 {
                DisplayState::InReview { comments, blockers }
            } else {
                let open = plan_view::open_lint_count(plan);
                if open > 0 {
                    DisplayState::Lint { open }
                } else {
                    DisplayState::InReview { comments: 0, blockers: 0 }
                }
            }
        }
        Status::Drafting => {
            let open = plan_view::open_lint_count(plan);
            if open > 0 {
                DisplayState::Lint { open }
            } else {
                DisplayState::Drafting
            }
        }
        Status::Intake => DisplayState::Intake {
            questions: plan
                .spec
                .open_questions
                .iter()
                .filter(|question| question.answer.is_none())
                .count(),
        },
    }
}

/// A kind of thing that can need the user in the active plan (F5.7 attention
/// queue). Ordered as the PRD's needs-you cycle enumerates them.
// Consumed by the queue rendering in C2; until then the enumeration is exercised
// only by its unit tests.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttentionKind {
    Question,
    Blocker,
    Lint,
    Guard,
    Gate,
    Failed,
    Drift,
    RevBehind,
}

/// One row of the attention queue (F5.7): what needs the user, a short human
/// label, and the lens the row jumps to when actioned. Rendering is C2.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttentionItem {
    pub kind: AttentionKind,
    pub label: SharedString,
    pub lens: plan_view::Lens,
}

/// Enumerate everything that needs the user in the (single, active) plan, in the
/// PRD's needs-you order (F5.7). Pure: no rendering, no side effects. Aggregate
/// signals (questions, blockers, lint findings, drift, rev-behind) each collapse
/// to one counted item; holding guards and failed tasks yield one item apiece.
/// Reuses the existing predicates rather than re-deriving the filters.
#[allow(dead_code)]
pub(crate) fn attention_items(plan: &Plan) -> Vec<AttentionItem> {
    use plan_view::Lens;

    let mut items = Vec::new();

    // 1. Unanswered open questions.
    let questions = plan
        .spec
        .open_questions
        .iter()
        .filter(|question| question.answer.is_none())
        .count();
    if questions > 0 {
        items.push(AttentionItem {
            kind: AttentionKind::Question,
            label: pluralize(questions, "open question").into(),
            lens: Lens::Spec,
        });
    }

    // 2. Non-lint blocker comments not yet resolved. `open_blocker_count` is
    //    author-agnostic; exclude the lint author here so lint is its own kind.
    let blockers = plan
        .comments
        .iter()
        .filter(|comment| {
            comment.severity.as_deref() == Some("blocker")
                && comment.state.as_deref() != Some("resolved")
                && comment.author.as_deref() != Some(plan_core::lint::LINT_AUTHOR)
        })
        .count();
    if blockers > 0 {
        items.push(AttentionItem {
            kind: AttentionKind::Blocker,
            label: pluralize(blockers, "blocker").into(),
            lens: Lens::Spec,
        });
    }

    // 3. Open policy-lint findings.
    let lint = plan_view::open_lint_count(plan);
    if lint > 0 {
        items.push(AttentionItem {
            kind: AttentionKind::Lint,
            label: pluralize(lint, "lint finding").into(),
            lens: Lens::Spec,
        });
    }

    // 4. Every holding step guard that isn't a gate. Mirrors the holding-guard
    //    scan in `exec::current_hold`, but enumerates all holds, not just the first.
    for task in &plan.tasks {
        for step in &task.steps {
            if let Some(guard) = &step.guard {
                if guard.state.as_deref() == Some("holding")
                    && guard.guard_type.as_deref() != Some("gate")
                {
                    items.push(AttentionItem {
                        kind: AttentionKind::Guard,
                        label: format!("guard: {}.{}", task.id, step.id).into(),
                        lens: Lens::Tasks,
                    });
                }
            }
        }
    }

    // 5. The gate hold: an in-progress GATE task, or a plain `Gate` status.
    if let Some(task) = plan
        .tasks
        .iter()
        .find(|task| task.gate && task.status == TaskStatus::InProgress)
    {
        items.push(AttentionItem {
            kind: AttentionKind::Gate,
            label: format!("gate: {}", task.id).into(),
            lens: Lens::Tasks,
        });
    } else if plan.status == Status::Gate {
        let label = plan
            .tasks
            .iter()
            .find(|task| task.gate)
            .map(|task| format!("gate: {}", task.id))
            .unwrap_or_else(|| "gate".to_string());
        items.push(AttentionItem {
            kind: AttentionKind::Gate,
            label: label.into(),
            lens: Lens::Tasks,
        });
    }

    // 6. Every failed task.
    for task in plan.tasks.iter().filter(|task| task.status == TaskStatus::Failed) {
        items.push(AttentionItem {
            kind: AttentionKind::Failed,
            label: format!("{} failed", task.id).into(),
            lens: Lens::Tasks,
        });
    }

    // 7. Any stamped ticket drift.
    if any_ticket_drift(plan) {
        items.push(AttentionItem {
            kind: AttentionKind::Drift,
            label: "ticket drift".into(),
            lens: Lens::Spec,
        });
    }

    // 8. The executing agent trails the plan's current rev (`sync_receipt`'s signal).
    let (_, behind) = sync_receipt(plan);
    if behind {
        items.push(AttentionItem {
            kind: AttentionKind::RevBehind,
            label: "agent a rev behind".into(),
            lens: Lens::Tasks,
        });
    }

    items
}

/// `"1 noun"` / `"{count} nouns"` for the aggregate attention labels.
#[allow(dead_code)]
fn pluralize(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
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

    fn plan(value: serde_json::Value) -> Plan {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn display_state_derivation_table() {
        // (seed plan, expected DisplayState) — the §9 state→surface matrix.
        let base = |status: &str| {
            serde_json::json!({
                "schema_version": 1, "id": "X", "title": "t", "status": status,
                "rev": 1, "thread": "a", "spec": { "goal": "g" }
            })
        };
        let cases: Vec<(Plan, DisplayState)> = vec![
            // Intake counts only unanswered open questions.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "intake",
                    "rev": 1, "thread": "a",
                    "spec": { "goal": "g", "open_questions": [
                        {"id": "q1"}, {"id": "q2", "answer": "yes"}, {"id": "q3"}
                    ] }
                })),
                DisplayState::Intake { questions: 2 },
            ),
            (plan(base("drafting")), DisplayState::Drafting),
            // Drafting + an open lint finding surfaces as Lint.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "drafting",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "comments": [
                        {"id": "lint-a-b", "author": "plan-lint", "state": "open"},
                        {"id": "lint-c-d", "author": "plan-lint", "state": "resolved"}
                    ]
                })),
                DisplayState::Lint { open: 1 },
            ),
            // Review with comments + a blocker.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "in_review",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "comments": [
                        {"id": "c1", "author": "user", "state": "open"},
                        {"id": "c2", "author": "user", "severity": "blocker", "state": "open"},
                        {"id": "l1", "author": "plan-lint", "state": "open"}
                    ]
                })),
                DisplayState::InReview { comments: 2, blockers: 1 },
            ),
            // Review, no user comments, but open lint → Lint.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "in_review",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "comments": [{"id": "l1", "author": "plan-lint", "state": "open"}]
                })),
                DisplayState::Lint { open: 1 },
            ),
            // Review, nothing open → empty InReview.
            (plan(base("in_review")), DisplayState::InReview { comments: 0, blockers: 0 }),
            // Revising with a staged revision → RevStaged.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "revising",
                    "rev": 3, "thread": "a", "spec": { "goal": "g" },
                    "pending_revision": { "hunks": [] }
                })),
                DisplayState::RevStaged,
            ),
            (plan(base("approved")), DisplayState::Approved),
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "executing",
                    "rev": 1, "thread": "a",
                    "spec": { "goal": "g", "acceptance": [
                        {"id": "a1", "done": true}, {"id": "a2", "done": false}
                    ] },
                    "tasks": [{"id": "t1", "status": "done"}, {"id": "t2", "status": "pending"}]
                })),
                DisplayState::Executing { done: 1, total: 2, acc_done: 1, acc_total: 2 },
            ),
            // Amending → TaskFailed with the first failed task's id.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "amending",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "tasks": [
                        {"id": "t1", "status": "done"},
                        {"id": "t2", "status": "failed"}
                    ]
                })),
                DisplayState::TaskFailed { task: "t2".to_string() },
            ),
            // Amending with no failed task falls back to the first task id.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "amending",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "tasks": [{"id": "t1", "status": "done"}]
                })),
                DisplayState::TaskFailed { task: "t1".to_string() },
            ),
            // Done with a recorded PR number.
            (
                plan(serde_json::json!({
                    "schema_version": 1, "id": "X", "title": "t", "status": "done",
                    "rev": 1, "thread": "a", "spec": { "goal": "g" },
                    "git": { "pr": { "number": 42, "url": "https://example/pr/42" } }
                })),
                DisplayState::Done { pr: Some(42) },
            ),
            (plan(base("done")), DisplayState::Done { pr: None }),
            (plan(base("paused")), DisplayState::Paused),
            (plan(base("abandoned")), DisplayState::Abandoned),
        ];
        for (plan, expected) in cases {
            assert_eq!(display_state(&plan), expected, "plan {} status {:?}", plan.id, plan.status);
        }
    }

    #[test]
    fn display_state_gate_hold_overrides_status_and_reads_drift() {
        // A holding GATE task wins over the coarse status, and stamped ticket drift
        // decorates it.
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 1, "thread": "a", "spec": { "goal": "g" },
            "tickets": [{
                "source": "jira", "key": "LED-1",
                "drift": { "status_changed": true, "ac_changed": false, "fields_changed": ["status"] }
            }],
            "tasks": [{"id": "g1", "gate": true, "status": "in_progress"}]
        }));
        assert_eq!(display_state(&plan), DisplayState::Gate { drift: true });
    }

    #[test]
    fn display_state_guard_hold_overrides_status() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 1, "thread": "a", "spec": { "goal": "g" },
            "tasks": [{
                "id": "t1", "status": "in_progress",
                "steps": [{"id": "s1", "guard": {"type": "approve", "state": "holding"}}]
            }]
        }));
        assert_eq!(display_state(&plan), DisplayState::GuardHold);
    }

    use super::plan_view::Lens;

    #[test]
    fn attention_items_enumerates_in_prd_order() {
        // Open question + a (non-lint) blocker + a holding step guard + a failed
        // task → one item each, emitted in the PRD needs-you order.
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 1, "thread": "a",
            "spec": { "goal": "g", "open_questions": [
                {"id": "q1"}, {"id": "q2", "answer": "yes"}
            ] },
            "comments": [
                {"id": "c1", "author": "user", "severity": "blocker", "state": "open"}
            ],
            "tasks": [
                {"id": "t1", "status": "failed"},
                {"id": "t2", "status": "in_progress",
                 "steps": [{"id": "s1", "guard": {"type": "approve", "state": "holding"}}]}
            ]
        }));
        let items = attention_items(&plan);
        let shape: Vec<(AttentionKind, &str, Lens)> = items
            .iter()
            .map(|item| (item.kind, item.label.as_ref(), item.lens))
            .collect();
        assert_eq!(
            shape,
            vec![
                (AttentionKind::Question, "1 open question", Lens::Spec),
                (AttentionKind::Blocker, "1 blocker", Lens::Spec),
                (AttentionKind::Guard, "guard: t2.s1", Lens::Tasks),
                (AttentionKind::Failed, "t1 failed", Lens::Tasks),
            ]
        );
    }

    #[test]
    fn attention_items_aggregates_counts_but_enumerates_instances() {
        // Questions/blockers/lint collapse to one counted item each; failed tasks
        // yield one item apiece. The lint comments are excluded from the blocker
        // count (own kind) even though they, too, are open.
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 1, "thread": "a",
            "spec": { "goal": "g", "open_questions": [{"id": "q1"}, {"id": "q2"}] },
            "comments": [
                {"id": "b1", "author": "user", "severity": "blocker", "state": "open"},
                {"id": "b2", "author": "user", "severity": "blocker", "state": "open"},
                {"id": "l1", "author": "plan-lint", "state": "open"},
                {"id": "l2", "author": "plan-lint", "state": "open"}
            ],
            "tasks": [{"id": "t1", "status": "failed"}, {"id": "t2", "status": "failed"}]
        }));
        let items = attention_items(&plan);
        let shape: Vec<(AttentionKind, &str)> = items
            .iter()
            .map(|item| (item.kind, item.label.as_ref()))
            .collect();
        assert_eq!(
            shape,
            vec![
                (AttentionKind::Question, "2 open questions"),
                (AttentionKind::Blocker, "2 blockers"),
                (AttentionKind::Lint, "2 lint findings"),
                (AttentionKind::Failed, "t1 failed"),
                (AttentionKind::Failed, "t2 failed"),
            ]
        );
    }

    #[test]
    fn attention_items_empty_for_a_clean_plan() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "approved",
            "rev": 1, "thread": "a", "spec": { "goal": "g" }
        }));
        assert!(attention_items(&plan).is_empty());
    }

    #[test]
    fn attention_items_surfaces_the_gate_hold() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 1, "thread": "a", "spec": { "goal": "g" },
            "tasks": [{"id": "g1", "gate": true, "status": "in_progress"}]
        }));
        let items = attention_items(&plan);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, AttentionKind::Gate);
        assert_eq!(items[0].label.as_ref(), "gate: g1");
        assert_eq!(items[0].lens, Lens::Tasks);
    }

    #[test]
    fn attention_items_flags_drift_and_rev_behind() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing",
            "rev": 6, "thread": "a", "spec": { "goal": "g" },
            "tickets": [{
                "source": "jira", "key": "LED-1",
                "drift": { "status_changed": true, "ac_changed": false, "fields_changed": ["status"] }
            }],
            "history": [
                { "by": "agent", "kind": "revision", "rev": 4, "at": "2026-07-11T09:00:00Z" }
            ]
        }));
        let items = attention_items(&plan);
        let shape: Vec<(AttentionKind, &str, Lens)> = items
            .iter()
            .map(|item| (item.kind, item.label.as_ref(), item.lens))
            .collect();
        assert_eq!(
            shape,
            vec![
                (AttentionKind::Drift, "ticket drift", Lens::Spec),
                (AttentionKind::RevBehind, "agent a rev behind", Lens::Tasks),
            ]
        );
    }
}
