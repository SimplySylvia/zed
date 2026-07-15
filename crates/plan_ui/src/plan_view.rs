//! The Plan tab (F1.1): a center-pane `Item` singleton that follows the active
//! thread and renders `plan.json` read-only. M3 builds it incrementally — this
//! module starts with the item shell, empty state, and open action; thread
//! binding, file-watch, lens rendering, and serialization follow.
//!
//! All colors resolve through `cx.theme()` per docs/design/plan-ui-compliance.md.

use std::path::PathBuf;

use crate::following;
use agent_ui::{AgentPanel, AgentPanelEvent};
use anyhow::Result;
use project::Project;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, WeakEntity, Window, actions, px,
};
use plan_core::{Anchor, Comment, Plan, Status, Step, Task, TaskStatus, anchor, comments, store};
use ui::prelude::*;
use ui::{Button, Indicator};
use workspace::{
    Workspace,
    item::{Item, ItemEvent, SerializableItem, TabContentParams},
};

actions!(
    plan,
    [
        /// Open the Plan tab for the active thread.
        OpenPlan
    ]
);

/// The three lenses over one plan.json (F3.1 lens switcher).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lens {
    Spec,
    Design,
    Tasks,
}

/// Uninhabited: the Plan tab is read-only and emits no item events; re-renders
/// are driven by `cx.notify()`.
pub enum PlanViewEvent {}

pub struct PlanView {
    focus_handle: FocusHandle,
    #[allow(dead_code)]
    workspace: WeakEntity<Workspace>,
    plans_dir: PathBuf,
    /// ACP session id of the followed thread (matches `plan.thread`); read by
    /// the watcher (T3).
    #[allow(dead_code)]
    thread_session: Option<String>,
    plan: Option<Plan>,
    lens: Lens,
    _agent_subscription: Option<Subscription>,
    _watch_task: Option<gpui::Task<()>>,
}

impl PlanView {
    /// Register the open action (and, later, the serializable item).
    pub fn register(workspace: &mut Workspace, cx: &mut Context<Workspace>) {
        workspace.register_action(Self::open);
        workspace::register_serializable_item::<PlanView>(cx);
    }

    fn open(workspace: &mut Workspace, _: &OpenPlan, window: &mut Window, cx: &mut Context<Workspace>) {
        // Singleton: reuse the existing tab if present, else open one. Bind the
        // lookup to a local first so its borrow of `workspace`/`cx` is released.
        let existing = workspace.items_of_type::<Self>(cx).next();
        if let Some(existing) = existing {
            workspace.activate_item(&existing, true, true, window, cx);
            return;
        }
        let plans_dir = following::plans_dir(workspace, cx);
        let handle = cx.entity().downgrade();
        let view = cx.new(|cx| PlanView {
            focus_handle: cx.focus_handle(),
            workspace: handle,
            plans_dir,
            thread_session: None,
            plan: None,
            lens: Lens::Tasks,
            _agent_subscription: None,
            _watch_task: None,
        });
        workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
    }

    /// Re-point the tab at the active thread's plan (F1.1). The binding key is
    /// the ACP session id (matches `plan.thread`), read from the active thread.
    fn retarget(&mut self, agent_panel: Entity<AgentPanel>, cx: &mut Context<Self>) {
        let session = following::active_session(cx, &agent_panel);
        self.plan = following::resolve_plan(&self.plans_dir, session.as_deref());
        self.thread_session = session;
        if let Some(lens) = self.plan.as_ref().map(|plan| default_lens(&plan.status)) {
            self.lens = lens;
        }
        cx.notify();
    }

