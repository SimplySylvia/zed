# M9b · Settings — the settings-page section — milestone note

**Status:** code COMPLETE; **§13 visual verification awaiting the developer** (One Dark, covers the
M9a behaviors too). `settings_ui` (56 tests) + full `zed` build green.

**Feature IDs:** F12.2c (settings page section).

## What works (built)
- **A "Plan" page** in Zed's settings editor — a `plan_page()` pushed into
  `settings_ui/page_data.rs::settings_data` (the +1 upstream file M9 anticipated), with a `Plan`
  section header + five `SettingItem`s over the `"plan"` key: **Enable Plan** (toggle),
  **Plan Panel Dock** (dropdown), **Open Plan Panel on Startup** (toggle), **Default Lens**
  (dropdown auto/spec/design/tasks), **Revisions** (dropdown staged/auto_apply). Each uses the
  standard `SettingField` pick/write closures into `SettingsContent.plan` (mirrors the Git Panel
  page), so the built-in dropdown/toggle rendering + JSON round-trip come free.

## §13 record — decisions & deviations
**Decisions:**
- **Standard `SettingItem` primitives** — no custom rendering; dropdowns come from the enums'
  `strum::VariantNames`, toggles from the `bool` fields. Lowest-risk path in upstream-adjacent code.

**Deviations (flagged):**
- **Presets (Careful/Balanced/Fast, F12.4) deferred.** Preset *cards* need a custom multi-key-write
  widget (a `DynamicItem`/custom render), which I chose not to rush in the settings UI. **The
  substance of F12.2c — every personal key individually settable in the page — is delivered;** the
  three preset cards are a polish layer to add later (they'd write the same keys). Flagged, not
  silently dropped.
- **Policy-editor extras** (live regex tester, provenance bar, per-agent table) are **v1** — not
  built (the page is personal keys only).

**§13 protocol (developer, under One Dark):** open Zed settings (`zed: open settings` UI) → the
**Plan** page → confirm the 5 controls render (toggles + dropdowns) and changing each rewrites
`settings.json`. Then confirm the **M9a behaviors**: set `"plan".dock` (panel re-docks; right-click
"move" persists), `"plan".default_lens` (tab opens on that lens), `"plan".auto_open` (panel opens on
launch), `"plan".enabled` (feature enables without `ZED_PLAN`), `"plan".revisions: "auto_apply"` (a
staged revision auto-applies; an amendment still stays staged).

## Fork discipline
Upstream touch: `settings_ui/page_data.rs` (+`plan_page`) — the one additional file M9 anticipated,
tracked in FORK_DIFF. Mirrors the Git Panel page rows exactly.

## Definition of done
- [x] A "Plan" settings page with the 5 personal-key controls (toggles + dropdowns).
- [x] `settings_ui` tests + full `zed` build green; FORK_DIFF updated (page_data.rs).
- [ ] **§13 visual verification under One Dark — developer gate (pending).**
- [~] Presets — **deferred** (flagged; keys are individually settable).

## Next: M9c — §12 hardening drills → then MVP is complete (pending this §13).
