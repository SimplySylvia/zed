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
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Hsla, Div,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, WeakEntity, Window,
    actions, px,
};
use plan_core::{
    Anchor, Comment, HistoryEntry, Hunk, Plan, Status, Step, Task, TaskStatus, anchor, comments,
    exec, lint, rev, store,
};
use ui::prelude::*;
use ui::{Button, ButtonStyle, CommonAnimationExt, Indicator, TintColor, Tooltip};
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

    /// Subscribe to the agent panel's active-thread changes and retarget now.
    fn subscribe_agent_panel(&mut self, agent_panel: Entity<AgentPanel>, cx: &mut Context<Self>) {
        self._agent_subscription = Some(cx.subscribe(&agent_panel, |this, panel, event, cx| {
            if matches!(event, AgentPanelEvent::ActiveViewChanged) {
                this.retarget(panel, cx);
            }
        }));
        self.retarget(agent_panel, cx);
    }

    /// One poll tick: bind to the agent panel if it has appeared since startup
    /// (so a tab restored before the panel existed still follows threads), then
    /// re-read the plan from disk.
    fn poll_tick(&mut self, cx: &mut Context<Self>) {
        if self._agent_subscription.is_none() {
            let agent_panel = self
                .workspace
                .upgrade()
                .and_then(|workspace| workspace.read(cx).panel::<AgentPanel>(cx));
            if let Some(agent_panel) = agent_panel {
                self.subscribe_agent_panel(agent_panel, cx);
                return;
            }
        }
        self.reload(cx);
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
        // The AgentPanel may not be registered yet on first launch / when a
        // serialized tab restores. Bind to it if present; otherwise resolve via the
        // sole-plan fallback now and bind late from the poll — never leave the tab
        // empty with no poll running.
        match workspace.panel::<AgentPanel>(cx) {
            Some(agent_panel) => self.subscribe_agent_panel(agent_panel, cx),
            None => self.reload(cx),
        }
        self._watch_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                if view.update(cx, |view, cx| view.poll_tick(cx)).is_err() {
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
            .when(
                matches!(plan.status, Status::Executing | Status::Paused),
                |header| {
                    header.child(
                        Button::new("stop", "⏹ Stop")
                            .on_click(cx.listener(|this, _, _window, cx| this.stop_plan(cx))),
                    )
                },
            )
            .child(self.render_primary(plan, blockers, cx))
    }

    /// The state-driven primary action (design-spec §9 matrix). In 5b: `Apply all`
    /// while a revision is staged; `Approve` in review states (disabled with a G5
    /// tooltip while blockers are open). Launch/Pause/gate primaries are M6.
    fn render_primary(&self, plan: &Plan, blockers: usize, cx: &Context<Self>) -> AnyElement {
        if plan.pending_revision.is_some() {
            return primary_button("apply-all", "Apply all")
                .on_click(cx.listener(|this, _, _window, cx| this.apply_all(cx)))
                .into_any_element();
        }
        if plan.status == Status::Approved {
            // ▶ Launch (§9 matrix). Rehearsal-mismatch gating is v1 (F9.4).
            return primary_button("launch", "▶ Launch")
                .on_click(cx.listener(|this, _, _window, cx| this.launch(cx)))
                .into_any_element();
        }
        if plan.status == Status::Executing {
            return primary_button("pause", "⏸ Pause")
                .on_click(cx.listener(|this, _, _window, cx| this.pause_plan(cx)))
                .into_any_element();
        }
        if plan.status == Status::Paused {
            return primary_button("resume", "▶ Resume")
                .on_click(cx.listener(|this, _, _window, cx| this.resume_plan(cx)))
                .into_any_element();
        }
        if matches!(
            plan.status,
            Status::Drafting | Status::InReview | Status::Revising
        ) {
            let enabled = approve_enabled(plan);
            let mut approve = primary_button("approve", "Approve")
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
            .child(sechead("TASKS", cx))
            .children(
                plan.tasks
                    .iter()
                    .enumerate()
                    .map(|(index, task)| self.render_task_card(index, task, cx)),
            )
    }

    /// Task card (compliance §7 / design-spec §3.5) with a flag affordance and its
    /// anchored comments. Structural first pass — fidelity gaps recorded for §13.
    fn render_task_card(&self, index: usize, task: &Task, cx: &Context<Self>) -> impl IntoElement {
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
        let done = task.status == TaskStatus::Done;
        // Active-task spotlight (F4.3): the in-progress card is info-tinted + bordered.
        let active = task.status == TaskStatus::InProgress;
        let guarded = task.steps.iter().filter(|step| step.guard.is_some()).count();
        let title = task.title.clone().unwrap_or_default();

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
                    .child(render_checkbox(task, cx))
                    .child(
                        Label::new(format!("{}", index + 1))
                            .buffer_font(cx)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(if done {
                        div()
                            .line_through()
                            .text_color(colors.text_muted)
                            .child(title)
                            .into_any_element()
                    } else {
                        Label::new(title).into_any_element()
                    })
                    // Chips in mockup order: ticket · system · guard · sha · tests.
                    .when_some(task.ticket.clone(), |row, ticket| {
                        row.child(crate::mono_chip(ticket, colors.text_accent, cx))
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
                    .when(task.artifacts.tests.is_some(), |row| {
                        row.child(chip("✓ tests", status.created))
                    })
                    .child(div().flex_1())
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
            .children(task_timeline(task, cx))
            .children(comment_rows)
            .children(self.guard_controls(task, cx))
            .children(self.recovery_controls(task, cx))
    }

    /// Recovery affordance for an interrupted task (F11.1): Resume / Redo / Keep
    /// manual.
    fn recovery_controls(&self, task: &Task, cx: &Context<Self>) -> Vec<AnyElement> {
        if task.status != TaskStatus::Interrupted {
            return Vec::new();
        }
        let deleted = cx.theme().status().deleted;
        let choices = [("Resume", "resume"), ("Redo", "redo"), ("Keep manual", "manual")];
        let buttons = choices.map(|(label, choice)| {
            let task_id = task.id.clone();
            Button::new(
                SharedString::from(format!("recover-{choice}-{}", task.id)),
                label,
            )
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.recover_task(&task_id, choice, cx)
            }))
        });
        vec![
            h_flex()
                .ml_4()
                .gap_2()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(deleted)
                .child(
                    Label::new("interrupted — recover:")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .children(buttons)
                .into_any_element(),
        ]
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
                        .child(primary_button(button_id, label).on_click(cx.listener(
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

    /// Load the plan fresh, apply `mutate`, save, and reload — the shared write path
    /// for control/recovery actions (the UI writes plan.json directly).
    fn mutate_plan(&mut self, cx: &mut Context<Self>, mutate: impl FnOnce(&mut Plan) -> Result<()>) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let id = plan.id.clone();
        let Ok(mut fresh) = store::load(&self.plans_dir, &id) else {
            return;
        };
        if mutate(&mut fresh).is_ok() && store::save(&self.plans_dir, &fresh).is_ok() {
            self.reload(cx);
        }
    }

    /// Pause / resume / stop execution (F5.1/F5.6).
    fn pause_plan(&mut self, cx: &mut Context<Self>) {
        self.mutate_plan(cx, exec::pause);
    }

    fn resume_plan(&mut self, cx: &mut Context<Self>) {
        self.mutate_plan(cx, exec::resume);
    }

    fn stop_plan(&mut self, cx: &mut Context<Self>) {
        self.mutate_plan(cx, exec::stop);
    }

    /// Recover an interrupted task or take a failure-ladder decision (F11.1/F11.4):
    /// choice ∈ resume | redo | manual.
    fn recover_task(&mut self, task: &str, choice: &'static str, cx: &mut Context<Self>) {
        let task = task.to_string();
        self.mutate_plan(cx, move |plan| exec::recover_task(plan, &task, choice));
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
    /// Failure-ladder escalation card (F11.4): shown when a task has ≥2 amendments.
    /// Offers Take over manually / Retry (guide-me is agent-side, deferred).
    fn render_escalation(&self, plan: &Plan, cx: &Context<Self>) -> Option<AnyElement> {
        let task = plan
            .tasks
            .iter()
            .find(|task| exec::needs_escalation(plan, &task.id))?;
        let status = cx.theme().status();
        let title: SharedString = task.title.clone().unwrap_or_else(|| task.id.clone()).into();
        let manual_id = task.id.clone();
        let redo_id = task.id.clone();
        Some(
            card_shell(
                "ESCALATION — 2 AMENDMENTS",
                status.deleted,
                status.deleted_background,
                Some(title),
                None,
                cx,
            )
            .child(
                card_row(cx)
                    .child(primary_button("escalate-manual", "Take over manually").on_click(
                        cx.listener(move |this, _, _window, cx| {
                            this.recover_task(&manual_id, "manual", cx)
                        }),
                    ))
                    .child(Button::new("escalate-redo", "Retry").on_click(cx.listener(
                        move |this, _, _window, cx| this.recover_task(&redo_id, "redo", cx),
                    ))),
            )
            .into_any_element(),
        )
    }

    fn render_gate_hold(&self, plan: &Plan, cx: &Context<Self>) -> Option<AnyElement> {
        let hold = exec::current_hold(plan)?;
        if hold.kind != "gate" {
            return None;
        }
        let status = cx.theme().status();
        let title: SharedString = plan
            .tasks
            .iter()
            .find(|task| task.id == hold.task)
            .and_then(|task| task.title.clone())
            .unwrap_or_else(|| hold.task.clone())
            .into();
        let task_id = hold.task;
        Some(
            card_shell(
                "GATE — NEEDS SIGN-OFF",
                status.modified,
                status.modified_background,
                Some(title),
                None,
                cx,
            )
            .child(
                card_row(cx).child(
                    primary_button("approve-gate", "✓ Approve gate").on_click(cx.listener(
                        move |this, _, _window, cx| this.approve_gate_task(&task_id, cx),
                    )),
                ),
            )
            .into_any_element(),
        )
    }

    fn render_staged_revision(&self, plan: &Plan, cx: &Context<Self>) -> Option<AnyElement> {
        let pending = plan.pending_revision.as_ref()?;
        let status = cx.theme().status();
        let rev_note = pending.rev.map(|rev| format!("rev {rev}"));
        // Amendments (F4.7) reuse this card with an err header instead of info.
        let is_amendment =
            pending.extra.get("kind").and_then(|kind| kind.as_str()) == Some("amendment");
        let (band_bg, band_color, band_label, band_note) = if is_amendment {
            (
                status.deleted_background,
                status.deleted,
                "◆ AMENDMENT",
                "proposed after a failure — accept or reject below",
            )
        } else {
            (
                status.info_background,
                status.info,
                "STAGED REVISION",
                "plan unchanged until applied",
            )
        };
        Some(
            card_shell(
                band_label,
                band_color,
                band_bg,
                Some(band_note.into()),
                rev_note.map(SharedString::from),
                cx,
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
                    primary_button(format!("apply-{id}"), "Apply").on_click(cx.listener({
                        let id = id.clone();
                        move |this, _, _window, cx| this.apply_hunk(&id, cx)
                    })),
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
            .px_3()
            .py_1p5()
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

/// The task checkbox (compliance §7 vocabulary): 15px rounded box — empty pending,
/// accent spinner running, success ✓ fill done, error ✕ fill failed, amber ⏸
/// outline gate, amber ⚠ interrupted.
fn render_checkbox(task: &Task, cx: &App) -> AnyElement {
    let colors = cx.theme().colors();
    let status = cx.theme().status();
    let on_fill = colors.editor_background;
    if task.gate && task.status == TaskStatus::Pending {
        return cb_box(status.modified, None, "⏸", status.modified);
    }
    match task.status {
        TaskStatus::InProgress => Icon::new(IconName::ArrowCircle)
            .size(IconSize::Small)
            .color(Color::Accent)
            .with_rotate_animation(2)
            .into_any_element(),
        TaskStatus::Done => cb_box(status.created, Some(status.created), "✓", on_fill),
        TaskStatus::Failed => cb_box(status.deleted, Some(status.deleted), "✕", on_fill),
        TaskStatus::Interrupted => cb_box(status.modified, None, "⚠", status.modified),
        TaskStatus::Skipped => cb_box(colors.border, None, "–", colors.text_muted),
        TaskStatus::Pending => cb_box(colors.border, None, "", colors.text_muted),
    }
}

/// A 15px checkbox box (§7): border + optional fill + centered glyph.
fn cb_box(border: Hsla, fill: Option<Hsla>, glyph: &'static str, glyph_color: Hsla) -> AnyElement {
    h_flex()
        .size(px(15.))
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(border)
        .when_some(fill, |element, fill| element.bg(fill))
        .child(
            div()
                .text_size(px(10.))
                .text_color(glyph_color)
                .child(glyph),
        )
        .into_any_element()
}

/// A task's timeline rows (F4.3): mono verb column + detail, under the steps.
fn task_timeline(task: &Task, cx: &App) -> Vec<AnyElement> {
    let verb_color = syntax_color(cx, "type");
    task.timeline
        .iter()
        .map(|entry| {
            h_flex()
                .pl_4()
                .gap_2()
                .child(
                    Label::new(entry.kind.clone().unwrap_or_default())
                        .buffer_font(cx)
                        .size(LabelSize::XSmall)
                        .color(Color::Custom(verb_color)),
                )
                .child(
                    Label::new(entry.detail.clone().unwrap_or_default())
                        .buffer_font(cx)
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
                .into_any_element()
        })
        .collect()
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

/// The shared card chassis (compliance §6 / demo `.card`): 8px radius, panel bg,
/// a clipped colored caps header band (bold, kind-tinted) with an optional subtitle
/// and right-aligned mono note. Callers chain body rows via [`card_row`].
fn card_shell(
    label: &str,
    header_color: Hsla,
    header_bg: Hsla,
    subtitle: Option<SharedString>,
    note: Option<SharedString>,
    cx: &App,
) -> Div {
    let colors = cx.theme().colors();
    v_flex()
        .mx_3()
        .mt_2()
        .rounded_lg()
        .overflow_hidden()
        .border_1()
        .border_color(colors.border_variant)
        .bg(colors.panel_background)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .px_3()
                .py_1p5()
                .bg(header_bg)
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(header_color)
                        .child(label.to_string()),
                )
                .when_some(subtitle, |header, subtitle| {
                    header.child(Label::new(subtitle).size(LabelSize::Small).color(Color::Muted))
                })
                .child(div().flex_1())
                .when_some(note, |header, note| {
                    header.child(
                        Label::new(note)
                            .buffer_font(cx)
                            .size(LabelSize::XSmall)
                            .color(Color::Placeholder),
                    )
                }),
        )
}

/// A card body row (demo `.crow2`): top hairline + consistent padding.
fn card_row(cx: &App) -> Div {
    h_flex()
        .gap_2()
        .items_center()
        .px_3()
        .py_1p5()
        .border_t_1()
        .border_color(cx.theme().colors().border_variant)
}

/// An emphasized (accent-tinted) button for a card/toolbar primary action
/// (design-spec §3.1 `.primary`).
fn primary_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Button {
    Button::new(id.into(), label.into()).style(ButtonStyle::Tinted(TintColor::Accent))
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

/// A section header (demo `.sechead`): caps label + trailing hairline.
fn sechead(title: &str, cx: &App) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .mt_2()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().colors().text_placeholder)
                .child(title.to_string()),
        )
        .child(div().h(px(1.)).flex_1().bg(cx.theme().colors().border_variant))
}

fn bullet(text: &str) -> impl IntoElement {
    Label::new(format!("• {text}")).size(LabelSize::Small)
}

/// A syntax highlight color by name (teal `type`, purple `keyword`), falling back
/// to the accent — the design's WHEN/SHALL/ticket identity colors.
fn syntax_color(cx: &App, name: &str) -> Hsla {
    cx.theme()
        .syntax()
        .style_for_name(name)
        .and_then(|style| style.color)
        .unwrap_or(cx.theme().colors().text_accent)
}

fn render_spec(plan: &Plan, cx: &App) -> impl IntoElement {
    let spec = &plan.spec;
    let colors = cx.theme().colors();
    let when_color = syntax_color(cx, "type");
    let shall_color = syntax_color(cx, "keyword");
    v_flex()
        .p_3()
        .gap_2()
        .child(sechead("GOAL", cx))
        .child(Label::new(spec.goal.clone()).size(LabelSize::Small))
        .when(!spec.scope.r#in.is_empty(), |column| {
            column
                .child(sechead("IN SCOPE", cx))
                .children(spec.scope.r#in.iter().map(|item| bullet(item)))
        })
        .when(!spec.scope.out.is_empty(), |column| {
            column
                .child(sechead("OUT OF SCOPE", cx))
                .children(spec.scope.out.iter().map(|item| bullet(item)))
        })
        .when(!spec.acceptance.is_empty(), |column| {
            column.child(sechead("ACCEPTANCE", cx)).children(
                spec.acceptance.iter().map(|acceptance| {
                    let (glyph, glyph_color) = if acceptance.done {
                        ("✓", Color::Created)
                    } else {
                        ("○", Color::Placeholder)
                    };
                    let evidence = acceptance.evidence.first().map(|evidence| {
                        format!(
                            "{} {}",
                            evidence.evidence_type.as_deref().unwrap_or("evidence"),
                            evidence.reference.as_deref().unwrap_or_default()
                        )
                        .trim()
                        .to_string()
                    });
                    h_flex()
                        .gap_2()
                        .items_start()
                        .py_1()
                        .border_b_1()
                        .border_color(colors.border_variant)
                        .child(Label::new(glyph).size(LabelSize::Small).color(glyph_color))
                        .child(
                            h_flex()
                                .flex_1()
                                .flex_wrap()
                                .gap_1()
                                .items_center()
                                .child(
                                    Label::new("WHEN")
                                        .size(LabelSize::Small)
                                        .color(Color::Custom(when_color)),
                                )
                                .child(
                                    Label::new(acceptance.when.clone().unwrap_or_default())
                                        .size(LabelSize::Small),
                                )
                                .child(
                                    Label::new("SHALL")
                                        .size(LabelSize::Small)
                                        .color(Color::Custom(shall_color)),
                                )
                                .child(
                                    Label::new(acceptance.shall.clone().unwrap_or_default())
                                        .size(LabelSize::Small),
                                )
                                .when_some(acceptance.ticket_ac.clone(), |row, ticket| {
                                    row.child(crate::mono_chip(ticket, shall_color, cx))
                                })
                                .when_some(evidence, |row, evidence| {
                                    row.child(crate::mono_chip(evidence, colors.text_accent, cx))
                                }),
                        )
                }),
            )
        })
}

/// A dashed preview block (design-spec §3.8): `◈ label` + body rows. The full
/// input→output grid / ui-states gallery as mini-buffers is deferred (v1).
fn preview_block(label: &str, rows: Vec<String>, cx: &App) -> impl IntoElement {
    let colors = cx.theme().colors();
    v_flex()
        .gap_1()
        .p_2()
        .rounded_md()
        .border_1()
        .border_dashed()
        .border_color(colors.border)
        .bg(colors.editor_background)
        .child(
            div()
                .text_size(px(10.))
                .text_color(syntax_color(cx, "type"))
                .child(format!("◈ {label}")),
        )
        .children(rows.into_iter().map(|row| {
            Label::new(row)
                .buffer_font(cx)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
        }))
}

fn render_design(plan: &Plan, cx: &App) -> impl IntoElement {
    let design = &plan.design;
    v_flex()
        .p_3()
        .gap_2()
        .when(!design.contracts.is_empty(), |column| {
            column.child(sechead("CONTRACTS", cx)).children(
                design.contracts.iter().map(|contract| {
                    let rows = contract
                        .rows
                        .iter()
                        .map(|row| {
                            format!(
                                "{} → {}",
                                row.r#in.as_deref().unwrap_or_default(),
                                row.out.as_deref().unwrap_or_default()
                            )
                        })
                        .collect();
                    preview_block(&contract.id, rows, cx)
                }),
            )
        })
        .when(!design.decisions.is_empty(), |column| {
            column.child(sechead("DECISIONS", cx)).children(
                design.decisions.iter().map(|decision| {
                    Label::new(format!(
                        "{} — {}",
                        decision.text.as_deref().unwrap_or(""),
                        decision.rationale.as_deref().unwrap_or("")
                    ))
                    .size(LabelSize::Small)
                }),
            )
        })
        .when(!design.risks.is_empty(), |column| {
            column
                .child(sechead("RISKS", cx))
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
                .children(self.render_escalation(plan, cx))
                .children(self.render_gate_hold(plan, cx))
                .children(self.render_staged_revision(plan, cx))
                .child(match self.lens {
                    Lens::Tasks => self.render_tasks(plan, cx).into_any_element(),
                    Lens::Spec => render_spec(plan, cx).into_any_element(),
                    Lens::Design => render_design(plan, cx).into_any_element(),
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
