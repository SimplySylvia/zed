//! GPUI surfaces for the Plan feature: the Plan tab, dock panel, status-bar
//! pill, and review UI. Everything user-visible lives here.
//!
//! M0 ships only a placeholder dock [`PlanPanel`], registered behind the
//! `ZED_PLAN` environment flag, to prove the fork's panel-registration diff and
//! the build loop. Real surfaces arrive in M3+ (see docs/milestones/).
//!
//! Fork discipline (PRD Part II §2): `plan_ui` may depend on
//! workspace/agent_ui/editor/theme; nothing may depend on `plan_ui`.

use anyhow::Result;
use gpui::{
    App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Pixels, Render, WeakEntity, Window, actions, px,
};
use ui::prelude::*;
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

pub mod following;
pub mod plan_pill;
pub mod plan_view;

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

/// M0 placeholder dock panel. Renders static content; retargeting to the active
/// thread's plan and the real pipeline/live columns (F1.3) arrive in M4.
pub struct PlanPanel {
    focus_handle: FocusHandle,
}

impl PlanPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> Result<Entity<Self>> {
        workspace.update_in(&mut cx, |_workspace, _window, cx| {
            cx.new(|cx| PlanPanel {
                focus_handle: cx.focus_handle(),
            })
        })
    }
}

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
        v_flex()
            .size_full()
            .p_4()
            .gap_1()
            .bg(cx.theme().colors().panel_background)
            .child(Label::new("Plan"))
            .child(
                Label::new("Hello from plan_ui — M0 placeholder panel (ZED_PLAN).")
                    .color(Color::Muted),
            )
    }
}
