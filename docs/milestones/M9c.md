# M9c · Settings + hardening — the §12 failure drills — milestone note

**Status:** COMPLETE. `plan_core` + `plan_ui` tests + clippy green; full `zed` build green. All
logic test-first. No §13 (the recovery path is exercised by tests; the M9b §13 covers the settings
UI). **With M9c, the MVP feature set is code-complete** (pending the M9b visual check).

**Feature IDs:** F11.5 (integrity + undo layers), F11.1c (MCP death mid-write), F11.5b (external
edits / merge conflict).

## What works (built)
- **Corrupt-file recovery (the real gap)** — `plan_core::store::load_or_recover(plans_dir, id) ->
  LoadOutcome { plan, recovered_from_backup, recovered_rev }`: when the live `plan.json` won't
  parse (e.g. an unresolved merge conflict), it falls back to the **newest `.bak` copy that
  parses** (F11.5's `.bak` undo layer), reporting the recovery; errors only when neither the live
  file nor any backup is valid. **3 tests** (recover-from-backup · live-file-when-valid ·
  no-backup-errors).
- **Wired into the UI** — the single-plan fallback in `following::resolve_plan` uses
  `load_or_recover`, so a corrupt sole plan recovers from `.bak` in-app rather than showing a blank
  tab.

## The §12 drills — where each lands (a designed state, not a surprise)
- **Kill the server mid-write** (F11.1c) — `store::save` writes a temp file + fsync + atomic
  `rename`, so a crash mid-write never tears the live file (the rename is all-or-nothing). Covered
  by the existing `save_leaves_no_temp_file` / round-trip tests; no new code needed.
- **Kill Zed mid-execution** (F11.1b) — the tab/panel are `SerializableItem`/`Panel`; on relaunch
  the binding re-derives and the plan reloads from disk (M3/M4). The plan file is the recovery
  point.
- **Corrupt the file** — `load` errors gracefully (never panics — existing test); `load_or_recover`
  upgrades that to `.bak` recovery (above).
- **Edit `plan.json` externally** (F11.5b) — the 500ms poll reloads external edits (they surface as
  the new state — "external edits become user revisions"). A fully-rendered **merge-conflict card**
  is **v1** (deferred); the recovery path handles the degraded/unparseable case meanwhile.

## Decisions recorded
- **`load` semantics unchanged** — it still errors on corruption (callers that want the raw result
  keep it); `load_or_recover` is the explicit recovery entry point, so recovery is never silent.
- **Recovery reports the backup rev** (`recovered_rev`) so a future UI can surface "recovered from
  rev N" — the hook is there; a banner is fidelity.

## Deferred (honored)
- **Merge-conflict card** (F11.5b full) — v1; the recovery path covers the unparseable case.
- **Three-layer undo UI** (atomic/`.bak`/git surfaced as user-facing undo) — the layers exist
  (atomic write, `.bak` recovery, git); a unified undo UI is later.

## Fork discipline
Additive only — `plan_core::store` (+`load_or_recover`/`LoadOutcome`), `plan_ui::following` (uses
it). **No upstream files touched — `FORK_DIFF.md` unchanged** (M9's upstream touches were all M9a/b).

## Definition of done
- [x] Corrupt-file → `.bak` recovery (`load_or_recover`), test-first; wired into the UI fallback.
- [x] §12 drills each documented with their designed state (atomic write / restart / corrupt /
      external edit).
- [x] `cargo test -p plan_core -p plan_ui` green; clippy clean; full `zed` build; `docs/milestones/
      M9c.md`; FORK_DIFF unchanged.

## Next: the M9b §13 visual check → **M9 done → MVP complete**. Then the v1 tail.
