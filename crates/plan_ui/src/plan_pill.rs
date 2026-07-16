//! Status-bar pill (F1.4): `◆ Plan <fragment>` for the active thread's plan,
//! per the state matrix (design-spec §9). Clicking toggles the plan panel.

use agent_ui::{AgentPanel, AgentPanelEvent};
use gpui::{App, Context, Render, Subscription, WeakEntity, Window};
use plan_core::Plan;
use ui::prelude::*;
use workspace::dock::Panel;
use workspace::{HideStatusItem, ItemHandle, StatusItemView, Workspace};

use crate::following::PlanFollower;
use crate::{DisplayState, PlanPanel};

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

/// Whether the Plan panel is the panel currently visible in its (open) dock —
/// drives the pill's toggle and its selected/active styling.
fn plan_panel_showing(workspace: &Workspace, window: &Window, cx: &App) -> bool {
    workspace.panel::<PlanPanel>(cx).is_some_and(|panel| {
        let position = panel.read(cx).position(window, cx);
        let dock = workspace.dock_at_position(position).read(cx);
        dock.is_open()
            && dock
                .visible_panel()
                .is_some_and(|visible| visible.panel_id() == panel.entity_id())
    })
}

impl Render for PlanPill {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.follower.plan().map(|plan| {
            // Only a hold (GATE / guard) is a "needs you now" pulse at the pill —
            // unlike the status dots, which also pulse for drafting/executing (G4).
            let needs_you = crate::display_state(plan).pulses();
            (pill_fragment(plan), needs_you)
        });
        // Selected/active state: a solid fill while the panel is open (like the other
        // status-bar toggles); hover gives the standard element.hover feedback (G8).
        let panel_open = self
            .workspace
            .upgrade()
            .is_some_and(|workspace| plan_panel_showing(workspace.read(cx), window, cx));
        let hover_bg = cx.theme().colors().element_hover;
        let selected_bg = cx.theme().colors().element_selected;
        let workspace = self.workspace.clone();
        div()
            .id("plan-pill")
            .when_some(content, |el, ((fragment, color), needs_you)| {
                let pill = div()
                    .id("plan-pill-body")
                    .px_1p5()
                    .rounded_full()
                    .cursor_pointer()
                    // Active state: solid selected fill + a drop shadow so it reads
                    // clearly "raised/selected" (more noticeable than the fill alone).
                    .when(panel_open, |pill| pill.bg(selected_bg).shadow_sm())
                    .hover(|style| style.bg(hover_bg))
                    .child(
                        Label::new(format!("◆ Plan {fragment}"))
                            .color(color)
                            .size(LabelSize::Small),
                    )
                    .on_click(move |_, window, cx| {
                        if let Some(workspace) = workspace.upgrade() {
                            workspace.update(cx, |workspace, cx| {
                                // `toggle_panel_focus` only closes a focused panel, which a
                                // status-bar click doesn't retain — toggle explicitly.
                                if plan_panel_showing(workspace, window, cx) {
                                    workspace.close_panel::<PlanPanel>(window, cx);
                                } else {
                                    workspace.open_panel::<PlanPanel>(window, cx);
                                    workspace.focus_panel::<PlanPanel>(window, cx);
                                }
                            });
                        }
                    });
                // The pill pulses for needs-you holds (gate / guard-hold, contract §1),
                // unlike the status dots (which also pulse for drafting/executing).
                if needs_you {
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

/// The pill fragment + color for a plan's lifecycle surface (design-spec §9 matrix),
/// derived from [`crate::display_state`] so the pill reflects sub-states the coarse
/// `Status` can't (a failed task reads red, staged revs / lint / review counts, etc.).
pub(crate) fn pill_fragment(plan: &Plan) -> (String, Color) {
    let state = crate::display_state(plan);
    // The color family lives in one place (`DisplayState::role`); the match below
    // only supplies the fragment text.
    let color = state.role();
    let text = match state {
        DisplayState::Intake { questions } => {
            // No open questions is the matrix's intake-answered row (drafting next),
            // not a needs-you state.
            if questions == 0 {
                "intake ✓ — drafting".into()
            } else {
                let noun = if questions == 1 { "question" } else { "questions" };
                format!("{questions} {noun} — need you")
            }
        }
        DisplayState::Drafting => "drafting…".into(),
        DisplayState::Lint { open } => format!("lint {open} open"),
        DisplayState::InReview { comments, blockers } => {
            if blockers > 0 {
                format!("review · {comments}💬 ⚑")
            } else {
                format!("review · {comments}💬")
            }
        }
        DisplayState::RevStaged => format!("rev {} staged", plan.rev),
        DisplayState::Approved => "approved — launch?".into(),
        DisplayState::GuardHold => "guarded step — needs you".into(),
        DisplayState::Executing { done, total, acc_done, acc_total } => {
            format!("{done}/{total} · acc {acc_done}/{acc_total}")
        }
        // The failed-task row (design-spec §9) — red, and it's the one the coarse
        // `Amending` status couldn't express before.
        DisplayState::TaskFailed { task } => format!("{task} failed — needs you"),
        DisplayState::Gate { drift } => {
            if drift { "GATE + drift — needs you".into() } else { "GATE — needs you".into() }
        }
        DisplayState::Done { pr } => match pr {
            Some(number) => format!("done ✓ · PR #{number}"),
            None => "done ✓".to_string(),
        },
        DisplayState::Paused => "paused".into(),
        DisplayState::Abandoned => "abandoned".into(),
    };
    (text, color)
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

    #[test]
    fn task_failed_fragment_is_red() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "amending", "rev": 1,
            "thread": "a", "spec": { "goal": "g" },
            "tasks": [{"id": "t1", "status": "done"}, {"id": "t2", "status": "failed"}]
        }));
        assert_eq!(pill_fragment(&plan), ("t2 failed — needs you".to_string(), Color::Error));
    }

