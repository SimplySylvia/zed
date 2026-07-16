//! The `"plan"` settings key (PRD §13 F12.1) — how *you* drive the Plan feature
//! (distinct from `.plans/policy.json`, which is what the *work* must satisfy).
//! Mirrors the `GitPanelSettings` template: a `PlanSettingsContent` in upstream
//! `settings_content` + defaults in `default.json`, mapped here via `from_settings`
//! and auto-registered by `#[derive(RegisterSetting)]`.

use settings::{
    PlanDefaultLens, PlanRevisionMode, RegisterSetting, Settings, SettingsContent,
};
use workspace::dock::DockPosition;

#[derive(Debug, Clone, PartialEq, RegisterSetting)]
pub struct PlanSettings {
    pub enabled: bool,
    pub dock: DockPosition,
    pub auto_open: bool,
    pub default_lens: PlanDefaultLens,
    pub revisions: PlanRevisionMode,
}

impl Settings for PlanSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let plan = content.plan.clone().unwrap();
        Self {
            enabled: plan.enabled.unwrap(),
            dock: plan.dock.unwrap().into(),
            auto_open: plan.auto_open.unwrap(),
            default_lens: plan.default_lens.unwrap(),
            revisions: plan.revisions.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fully-populated content block round-trips through `from_settings` with the
    /// values intact (and every field is handled — the mapping's completeness check).
    #[test]
    fn from_settings_maps_a_populated_block() {
        let mut content = SettingsContent::default();
        content.plan = Some(settings::PlanSettingsContent {
            enabled: Some(true),
            dock: Some(settings::DockPosition::Right),
            auto_open: Some(true),
            default_lens: Some(PlanDefaultLens::Tasks),
            revisions: Some(PlanRevisionMode::AutoApply),
        });
        let settings = PlanSettings::from_settings(&content);
        assert!(settings.enabled);
        assert_eq!(settings.dock, DockPosition::Right);
        assert!(settings.auto_open);
        assert_eq!(settings.default_lens, PlanDefaultLens::Tasks);
        assert_eq!(settings.revisions, PlanRevisionMode::AutoApply);
    }
}