    /// Re-read the bound plan from disk, re-rendering only if it changed. Drives
    /// the "watch the agent draft live" demo. M3 polls at 500ms (the §7 fallback);
    /// a proper fs watch is a later refinement.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let fresh = following::resolve_plan(&self.plans_dir, self.thread_session.as_deref());
        if fresh != self.plan {
            self.plan = fresh;
            cx.notify();
        }
    }

    /// The lifecycle status-dot color (design spec §9 tab-dot column).
    fn status_color(&self) -> Color {
        match self.plan.as_ref().map(|plan| &plan.status) {
            Some(Status::Executing) => Color::Accent,
            Some(Status::Approved | Status::Done) => Color::Created,
            Some(Status::InReview | Status::Revising | Status::Gate | Status::Amending) => {
                Color::Modified
            }
            Some(Status::Drafting) => Color::Placeholder,
            _ => Color::Muted,
        }
    }
}

impl Focusable for PlanView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PlanViewEvent> for PlanView {}

impl Item for PlanView {
    type Event = PlanViewEvent;

    fn to_item_events(_event: &Self::Event, _f: &mut dyn FnMut(ItemEvent)) {}

    fn added_to_workspace(
        &mut self,
        workspace: &mut Workspace,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(agent_panel) = workspace.panel::<AgentPanel>(cx) else {
            return;
        };
        self._agent_subscription = Some(cx.subscribe(&agent_panel, |this, panel, event, cx| {
            if matches!(event, AgentPanelEvent::ActiveViewChanged) {
                this.retarget(panel, cx);
            }
        }));
        self.retarget(agent_panel, cx);
        self._watch_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                if view.update(cx, |view, cx| view.reload(cx)).is_err() {
                    break;
                }
            }
        }));
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::ListTodo).color(Color::Muted))
    }

    fn tab_content(&self, params: TabContentParams, _window: &Window, _cx: &App) -> AnyElement {
        h_flex()
            .gap_1()
            .child(Indicator::dot().color(self.status_color()))
            .child(
                Label::new(self.tab_content_text(0, _cx)).color(if params.selected {
                    Color::Default
                } else {
                    Color::Muted
                }),
            )
            .into_any_element()
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        match self.plan.as_ref() {
            Some(plan) => format!("Plan — {}", plan.id).into(),
            None => "Plan".into(),
        }
    }

    fn tab_tooltip_text(&self, cx: &App) -> Option<SharedString> {
        Some(self.tab_content_text(0, cx))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Plan Opened")
    }
}

