# M9a · Settings + hardening — the `"plan"` settings key + consumption — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the `from_settings` mapping + any `plan_core` hardening are
> test-first); the `plan-ui-design` skill contract if/when the settings-page UI (M9b) is built.

**Status:** COMPLETE — M9a/M9b/M9c all done; **§13 confirmed by the developer (2026-07-15)**.
**M9 was the last MVP milestone — MVP is now complete.** Presets (F12.4) deferred; the settings
key/page were the accepted upstream touch (FORK_DIFF).

## M9 is split — this plan is slice **a** of three
- **M9a (this) — the `"plan"` settings key + consumption.** Add `PlanSettingsContent` to upstream
  `settings_content`, defaults to `assets/settings/default.json`, a `PlanSettings`
  (`#[derive(RegisterSetting)]` + `from_settings`) in a new additive `plan_ui::plan_settings`, and
  **consume the core personal keys** in `plan_ui` (dock position, auto-open, default lens, revisions
  staged|auto) + a durable `enabled` key alongside the `ZED_PLAN` env flag. *Demo: set
  `settings.json → "plan"` keys and see the panel dock / default lens / revision mode change; the
  feature enables via the setting (not just the env var).*
- **M9b — settings-page section + presets (F12.2c).** A "Plan" section in upstream
  `settings_ui/page_data.rs`: the personal-key controls + the **Careful/Balanced/Fast presets**
  (presets write the same keys, F12.4). GPUI + §13. **Policy-editor extras** (live regex tester,
  provenance bar, per-agent table) are **v1** — the section is personal keys + presets only.
- **M9c — §12 hardening drills.** Kill the server mid-write (atomic temp+rename already prevents
  tears — verify) · kill Zed mid-execution (restore-from-file — verify) · **corrupt the file**
  (→ `.bak` recovery + a designed error state, not a crash) · **edit `plan.json` externally**
  (→ becomes a user revision; a merge conflict on the file renders a conflict card, F11.5b). Mostly
  `plan_core::store` hardening + tests + filling the recovery gaps.

