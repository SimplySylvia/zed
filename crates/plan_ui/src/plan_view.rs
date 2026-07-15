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
use plan_core::{
    Anchor, Comment, HistoryEntry, Hunk, Plan, Status, Step, Task, TaskStatus, anchor, comments,
    exec, lint, rev, store,
};
use ui::prelude::*;
use ui::{Button, Indicator, Tooltip};
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
        Self::open_tab(workspace, window, cx);
    }

    /// Open (or re-activate) the singleton Plan tab in the active pane. Reused by
    /// the `OpenPlan` action and the panel's "Open as tab" button (F1.3).
    pub fn open_tab(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
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
        let Some(plan) = self.plan.as_ref() else {
            return Color::Muted;
        };
        // A guard/gate hold is a needs-you state regardless of lifecycle status (§9).
        if exec::current_hold(plan).is_some() {
            return Color::Modified;
        }
        match plan.status {
            Status::Executing => Color::Accent,
            Status::Approved | Status::Done => Color::Created,
            Status::InReview | Status::Revising | Status::Gate | Status::Amending => Color::Modified,
            Status::Drafting => Color::Placeholder,
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

    fn tab_content(&self, params: TabContentParams, _window: &Window, cx: &App) -> AnyElement {
        let dot = match self.plan.as_ref() {
            Some(plan) => crate::status_dot(&plan.status, self.status_color(), "plan-tab-dot"),
            None => Indicator::dot().color(self.status_color()).into_any_element(),
        };
        h_flex()
            .gap_1()
            .child(dot)
            .child(
                Label::new(self.tab_content_text(0, cx)).color(if params.selected {
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
    /// Plan toolbar (compliance §3 / design-spec §3.1): status pill · rev · lens
    /// switcher · blocker chip (when > 0) · spacer · contextual · primary.
    fn render_header(&self, plan: &Plan, cx: &mut Context<Self>) -> impl IntoElement {
        let open_comments = plan
            .comments
            .iter()
            .filter(|comment| comment.state.as_deref() == Some("open"))
            .count();
        let blockers = open_blocker_count(plan);
        let deleted = cx.theme().status().deleted;
        h_flex()
            .gap_2()
            .px_3()
            .py_1()
            .items_center()
            .child(crate::status_dot(&plan.status, self.status_color(), "plan-header-dot"))
            .child(Label::new(format!("Plan — {}", plan.id)).size(LabelSize::Large))
            .child(
                Label::new(format!("rev {}", plan.rev))
                    .buffer_font(cx)
                    .size(LabelSize::XSmall)
                    .color(Color::Placeholder),
            )
            .child(
                h_flex()
                    .gap_0p5()
                    .child(self.lens_button(Lens::Spec, "Spec", cx))
                    .child(self.lens_button(Lens::Design, "Design", cx))
                    .child(self.lens_button(Lens::Tasks, "Tasks", cx)),
            )
            .when(blockers > 0, |header| {
                header.child(chip(
                    format!("⚑ {blockers} blocker{}", if blockers == 1 { "" } else { "s" }),
                    deleted,
                ))
            })
            .child(div().flex_1())
            .child(
                Button::new("run-lint", "Lint")
                    .on_click(cx.listener(|this, _, _window, cx| this.run_lint(cx))),
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
            .child(self.render_primary(plan, blockers, cx))
    }

    /// The state-driven primary action (design-spec §9 matrix). In 5b: `Apply all`
    /// while a revision is staged; `Approve` in review states (disabled with a G5
    /// tooltip while blockers are open). Launch/Pause/gate primaries are M6.
    fn render_primary(&self, plan: &Plan, blockers: usize, cx: &Context<Self>) -> AnyElement {
        if plan.pending_revision.is_some() {
            return Button::new("apply-all", "Apply all")
                .on_click(cx.listener(|this, _, _window, cx| this.apply_all(cx)))
                .into_any_element();
        }
        if plan.status == Status::Approved {
            // ▶ Launch (§9 matrix). Rehearsal-mismatch gating is v1 (F9.4).
            return Button::new("launch", "▶ Launch")
                .on_click(cx.listener(|this, _, _window, cx| this.launch(cx)))
                .into_any_element();
        }
        if matches!(
            plan.status,
            Status::Drafting | Status::InReview | Status::Revising
        ) {
            let enabled = approve_enabled(plan);
            let mut approve = Button::new("approve", "Approve")
                .disabled(!enabled)
                .on_click(cx.listener(|this, _, _window, cx| this.approve(cx)));
            if !enabled {
                approve = approve.tooltip(Tooltip::text(format!(
                    "{blockers} blocker{} open",
                    if blockers == 1 { "" } else { "s" }
                )));
            }
            return approve.into_any_element();
        }
        div().into_any_element()
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
        // Active-task spotlight (F4.3): the in-progress card is info-tinted + bordered.
        let active = task.status == TaskStatus::InProgress;
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
                        self.render_comment(comment, outdated, cx)
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
            .bg(if active {
                status.info.opacity(0.08)
            } else {
                colors.panel_background
            })
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
                        row.child(crate::mono_chip(
                            format!("⌥ {}", sha.chars().take(7).collect::<String>()),
                            status.created,
                            cx,
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
            .children(self.guard_controls(task, cx))
    }

    /// Clear affordances for any holding guard on this task (F4.5b): ⛨ Approve to
    /// run / ✋ Record & continue. The ✋ free-text field is deferred (fidelity).
    fn guard_controls(&self, task: &Task, cx: &Context<Self>) -> Vec<AnyElement> {
        let modified = cx.theme().status().modified;
        task.steps
            .iter()
            .filter(|step| {
                step.guard.as_ref().and_then(|guard| guard.state.as_deref()) == Some("holding")
            })
            .filter_map(|step| {
                let guard = step.guard.as_ref()?;
                let is_input = guard.guard_type.as_deref() == Some("input");
                let prompt = guard.prompt.clone().unwrap_or_default();
                let label = if is_input {
                    "✋ Record & continue"
                } else {
                    "⛨ Approve to run"
                };
                let task_id = task.id.clone();
                let step_id = step.id.clone();
                let button_id = SharedString::from(format!("clear-guard-{task_id}-{step_id}"));
                Some(
                    v_flex()
                        .ml_4()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(modified)
                        .child(Label::new(prompt).size(LabelSize::Small))
                        .child(Button::new(button_id, label).on_click(cx.listener(
                            move |this, _, _window, cx| this.clear_step_guard(&task_id, &step_id, cx),
                        )))
                        .into_any_element(),
                )
            })
            .collect()
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

    /// Apply/reject a staged hunk, then resolve (F9.3). `resolve_revision` is a
    /// no-op until every hunk is decided, then commits once (bumps rev, stamps
    /// provenance) — so this is safe to call after each per-hunk action.
    fn apply_hunk(&mut self, hunk_id: &str, cx: &mut Context<Self>) {
        self.mutate_pending(cx, |plan| {
            rev::apply_hunk(plan, hunk_id);
        });
    }

    fn reject_hunk(&mut self, hunk_id: &str, cx: &mut Context<Self>) {
        self.mutate_pending(cx, |plan| {
            rev::reject_hunk(plan, hunk_id);
        });
    }

    fn apply_all(&mut self, cx: &mut Context<Self>) {
        self.mutate_pending(cx, |plan| {
            rev::apply_all(plan);
        });
    }

    fn mutate_pending(&mut self, cx: &mut Context<Self>, mutate: impl FnOnce(&mut Plan)) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        mutate(&mut fresh);
        rev::resolve_revision(&mut fresh);
        if store::save(&self.plans_dir, &fresh).is_ok() {
            self.reload(cx);
        }
    }

    /// Resolve a comment (F3.4d: resolution belongs to the user); clears it from
    /// the Approve gate.
    fn resolve_comment(&mut self, comment_id: &str, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if comments::set_comment_state(&mut fresh, comment_id, "resolved")
            && store::save(&self.plans_dir, &fresh).is_ok()
        {
            self.reload(cx);
        }
    }

    /// Run policy lint from the UI (F9.1): reconcile findings into plan-lint
    /// comments, writing plan.json directly. On-demand (not per-poll) to avoid
    /// rev-churn; the agent runs it via the `plan_lint` tool.
    fn run_lint(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        let policy = lint::Policy::load(&self.plans_dir);
        let findings = lint::lint(&fresh, &policy, self.plans_dir.parent());
        if lint::reconcile(&mut fresh, &findings) && store::save(&self.plans_dir, &fresh).is_ok() {
            self.reload(cx);
        }
    }

    /// Clear a held step guard (F4.5b), writing plan.json via plan_core. The ✋ input
    /// free-text field is deferred (fidelity); the button records the clear + evidence
    /// + receipt with no typed value for now.
    fn clear_step_guard(&mut self, task: &str, step: &str, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if exec::clear_guard(&mut fresh, task, step, None).is_ok()
            && store::save(&self.plans_dir, &fresh).is_ok()
        {
            self.reload(cx);
        }
    }

    /// Approve a GATE task (F4.5) so execution proceeds.
    fn approve_gate_task(&mut self, task: &str, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if exec::approve_gate(&mut fresh, task).is_ok()
            && store::save(&self.plans_dir, &fresh).is_ok()
        {
            self.reload(cx);
        }
    }

    /// Launch the plan (F4.1): approved → executing + take the lease, via plan_core
    /// (the UI writes plan.json directly). Uses the followed thread's session id as
    /// the lease holder, falling back to the plan's owning thread. Git branch setup
    /// (F10.2) is M7.
    fn launch(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let thread = self
            .thread_session
            .clone()
            .unwrap_or_else(|| plan.thread.clone());
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if exec::launch(&mut fresh, &thread).is_ok() && store::save(&self.plans_dir, &fresh).is_ok()
        {
            self.reload(cx);
        }
    }

    /// Approve the plan (F3.6). Guarded to zero open blockers even though the
    /// button is disabled, since the UI writes plan.json directly. Mirrors the
    /// server's rev+history stamping.
    fn approve(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if open_blocker_count(&fresh) > 0 {
            return;
        }
        fresh.status = Status::Approved;
        fresh.rev += 1;
        fresh.history.push(HistoryEntry {
            rev: Some(fresh.rev),
            by: Some("user".to_string()),
            kind: Some("status".to_string()),
            summary: Some("approved".to_string()),
            at: None,
            extra: Default::default(),
        });
        if store::save(&self.plans_dir, &fresh).is_ok() {
            self.reload(cx);
        }
    }

    /// Staged-revision change card (compliance §6, design-spec §3.4): info header
    /// band + "plan unchanged until applied"; one row per hunk. Rendered as GPUI
    /// chrome for 5b — real editor-diff mini-buffers are a fidelity-pass upgrade.
    /// Gate evidence card (F4.5) when execution holds at a GATE task: header +
    /// ✓ Approve gate. Compliance §6 card chassis.
    fn render_gate_hold(&self, plan: &Plan, cx: &Context<Self>) -> Option<AnyElement> {
        let hold = exec::current_hold(plan)?;
        if hold.kind != "gate" {
            return None;
        }
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        let title = plan
            .tasks
            .iter()
            .find(|task| task.id == hold.task)
            .and_then(|task| task.title.clone())
            .unwrap_or_else(|| hold.task.clone());
        let task_id = hold.task;
        Some(
            v_flex()
                .mx_3()
                .mt_2()
                .rounded_md()
                .border_1()
                .border_color(colors.border)
                .bg(colors.panel_background)
                .child(
                    h_flex()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .items_center()
                        .bg(status.modified.opacity(0.15))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(status.modified)
                                .child("GATE — NEEDS SIGN-OFF"),
                        )
                        .child(Label::new(title).size(LabelSize::Small).color(Color::Muted)),
                )
                .child(
                    h_flex().px_2().py_1().child(
                        Button::new("approve-gate", "✓ Approve gate").on_click(cx.listener(
                            move |this, _, _window, cx| this.approve_gate_task(&task_id, cx),
                        )),
                    ),
                )
                .into_any_element(),
        )
    }

    fn render_staged_revision(&self, plan: &Plan, cx: &Context<Self>) -> Option<AnyElement> {
        let pending = plan.pending_revision.as_ref()?;
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        let rev_note = pending
            .rev
            .map(|rev| format!("rev {rev}"))
            .unwrap_or_default();
        Some(
            v_flex()
                .mx_3()
                .mt_2()
                .rounded_md()
                .border_1()
                .border_color(colors.border)
                .bg(colors.panel_background)
                .child(
                    h_flex()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .items_center()
                        .bg(status.info_background)
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(status.info)
                                .child("STAGED REVISION"),
                        )
                        .child(
                            Label::new("plan unchanged until applied")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(div().flex_1())
                        .child(
                            Label::new(rev_note)
                                .buffer_font(cx)
                                .size(LabelSize::XSmall)
                                .color(Color::Placeholder),
                        ),
                )
                .children(pending.hunks.iter().map(|hunk| self.render_hunk(hunk, cx)))
                .into_any_element(),
        )
    }

    /// One staged hunk: target + provenance chip + old (struck, deleted-bg) / new
    /// (created-bg) lines + Apply/Reject (or the resolved state).
    fn render_hunk(&self, hunk: &Hunk, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        let id = hunk.id.clone();
        let old = hunk.old.clone().unwrap_or_default();
        let new = hunk.new.clone().unwrap_or_default();
        let controls = match hunk.state.as_deref() {
            Some("applied") => Label::new("✓ Applied")
                .size(LabelSize::Small)
                .color(Color::Created)
                .into_any_element(),
            Some("rejected") => Label::new("Rejected")
                .size(LabelSize::Small)
                .color(Color::Muted)
                .into_any_element(),
            _ => h_flex()
                .gap_1()
                .child(
                    Button::new(SharedString::from(format!("apply-{id}")), "Apply").on_click(
                        cx.listener({
                            let id = id.clone();
                            move |this, _, _window, cx| this.apply_hunk(&id, cx)
                        }),
                    ),
                )
                .child(
                    Button::new(SharedString::from(format!("reject-{id}")), "Reject").on_click(
                        cx.listener(move |this, _, _window, cx| this.reject_hunk(&id, cx)),
                    ),
                )
                .into_any_element(),
        };

        v_flex()
            .gap_0p5()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(colors.border_variant)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Label::new(hunk.target.clone().unwrap_or_default())
                            .buffer_font(cx)
                            .size(LabelSize::XSmall)
                            .color(Color::Placeholder),
                    )
                    .when_some(hunk.from.clone(), |row, from| {
                        row.child(crate::mono_chip(format!("from {from}"), status.info, cx))
                    })
                    .child(div().flex_1())
                    .child(controls),
            )
            .when(!old.is_empty(), |row| {
                row.child(
                    div()
                        .px_1()
                        .line_through()
                        .bg(status.deleted_background)
                        .text_color(status.deleted)
                        .text_size(px(12.))
                        .child(SharedString::from(old)),
                )
            })
            .when(!new.is_empty(), |row| {
                row.child(
                    div()
                        .px_1()
                        .bg(status.created_background)
                        .text_color(status.created)
                        .text_size(px(12.))
                        .child(SharedString::from(new)),
                )
            })
    }
}

impl PlanView {
    /// Render one anchored comment (severity glyph + text + state + outdated badge).
    /// Open blockers carry a `✓ resolve` affordance (F3.4d) so the Approve gate is
    /// clearable in the tab.
    fn render_comment(
        &self,
        comment: &Comment,
        outdated: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
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
        let is_lint = comment.author.as_deref() == Some("plan-lint");
        // Lint blockers clear by fixing the issue and re-linting, not by manual
        // resolve — so the ✓ resolve affordance is offered only on user/agent flags.
        let resolvable =
            comment.severity.as_deref() == Some("blocker") && state != "resolved" && !is_lint;
        let rule_id = if is_lint { lint_rule_id(comment) } else { None };
        let block = comment.anchor.as_ref().and_then(|anchor| anchor.block.clone());
        let comment_id = comment.id.clone();
        let muted = cx.theme().colors().text_muted;
        let state_color = comment_state_color(&state).color(cx);
        h_flex()
            .pl_4()
            .gap_2()
            .items_center()
            .child(Label::new(glyph).color(color).size(LabelSize::Small))
            .child(Label::new(text).size(LabelSize::Small))
            .child(chip(state, state_color))
            .when(!is_lint, |row| {
                row.when_some(block, |row, block| {
                    row.child(crate::mono_chip(format!("{comment_id} ↪ {block}"), muted, cx))
                })
            })
            .when_some(rule_id, |row, rule_id| {
                row.child(crate::mono_chip(rule_id, muted, cx))
            })
            .when(outdated, |row| {
                row.child(
                    Label::new("outdated")
                        .size(LabelSize::Small)
                        .color(Color::Warning),
                )
            })
            .when(resolvable, |row| {
                row.child(
                    Button::new(SharedString::from(format!("resolve-{comment_id}")), "✓ resolve")
                        .on_click(cx.listener({
                            let comment_id = comment_id.clone();
                            move |this, _, _window, cx| this.resolve_comment(&comment_id, cx)
                        })),
                )
            })
    }
}

fn render_step(index: usize, step: &Step, cx: &App) -> impl IntoElement {
    let guard_chip = step.guard.as_ref().map(|guard| {
        let is_input = guard.guard_type.as_deref() == Some("input");
        let (glyph, color) = if is_input {
            (
                "✋ input",
                cx.theme()
                    .syntax()
                    .style_for_name("type")
                    .and_then(|style| style.color)
                    .unwrap_or(cx.theme().colors().text_accent),
            )
        } else {
            ("⛨ approve", cx.theme().status().modified)
        };
        match guard.state.as_deref() {
            // Cleared guard → mono success receipt (F4.5b).
            Some("cleared") => crate::mono_chip(
                format!("{} cleared", if is_input { "✋" } else { "⛨" }),
                cx.theme().status().created,
                cx,
            )
            .into_any_element(),
            // Holding → the badge pulses (needs you now).
            Some("holding") => crate::pulse(chip(glyph, color), "guard-hold-pulse"),
            _ => chip(glyph, color).into_any_element(),
        }
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

/// The rule id encoded in a plan-lint comment's deterministic id
/// (`lint-<rule>-<block>`), for display (compliance §6 mono rule id).
fn lint_rule_id(comment: &Comment) -> Option<String> {
    let stripped = comment.id.strip_prefix("lint-")?;
    match comment.anchor.as_ref().and_then(|anchor| anchor.block.as_deref()) {
        Some(block) => stripped
            .strip_suffix(&format!("-{block}"))
            .map(str::to_string),
        None => Some(stripped.to_string()),
    }
}

/// Comment thread state → chip color (compliance §9: open amber / resolved green;
/// addressed = agent acted, awaiting the user).
fn comment_state_color(state: &str) -> Color {
    match state {
        "resolved" => Color::Created,
        "addressed" => Color::Info,
        "open" | "sent" | "reopened" => Color::Modified,
        _ => Color::Muted,
    }
}

/// Comments that gate Approve (F3.6): blocker-severity and not yet resolved
/// (F3.4d — resolution belongs to the user).
fn open_blocker_count(plan: &Plan) -> usize {
    plan.comments
        .iter()
        .filter(|comment| {
            comment.severity.as_deref() == Some("blocker")
                && comment.state.as_deref() != Some("resolved")
        })
        .count()
}

/// Approve is enabled only at zero open blockers (F3.6).
fn approve_enabled(plan: &Plan) -> bool {
    open_blocker_count(plan) == 0
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
                .children(self.render_gate_hold(plan, cx))
                .children(self.render_staged_revision(plan, cx))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(value: serde_json::Value) -> Plan {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn open_blocker_gates_approve() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "in_review", "rev": 1,
            "thread": "a", "spec": { "goal": "g" },
            "comments": [
                { "id": "c1", "severity": "blocker", "state": "open" },
                { "id": "c2", "severity": "concern", "state": "open" }
            ]
        }));
        assert_eq!(open_blocker_count(&plan), 1);
        assert!(!approve_enabled(&plan));
    }

    #[test]
    fn resolved_blocker_enables_approve() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "in_review", "rev": 1,
            "thread": "a", "spec": { "goal": "g" },
            "comments": [{ "id": "c1", "severity": "blocker", "state": "resolved" }]
        }));
        assert_eq!(open_blocker_count(&plan), 0);
        assert!(approve_enabled(&plan));
    }

    #[test]
    fn lint_rule_id_parses_the_rule_from_the_comment_id() {
        let comment: Comment = serde_json::from_value(serde_json::json!({
            "id": "lint-max-files-per-task-t1", "author": "plan-lint",
            "anchor": { "block": "t1" }
        }))
        .unwrap();
        assert_eq!(lint_rule_id(&comment).as_deref(), Some("max-files-per-task"));
    }

    #[test]
    fn no_blockers_enables_approve() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "in_review", "rev": 1,
            "thread": "a", "spec": { "goal": "g" }
        }));
        assert!(approve_enabled(&plan));
    }
}