impl SerializableItem for PlanView {
    fn serialized_item_kind() -> &'static str {
        "PlanView"
    }

    fn cleanup(
        _workspace_id: workspace::WorkspaceId,
        _alive_items: Vec<workspace::ItemId>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::Task<Result<()>> {
        // No per-item payload to prune (the binding is re-derived on restore).
        gpui::Task::ready(Ok(()))
    }

    fn deserialize(
        _project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        _workspace_id: workspace::WorkspaceId,
        _item_id: workspace::ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Task<Result<Entity<Self>>> {
        // Rebuild a fresh tab; `added_to_workspace` re-binds it to the restored
        // active thread (F1.1 — the singleton follows the active thread anyway).
        window.spawn(cx, async move |cx| {
            cx.update(|_window, cx| {
                let plans_dir = workspace
                    .upgrade()
                    .map(|ws| following::plans_dir(ws.read(cx), cx))
                    .unwrap_or_else(|| PathBuf::from(".plans"));
                cx.new(|cx| PlanView {
                    focus_handle: cx.focus_handle(),
                    workspace,
                    plans_dir,
                    thread_session: None,
                    plan: None,
                    lens: Lens::Tasks,
                    _agent_subscription: None,
                    _watch_task: None,
                })
            })
        })
    }

    fn serialize(
        &mut self,
        _workspace: &mut Workspace,
        _item_id: workspace::ItemId,
        _closing: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<gpui::Task<Result<()>>> {
        // The workspace DB records that the tab exists; its binding is re-derived.
        Some(gpui::Task::ready(Ok(())))
    }

    fn should_serialize(&self, _event: &Self::Event) -> bool {
        false
    }
}

impl PlanView {
    fn render_header(&self, plan: &Plan, cx: &mut Context<Self>) -> impl IntoElement {
        let open_comments = plan
            .comments
            .iter()
            .filter(|comment| comment.state.as_deref() == Some("open"))
            .count();
        h_flex()
            .gap_2()
            .px_3()
            .py_1()
            .items_center()
            .child(Indicator::dot().color(self.status_color()))
            .child(Label::new(format!("Plan — {}", plan.id)).size(LabelSize::Large))
            .child(
                Label::new(format!("rev {}", plan.rev))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .child(
                h_flex()
                    .gap_0p5()
                    .child(self.lens_button(Lens::Spec, "Spec", cx))
                    .child(self.lens_button(Lens::Design, "Design", cx))
                    .child(self.lens_button(Lens::Tasks, "Tasks", cx)),
            )
            .when(open_comments > 0, |header| {
                header.child(
                    Button::new(
                        "send-for-revision",
                        format!("Send for revision · {open_comments}"),
                    )
                    .on_click(cx.listener(|this, _, _window, cx| this.send_for_revision(cx))),
                )
            })
    }

    fn lens_button(
        &self,
        lens: Lens,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(label, label)
            .toggle_state(self.lens == lens)
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.lens = lens;
                cx.notify();
            }))
    }

    fn render_tasks(&self, plan: &Plan, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_3()
            .children(plan.tasks.iter().map(|task| self.render_task_card(task, cx)))
    }

    /// Task card (compliance §7 / design-spec §3.5) with a flag affordance and its
    /// anchored comments. Structural first pass — fidelity gaps recorded for §13.
    fn render_task_card(&self, task: &Task, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        let border = if task.status == TaskStatus::InProgress {
            status.info
        } else if task.status == TaskStatus::Failed {
            status.deleted
        } else if task.gate {
            status.modified
        } else {
            colors.border_variant
        };
        let (glyph, glyph_color) = status_glyph(task);
        let done = task.status == TaskStatus::Done;
        let guarded = task.steps.iter().filter(|step| step.guard.is_some()).count();

        let comment_rows: Vec<_> = self
            .plan
            .as_ref()
            .map(|plan| {
                plan.comments
                    .iter()
                    .filter(|comment| {
                        comment.anchor.as_ref().and_then(|a| a.block.as_deref())
                            == Some(task.id.as_str())
                    })
                    .map(|comment| {
                        let outdated = comment.anchor.as_ref().is_some_and(|a| {
                            matches!(anchor::reanchor(a, plan), anchor::ReanchorResult::Outdated)
                        });
                        render_comment(comment, outdated)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let block = task.id.clone();
        let quote = task.title.clone().unwrap_or_default();

        v_flex()
            .gap_1()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(colors.panel_background)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .child(Label::new(glyph).color(glyph_color))
                    .child(
                        Label::new(task.id.clone())
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        Label::new(task.title.clone().unwrap_or_default())
                            .color(if done { Color::Muted } else { Color::Default }),
                    )
                    .when_some(task.ticket.clone(), |row, ticket| {
                        row.child(chip(ticket, colors.text_muted))
                    })
                    .when_some(task.system.clone(), |row, system| {
                        row.child(chip(system.to_uppercase(), system_color(Some(&system), cx)))
                    })
                    .when(guarded > 0, |row| {
                        row.child(chip(format!("⛨ {guarded} guarded"), status.modified))
                    })
                    .when_some(task.artifacts.sha.clone(), |row, sha| {
                        row.child(chip(
                            format!("⌥ {}", sha.chars().take(7).collect::<String>()),
                            status.created,
                        ))
                    })
                    .child(
                        Button::new(SharedString::from(format!("flag-{block}")), "⚑").on_click(
                            cx.listener(move |this, _, _window, cx| {
                                this.add_flag(&block, &quote, cx)
                            }),
                        ),
                    ),
            )
            .child(
                v_flex().pl_4().gap_0p5().children(
                    task.steps
                        .iter()
                        .enumerate()
                        .map(|(index, step)| render_step(index, step, cx)),
                ),
            )
            .children(comment_rows)
    }

    /// Add a blocker flag anchored to a block, writing plan.json via plan_core.
    fn add_flag(&mut self, block: &str, quote: &str, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        let comment_id = format!("c{}", fresh.comments.len() + 1);
        let anchor = Anchor {
            lens: Some("tasks".to_string()),
            block: Some(block.to_string()),
            range: None,
            quote: Some(quote.to_string()),
            code_refs: vec![],
            extra: Default::default(),
        };
        comments::add_comment(
            &mut fresh,
            &comment_id,
            "flag",
            "user",
            Some("blocker"),
            anchor,
            "Flagged for review",
        );
        if store::save(&self.plans_dir, &fresh).is_ok() {
            self.reload(cx);
        }
    }

    /// Mark all open comments sent for revision (F3.4), writing plan.json.
    fn send_for_revision(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if comments::mark_sent_batch(&mut fresh) > 0
            && store::save(&self.plans_dir, &fresh).is_ok()
        {
            self.reload(cx);
        }
    }
}

/// Render one anchored comment (severity glyph + text + state + outdated badge).
fn render_comment(comment: &Comment, outdated: bool) -> impl IntoElement {
    let (glyph, color) = match comment.severity.as_deref() {
        Some("blocker") => ("⚑", Color::Error),
        Some("concern") => ("⚠", Color::Warning),
        _ => ("💡", Color::Muted),
    };
    let text = comment
        .thread
        .first()
        .and_then(|message| message.text.clone())
        .unwrap_or_default();
    let state = comment.state.clone().unwrap_or_default();
    h_flex()
        .pl_4()
        .gap_2()
        .items_center()
        .child(Label::new(glyph).color(color).size(LabelSize::Small))
        .child(Label::new(text).size(LabelSize::Small))
        .child(Label::new(state).size(LabelSize::Small).color(Color::Muted))
        .when(outdated, |row| {
            row.child(
                Label::new("outdated")
                    .size(LabelSize::Small)
                    .color(Color::Warning),
            )
        })
}

fn render_step(index: usize, step: &Step, cx: &App) -> impl IntoElement {
    let guard_chip = step.guard.as_ref().map(|guard| {
        let (glyph, color) = match guard.guard_type.as_deref() {
            Some("input") => (
                "✋ input",
                cx.theme()
                    .syntax()
                    .style_for_name("type")
                    .and_then(|style| style.color)
                    .unwrap_or(cx.theme().colors().text_accent),
            ),
            _ => ("⛨ approve", cx.theme().status().modified),
        };
        chip(glyph, color)
    });

    h_flex()
        .gap_2()
        .items_start()
        .child(
            Label::new(format!("{}.", index + 1))
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
        .child(Label::new(step.text.clone().unwrap_or_default()).size(LabelSize::Small))
        .children(guard_chip)
}

/// The checkbox glyph + color for a task's state (compliance §7 vocabulary).
fn status_glyph(task: &Task) -> (&'static str, Color) {
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

/// System-badge color (design-spec §3.5): Backend purple / Frontend teal /
/// Testing green / GATE amber. Purple/teal have no `ThemeColors` field, so they
/// map to the syntax keyword/type highlight colors (recorded mapping decision).
fn system_color(system: Option<&str>, cx: &App) -> Hsla {
    let theme = cx.theme();
    match system.map(str::to_ascii_lowercase).as_deref() {
        Some("backend") => theme
            .syntax()
            .style_for_name("keyword")
            .and_then(|style| style.color)
            .unwrap_or(theme.colors().text_accent),
        Some("frontend") => theme
            .syntax()
            .style_for_name("type")
            .and_then(|style| style.color)
            .unwrap_or(theme.colors().text_accent),
        Some("testing") => theme.status().created,
        Some("gate") => theme.status().modified,
        _ => theme.colors().text_muted,
    }
}

/// A small bordered pill whose text inherits the given color.
fn chip(text: impl Into<SharedString>, color: Hsla) -> impl IntoElement {
    div()
        .px_1()
        .rounded_sm()
        .border_1()
        .border_color(color)
        .text_color(color)
        .text_size(px(10.))
        .child(text.into())
}

/// The default lens for a status (F12.1: default lens follows status).
fn default_lens(status: &Status) -> Lens {
    match status {
        Status::Executing | Status::Paused | Status::Gate | Status::Amending => Lens::Tasks,
        _ => Lens::Spec,
    }
}

fn section(title: &'static str) -> impl IntoElement {
    Label::new(title).size(LabelSize::Small).color(Color::Muted)
}

fn bullet(text: &str) -> impl IntoElement {
    Label::new(format!("• {text}")).size(LabelSize::Small)
}

fn render_spec(plan: &Plan) -> impl IntoElement {
    let spec = &plan.spec;
    v_flex()
        .p_3()
        .gap_2()
        .child(section("GOAL"))
        .child(Label::new(spec.goal.clone()).size(LabelSize::Small))
        .when(!spec.scope.r#in.is_empty(), |column| {
            column
                .child(section("IN SCOPE"))
                .children(spec.scope.r#in.iter().map(|item| bullet(item)))
        })
        .when(!spec.scope.out.is_empty(), |column| {
            column
                .child(section("OUT OF SCOPE"))
                .children(spec.scope.out.iter().map(|item| bullet(item)))
        })
        .when(!spec.acceptance.is_empty(), |column| {
            column.child(section("ACCEPTANCE")).children(spec.acceptance.iter().map(
                |acceptance| {
                    Label::new(format!(
                        "when {} — shall {}",
                        acceptance.when.as_deref().unwrap_or(""),
                        acceptance.shall.as_deref().unwrap_or("")
                    ))
                    .size(LabelSize::Small)
                },
            ))
        })
}

fn render_design(plan: &Plan) -> impl IntoElement {
    let design = &plan.design;
    v_flex()
        .p_3()
        .gap_2()
        .when(!design.contracts.is_empty(), |column| {
            column.child(section("CONTRACTS")).children(
                design
                    .contracts
                    .iter()
                    .map(|contract| Label::new(contract.id.clone()).size(LabelSize::Small)),
            )
        })
        .when(!design.decisions.is_empty(), |column| {
            column.child(section("DECISIONS")).children(design.decisions.iter().map(|decision| {
                Label::new(format!(
                    "{} — {}",
                    decision.text.as_deref().unwrap_or(""),
                    decision.rationale.as_deref().unwrap_or("")
                ))
                .size(LabelSize::Small)
            }))
        })
        .when(!design.risks.is_empty(), |column| {
            column
                .child(section("RISKS"))
                .children(design.risks.iter().map(|risk| bullet(risk)))
        })
}

impl Render for PlanView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor_bg = cx.theme().colors().editor_background;
        let body = match &self.plan {
            None => v_flex()
                .items_center()
                .justify_center()
                .gap_1()
                .size_full()
                .child(Label::new("Plan").size(LabelSize::Large))
                .child(Label::new("no plan — ask the agent to draft one").color(Color::Muted))
                .into_any_element(),
            Some(plan) => v_flex()
                .size_full()
                .child(self.render_header(plan, cx))
                .child(match self.lens {
                    Lens::Tasks => self.render_tasks(plan, cx).into_any_element(),
                    Lens::Spec => render_spec(plan).into_any_element(),
                    Lens::Design => render_design(plan).into_any_element(),
                })
                .into_any_element(),
        };

        v_flex()
            .size_full()
            .bg(editor_bg)
            .track_focus(&self.focus_handle)
            .child(body)
    }
}