**Fork caveat (zed-notes study #8 — the headline decision):** the settings **key** touches **2
upstream files** (`crates/settings_content/src/settings_content.rs` + `assets/settings/
default.json`) and the settings **page** (M9b) touches **1 more** (`settings_ui/page_data.rs`).
These are well-trodden (dozens of examples) but **not purely additive** — the first upstream touch
beyond M0/M4 registration + wiring. All must be tracked in `FORK_DIFF.md`. A raw-JSON fallback
exists today, so the structured page can be deferred (open question 1).

**Feature IDs (M9a):** F12.1 (personal keys), F12.2 (precedence — managed→project→personal), plus
the durable enable flag (M0 deferred it here).

**Architecture:** the settings **key** is Zed-native (`SettingsContent` typed model + `from_settings`
+ `inventory` auto-register — no manual registration). `plan_ui` reads `PlanSettings::get(cx)` at
render/init. Policy (`.plans/policy.json`) is unchanged — it's the *work* contract; settings are
*how you drive* (PRD §13's two homes). Policy severity wiring already exists (`lint::Policy`,
`git::GitPolicy`, `exec::GuardPolicy` all load from policy.json) — M9 does **not** re-do it.

**Tech stack:** Rust — upstream `settings_content` + `default.json`; additive `plan_ui::plan_settings`
+ consumption. `from_settings` mapping test-first. Commit prefix `[PLAN-M9a]`.

---

## Tasks

### T1 — `PlanSettingsContent` + defaults (upstream) + `PlanSettings` (additive, TDD)
`[PLAN-M9a]: add the plan settings key and from_settings mapping`
- **Upstream (tracked in FORK_DIFF):** add `pub plan: Option<PlanSettingsContent>` to
  `SettingsContent` + define `PlanSettingsContent` (all fields `Option<T>`), mirroring
  `GitPanelSettingsContent`. Add the defaults to `assets/settings/default.json` (every field needs
  one — `from_settings` panics on a missing default, per study #8).
- **Additive:** `crates/plan_ui/src/plan_settings.rs` — `#[derive(RegisterSetting)] PlanSettings` +
  `impl Settings { fn from_settings(content) -> Self }`, mapping the content to typed values with
  fallbacks. **MVP keys only** (those that gate shipped behavior — open question 3):
  `enabled: bool`, `dock: DockPosition` (default Bottom), `auto_open: bool`, `default_lens:
  auto|spec|design|tasks` (auto follows status), `revisions: staged|auto_apply`.
- **Tests:** `from_settings` maps set values + falls back to defaults for absent ones; the
  `default.json` block deserializes; precedence tightens (a project value overrides personal).

### T2 — Consume the keys in `plan_ui`
`[PLAN-M9a]: drive the panel, lens, and revisions from plan settings`
- **`enabled`** — the durable flag: the feature turns on when `plan.enabled == true` **or**
  `ZED_PLAN` is set (keep the env var for dev). Gate the panel + pill + item registration on the
  combined check (extends the existing `plan_enabled()`).
- **`dock`** — the panel's `position`/`set_position` read/write this key (real right-click-move
  persistence, per study #1 — currently the panel may hardcode Bottom).
- **`default_lens`** — `default_lens(status)` honors the setting (auto → follows status, else the
  chosen lens).
- **`auto_open`** / **`revisions`** — auto-open the tab on draft when set; the staged-vs-auto_apply
  revision default reads the key (staged stays the default).
- **Acceptance:** changing each key in `settings.json` changes behavior; unset = today's defaults.
  Smoke where testable; the visual/interaction confirmation folds into M9b's §13.

### T3 — Milestone note + FORK_DIFF + build
`[PLAN-M9a]: add M9a milestone note`
`docs/milestones/M9a.md`; **FORK_DIFF updated** with the 2 upstream files (settings_content +
default.json) — the accepted conflict surface + why; full `zed` build; `cargo test`. No §13 (the
settings UI is M9b) beyond confirming the feature still enables/renders.

---

## M9b / M9c sketch (separate sign-off each)
- **M9b:** push a `SectionHeader("Plan")` + `SettingItem`s (pick/write closures into
  `SettingsContent.plan`) into `settings_data` (`page_data.rs`, +1 upstream file) + the 3 preset
  cards (each rewrites the same keys). §13 under One Dark. Policy-editor extras stay v1.
- **M9c:** `plan_core::store` — corrupt-file → `.bak` recovery (load falls back to the newest good
  `.bak` + surfaces a designed error, not a panic); external-edit/merge-conflict → user-revision /
  conflict-card path (F11.5b). Drills as tests + a short manual runbook in the note.

## Scope guard (MVP vs v1)
MVP = the settings **key** + personal keys + the settings **page** + presets (PRD §14). **v1:**
per-agent overrides (F12.1b), the policy-editor extras (regex tester / provenance / propose-as-PR),
distilled-rules promotion (F12.3b). Don't build those — flag.

## Definition of done (M9a)
- [ ] `PlanSettingsContent` + `default.json` defaults + `PlanSettings`/`from_settings` correct,
      test-first; precedence verified.
- [ ] `plan_ui` consumes enabled/dock/default_lens/auto_open/revisions; unset = current defaults.
- [ ] Full `zed` build green; clippy clean; **FORK_DIFF updated** (2 upstream files); `docs/
      milestones/M9a.md`.

## Open questions (recommendations in parens)
1. **Build the structured settings page (M9b), or use the raw-JSON fallback for MVP?** *(Do **M9a**
   now — the key is needed for any real settings and its upstream touch is minimal/standard (2
   files). For **M9b**, my recommendation is **build the minimal Plan section + presets** — PRD §14
   lists "settings page" as MVP and the page_data.rs touch is 1 well-trodden file. But if you'd
   rather hold the fork line tighter, we **defer M9b** and rely on the raw-JSON escape hatch
   (settings still work, just edited as JSON), marking F12.2c a documented deferral. **Your call —
   it's the fork-thesis tradeoff.**)*
2. **Durable enable vs `ZED_PLAN`.** *(The feature enables on `plan.enabled == true` **or**
   `ZED_PLAN` set — keep the env var for dev/CI; the setting is the durable path M0 deferred.)*
3. **Which personal keys in MVP?** *(Only keys that gate shipped behavior: enabled, dock, auto_open,
   default_lens, revisions. Keys for v1 features — peek-on-hover, rehearse, worktree mode, attention
   pulses, stuck thresholds — are added **when those features land**, not as dead no-ops now.)*
4. **`from_settings` panic-on-missing-default.** *(Study #8: a missing `default.json` default makes
   `from_settings` panic. Every `PlanSettingsContent` field gets a default in `default.json` — a
   T1 checklist item, verified by the deserialize test.)*

## Heads-up
This is the milestone that **breaks the near-zero fork diff** — deliberately, and along a
well-trodden path. Keep the upstream edits to the exact fields/rows needed, mirror
`GitPanelSettingsContent`/`git_panel_settings.rs` verbatim, and record every touched file in
FORK_DIFF with the reason. Expect a checkpoint after **T1** (the key + mapping) before consumption.
Policy severity wiring is **already done** (M5c/M7a/M6b) — don't re-litigate it.
