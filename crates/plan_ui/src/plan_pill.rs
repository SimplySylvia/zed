//! Status-bar pill (F1.4): `◆ Plan <fragment>` for the active thread's plan,
//! per the state matrix (design-spec §9). Clicking toggles the plan panel.

use agent_ui::{AgentPanel, AgentPanelEvent};
use gpui::{App, Context, Render, Subscription, WeakEntity, Window};
use plan_core::{Plan, Status, TaskStatus};
use ui::prelude::*;
use workspace::{HideStatusItem, ItemHandle, StatusItemView, Workspace};

use crate::PlanPanel;
use crate::following::PlanFollower;

pub struct PlanPill {
    follower: PlanFollower,
    workspace: WeakEntity<Workspace>,
    _agent_subscription: Option<Subscription>,
    _watch_task: Option<gpui::Task<()>>,
}

impl PlanPill {
    pub fn new(workspace: &Workspace, cx: &mut Context<Self>) -> Self {
        let mut pill = Self {
            follower: PlanFollower::new(workspace, cx),
            workspace: workspace.weak_handle(),
            _agent_subscription: None,
            _watch_task: None,
        };
        if let Some(agent_panel) = workspace.panel::<AgentPanel>(cx) {
            pill._agent_subscription =
                Some(cx.subscribe(&agent_panel, |this, _panel, event, cx| {
                    if matches!(event, AgentPanelEvent::ActiveViewChanged) && this.follower.refresh(cx)
                    {
                        cx.notify();
                    }
                }));
        }
        pill.follower.refresh(cx);
        pill._watch_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                if view
                    .update(cx, |this, cx| {
                        if this.follower.refresh(cx) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        pill
    }
}

impl Render for PlanPill {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self
            .follower
            .plan()
            .map(|plan| (pill_fragment(plan), plan.status.clone()));
        let workspace = self.workspace.clone();
        div()
            .id("plan-pill")
            .when_some(content, |el, ((fragment, color), status)| {
                // Rounded caps pill, tinted by the state family (compliance §1).
                let tint = color.color(cx);
                let pill = div()
                    .id("plan-pill-body")
                    .px_1p5()
                    .rounded_full()
                    .border_1()
                    .border_color(tint)
                    .bg(tint.opacity(0.1))
                    .cursor_pointer()
                    .child(
                        Label::new(format!("◆ Plan {fragment}"))
                            .color(color)
                            .size(LabelSize::Small),
                    )
                    .on_click(move |_, window, cx| {
                        if let Some(workspace) = workspace.upgrade() {
                            workspace.update(cx, |workspace, cx| {
                                workspace.toggle_panel_focus::<PlanPanel>(window, cx);
                            });
                        }
                    });
                // The pill pulses only for gate/guard-hold (contract §1), unlike the
                // status dots (which also pulse for drafting/executing).
                if matches!(status, Status::Gate) {
                    el.child(crate::pulse(pill, "plan-pill-pulse"))
                } else {
                    el.child(pill)
                }
            })
    }
}

impl StatusItemView for PlanPill {
    // The pill follows the active thread (not the active pane item).
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn hide_setting(&self, _cx: &App) -> Option<HideStatusItem> {
        None
    }
}

/// The pill fragment + color for a plan's lifecycle state (design-spec §9).
pub(crate) fn pill_fragment(plan: &Plan) -> (String, Color) {
    match plan.status {
        Status::Executing => {
            let done = plan
                .tasks
                .iter()
                .filter(|task| task.status == TaskStatus::Done)
                .count();
            let acc_done = plan.spec.acceptance.iter().filter(|a| a.done).count();
            (
                format!(
                    "{done}/{} · acc {acc_done}/{}",
                    plan.tasks.len(),
                    plan.spec.acceptance.len()
                ),
                Color::Info,
            )
        }
        Status::Intake => ("intake — need you".into(), Color::Warning),
        Status::Drafting => ("drafting…".into(), Color::Muted),
        Status::InReview => ("in review".into(), Color::Warning),
        Status::Revising => ("revising".into(), Color::Warning),
        Status::Approved => ("approved — launch?".into(), Color::Created),
        Status::Paused => ("paused".into(), Color::Muted),
        Status::Gate => ("gate — needs you".into(), Color::Warning),
        Status::Amending => ("amending".into(), Color::Warning),
        Status::Done => ("done ✓".into(), Color::Created),
        Status::Abandoned => ("abandoned".into(), Color::Muted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(value: serde_json::Value) -> Plan {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn executing_fragment_shows_task_and_acceptance_progress() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "executing", "rev": 1,
            "thread": "a",
            "spec": { "goal": "g", "acceptance": [{"id":"a1","done":true},{"id":"a2","done":false}] },
            "tasks": [{"id":"t1","status":"done"},{"id":"t2","status":"pending"}]
        }));
        assert_eq!(pill_fragment(&plan).0, "1/2 · acc 1/2");
    }

    #[test]
    fn drafting_fragment() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "drafting", "rev": 1,
            "thread": "a", "spec": { "goal": "g" }
        }));
        assert_eq!(pill_fragment(&plan).0, "drafting…");
    }
}
