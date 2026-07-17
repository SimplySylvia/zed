# v1-bundle-1 — diff-view · peek cards · attention queue (COMPLETE, §13-verified)

The first **v1-tail** bundle after the MVP + fidelity passes. Plan:
`docs/milestones/v1-bundle-1-plan.md`. Executed via subagent-driven-development (implement → review →
verify), commits prefixed `[PLAN-V1A]`. Additive `plan_ui` + minimal `plan_core` (`block_text`/
`Lens` exposure) + one fork-local Cargo dep (`git_ui`) — **no upstream source files.**

**Status:** all tasks implemented + verified; `plan_ui` build + 27 tests + `plan_core` suites +
clippy green; full `zed` build green; **§13 visual verification under One Dark confirmed by the
developer (2026-07-16)** — diff-view opens a real read-only commit diff on sha-click, acceptance
evidence chips + running count render, hover-peek works on ticket/anchor/hunk chips, and the panel
"NEEDS YOU" queue lists items and jumps to the right lens (walked via `.plans/seed-state.py`).

## Task 0 — prereqs
`3d5930ab69` add `git_ui` to `plan_ui/Cargo.toml` (fork-local dep; no cycle — `agent_ui` already
pulls it) + make `plan_core::anchor::block_text` `pub`. `75b5c71bda` locks the dep.

## F4.6d — live evidence + diff-view
- `2ede59f399` **A1** — clicking a task's sha chip opens that commit's diff **read-only as a
  workspace tab** via the `pub git_ui::commit_view::CommitView::open` (guard-claused; repo from
  `active_repository`, workspace weak handle). `create_editor_diff` was private → the ready
  `CommitView::open` was used instead (per research).
- `16438b5b55` **A2** — Spec-lens acceptance rows render one mono chip per evidence entry (amber when
  `Evidence.stale`) + a running `ACCEPTANCE · n/m` sechead count.

## F0.5b — peek cards
- `ec8961e6cc` **B1** — hover-peek via `hoverable_tooltip` + `Tooltip::element`. A `peek_chip`
  helper wraps the block-resolvable chips (ticket `⛓`, comment anchor `↪`, staged-hunk `Δ from`);
  content resolves through `anchor::block_text` + `reanchor` (Outdated/Moved/Detached reflected).

## F5.7 — attention queue ("NEEDS YOU")
- `5010f50686` **C1** — pure `attention_items(plan) -> Vec<AttentionItem>` enumerating the built
  needs-you sources (questions · blockers · lint · guard/gate holds · failed tasks · drift ·
  rev-behind), reusing existing predicates; table-tested (5 tests, TDD).
- `ec6ed6a01e` **C3** — design-doc divergence (contract §10 + design-spec §5): F5.7 ships as a
  rendered active-plan panel section, not the PRD's pill click-cycle (signed off).
- `e25a097a5b` **C2** — the panel renders a caps "NEEDS YOU" section (full width above the
  pipeline/live columns) from `attention_items`; each row is clickable and does a coarse jump
  (`open_tab` now returns the `Entity<PlanView>`, then `set_lens`).

## Mapping decisions
- **Diff-view opens as a workspace tab** (Level A) — inline-in-card mini-buffer (Level B) needs
  copying the private `create_editor_diff` + `editor`/`buffer_diff` deps; deferred.
- **Peek is hover-only** via tooltip; ⌥-hover / Esc / click-to-jump (custom `deferred()`/`anchored()`
  overlay) deferred.
- **Attention queue is active-plan + coarse-jump**; cross-plan aggregation, the pill click-cycle,
  and precise scroll-to-item deferred.

## Deferrals (recorded)
- F4.6d: inline-in-card diff mini-buffer; UI-side evidence freshness computation (plan_core is
  time-free — `stale` is agent-supplied); test-chip re-run-on-click (agent-routed); deriving the
  acceptance checkbox from evidence (`done` stays authoritative).
- F0.5b: ⌥-hover / Esc / click-to-jump; the external `ticket_ac #n` peek (no cached criterion data);
  the `peek-on-hover` settings toggle (would touch upstream `default.json`).
- F5.7: cross-plan aggregation + pill click-cycle + precise scroll-to-item; the not-yet-built
  sources (rehearsal F9.4, pushback F9.2).

## Fork discipline
Additive `plan_ui` + `plan_core` `block_text`/`Lens` `pub` + one Cargo dep (`git_ui`). No upstream
source files; `FORK_DIFF.md` unchanged.