    #[test]
    fn done_fragment_shows_pr_number() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "done", "rev": 1,
            "thread": "a", "spec": { "goal": "g" },
            "git": { "pr": { "number": 42, "url": "https://example/pr/42" } }
        }));
        assert_eq!(pill_fragment(&plan), ("done ✓ · PR #42".to_string(), Color::Created));
    }

    #[test]
    fn done_fragment_without_pr() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "done", "rev": 1,
            "thread": "a", "spec": { "goal": "g" }
        }));
        assert_eq!(pill_fragment(&plan).0, "done ✓");
    }

    #[test]
    fn answered_intake_fragment_is_muted() {
        // No open questions → the intake-answered row: muted text *and* color.
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "intake", "rev": 1,
            "thread": "a", "spec": { "goal": "g" }
        }));
        assert_eq!(pill_fragment(&plan), ("intake ✓ — drafting".to_string(), Color::Muted));
    }

    #[test]
    fn asking_intake_fragment_is_warning() {
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "intake", "rev": 1,
            "thread": "a",
            "spec": { "goal": "g", "open_questions": [{"id":"q1","text":"why?"}] }
        }));
        assert_eq!(pill_fragment(&plan), ("1 question — need you".to_string(), Color::Warning));
    }

    #[test]
    fn gate_without_drift_fragment() {
        // A gate hold with no ticket drift → GATE, no "+ drift", amber.
        let plan = plan(serde_json::json!({
            "schema_version": 1, "id": "X", "title": "t", "status": "gate", "rev": 1,
            "thread": "a", "spec": { "goal": "g" },
            "tasks": [{"id": "g1", "gate": true, "status": "in_progress"}]
        }));
        assert_eq!(crate::display_state(&plan), DisplayState::Gate { drift: false });
        assert_eq!(pill_fragment(&plan), ("GATE — needs you".to_string(), Color::Warning));
    }
}
