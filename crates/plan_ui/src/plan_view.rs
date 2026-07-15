//! The Plan tab (F1.1): a center-pane `Item` singleton that follows the active
//! thread and renders `plan.json` read-only. M3 builds it incrementally — this
//! module starts with the item shell, empty state, and open action; thread
//! binding, file-watch, lens rendering, and serialization follow.
//!
//! All colors resolve through `cx.theme()` per docs/design/plan-ui-compliance.md.

use std::path::PathBuf;

use agent_ui::{AgentPanel, AgentPanelEvent};
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, WeakEntity, Window, actions,
};
use plan_core::{Plan, Status, store};
use ui::Indicator;
use ui::prelude::*;
use workspace::{
    Workspace,
    item::{Item, ItemEvent, TabContentParams},
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
}

impl PlanView {
    /// Register the open action (and, later, the serializable item).
    pub fn register(workspace: &mut Workspace, _cx: &mut Context<Workspace>) {
        workspace.register_action(Self::open);
    }

    fn open(workspace: &mut Workspace, _: &OpenPlan, window: &mut Window, cx: &mut Context<Workspace>) {
        // Singleton: reuse the existing tab if present, else open one. Bind the
        // lookup to a local first so its borrow of `workspace`/`cx` is released.
        let existing = workspace.items_of_type::<Self>(cx).next();
        if let Some(existing) = existing {
            workspace.activate_item(&existing, true, true, window, cx);
            return;
        }
        let plans_dir = workspace
            .project()
            .read(cx)
            .visible_worktrees(cx)
            .next()
            .map(|worktree| worktree.read(cx).abs_path().join(".plans"))
            .unwrap_or_else(|| PathBuf::from(".plans"));
        let handle = cx.entity().downgrade();
        let view = cx.new(|cx| PlanView {
            focus_handle: cx.focus_handle(),
            workspace: handle,
            plans_dir,
            thread_session: None,
            plan: None,
            lens: Lens::Tasks,
            _agent_subscription: None,
        });
        workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
    }

    /// Re-point the tab at the active thread's plan (F1.1). The binding key is
    /// the ACP session id (matches `plan.thread`), read from the active thread.
    fn retarget(&mut self, agent_panel: Entity<AgentPanel>, cx: &mut Context<Self>) {
        let session = agent_panel
            .read(cx)
            .active_agent_thread(cx)
            .map(|thread| thread.read(cx).session_id().0.to_string());
        self.plan = session
            .as_deref()
            .and_then(|session| store::find_by_thread(&self.plans_dir, session));
        self.thread_session = session;
        cx.notify();
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

impl Render for PlanView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.plan.as_ref() {
            None => v_flex()
                .items_center()
                .justify_center()
                .gap_1()
                .size_full()
                .child(Label::new("Plan").size(LabelSize::Large))
                .child(
                    Label::new("no plan — ask the agent to draft one").color(Color::Muted),
                )
                .into_any_element(),
            Some(plan) => {
                // Placeholder frame until the Tasks/Spec/Design lenses land (T6/T7).
                let _ = self.lens;
                v_flex()
                    .p_3()
                    .gap_1()
                    .child(
                        Label::new(format!("Plan — {} ({:?})", plan.id, plan.status))
                            .size(LabelSize::Large),
                    )
                    .child(Label::new(plan.title.clone()).color(Color::Muted))
                    .into_any_element()
            }
        };

        v_flex()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .track_focus(&self.focus_handle)
            .child(body)
    }
}
