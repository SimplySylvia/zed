# M5b · Review loop — staged revisions + Approve gating — milestone note

**Status:** code complete, `plan_core` + `plan_server` + `plan_ui` + full `zed` build green;
**§13 visual verification under One Dark is the outstanding developer gate** (drive the
staged-revision card + Approve gating per the protocol below).

**Feature IDs:** F9.3 (staged revisions — pending diff, per-hunk Apply/Reject), F3.5 (change
traceability — provenance), F3.6 (Approve — enabled only at zero open blockers), F6.2
(`plan_propose_revision`). Deferred within M5: pushback UI (F3.4c), inline Δ chips (F3.5),
real editor-diff mini-buffers, and M5c (the `policy.json` lint engine).

## What works (built)

- **`plan_core::rev`** (the staged-revision logic, shared by UI + server) — `stage_revision`
  (inert: sets `pending_revision`, no block change / no rev bump), `apply_hunk` / `reject_hunk`
  (per-hunk state via `anchor::set_block_text`), `resolve_revision` (commits once — bumps rev,
  stamps `history`, writes each applied hunk's target into its source comment's `caused_changes`
  and marks it `addressed`, F3.5; or clears with no bump if nothing applied), plus
  `apply_all` / `reject_all`. **8 tests** (inert staging, apply+reject+resolve single bump,
  provenance, apply-all, reject-all-no-bump, resolve-no-op-while-pending, unknown-id guard,
  disk round-trip).
- **`plan_propose_revision`** (`plan_server`) — the agent stages a pending diff; Apply/Reject
  stay user-side (F9.3). Server-side `HunkArg` maps to `plan_core::rev::HunkSpec`, keeping
  schemars out of `plan_core`. **2 tests** (in-process staging + one end-to-end MCP call).
- **Staged-revision change card** (Plan tab, compliance §6 / design-spec §3.4) — renders
  `pending_revision`: **info** header band + "plan unchanged until applied", one row per hunk
  with target, `from cN` provenance chip, old (struck, deleted-bg) / new (created-bg) lines,
  and **Apply / Reject** that swap to `✓ Applied` / `Rejected`. `Apply all` is the rev-staged
  toolbar primary. The UI writes `plan.json` directly via `plan_core::rev` and reloads via the
  poll.
- **Approve gating** (compliance §3 / §9 matrix) — toolbar **⚑ n blocker** chip (error) + a
  state-driven primary: `Apply all` while staged, else `Approve` in review states, **disabled
  with a G5 tooltip** while open blockers > 0, enabled at zero → status `approved`. A
  **✓ resolve** affordance on open blocker comments (F3.4d) clears the gate in the tab.

## The round-trip
Agent calls `plan_propose_revision(hunks)` → a pending diff lands in `plan.json` → the tab
renders the staged-revision card → the user applies/rejects per hunk → applied hunks rewrite
their target blocks, `rev` bumps once on resolve, and provenance flows back into the source
comment. Separately, an open blocker (⚑ flag) disables Approve until resolved; at zero
blockers Approve transitions the plan to `approved` (tab dot → success).

## §13 record — mapping decisions & deviations (for the developer gate + a fidelity pass)

**Mapping decisions (per Part III hard rule):**
- **Staged-rev header band = `status.info`** — compliance §6 resolves the design-spec §3.4
  palette ("warn/ok/info/err/purple") to *info* for the staged-revision kind.
- **Diff-line colors** use `StatusColors` `deleted`/`deleted_background` (old, struck) and
  `created`/`created_background` (new), matching the rest of `plan_ui` and design-spec §1's
  "created/deleted (+ bg variants)". (zed-notes Q6 flagged `version_control_*` /
  `editor_diff_hunk_*` as the alternative for real diff chrome — that swap belongs with the
  mini-buffer fidelity pass.)
- **Approve-gate semantics:** an "open blocker" is `severity == "blocker" && state != "resolved"`
  (F3.4d — resolution belongs to the user). To make the gate clearable in the tab without a
  live agent, open blocker rows carry a **✓ resolve** affordance (reuses
  `comments::set_comment_state`).
- **5b toolbar primaries:** `Apply all` (staged) / `Approve` (review). Launch / Pause / Approve
  gate / Open PR primaries are **M6+** and intentionally render no primary in 5b.

**Deviations (flagged, deferred):**
- **GPUI chrome, not mini-buffers** — the change card is styled GPUI (old struck / new
  created-bg), consistent with M5a and PRD §6 ("mini-buffers can wait through M5"). Real
  editor-diff mini-buffers (`create_editor_diff`, zed-notes study #5/#7) → fidelity pass.
- **Pushback (F3.4c)** — `[Change anyway] / [Keep as planned]` deferred (M5b open q #2,
  approved); the data path exists (`plan_reply_comment` action=`pushback`).
- **Inline Δ chips (F3.5)** — provenance is shown via the card's `from cN` chip and recorded in
  `caused_changes`; per-block `Δ rev n · cN` chips deferred.
- **Acceptance-criterion revision targets** — out of scope for 5b (approved q #1 option a);
  `set_block_text` writes task/step blocks only. A criterion-targeted hunk no-ops safely.
- **External plan.json edits under a pending revision** — §12 hardening for M9.

**§13 protocol to run (developer, under One Dark):**
1. Seed a test plan with a `pending_revision` (or call `plan_propose_revision`); open the Plan
   tab → confirm the staged-revision card (§6: info band, "plan unchanged until applied",
   old-struck/new-created rows, `from cN` chip). Apply one hunk / Reject another → block text
   updates, rev bumps once, card clears; `Apply all` works.
2. Flag a task (⚑) → confirm the toolbar **⚑ 1 blocker** chip and **Approve disabled** with the
   tooltip; click **✓ resolve** → Approve enables → click → status `approved`, tab dot flips to
   success (§9 rows: in_review / rev-staged / approved).
3. Record the result here and flag any deviation before merge.

## Fork discipline
Additive only. `plan_core` gained the `rev` module; `plan_server` gained `plan_propose_revision`;
`plan_ui` gained the staged-revision card + Approve gating (writing `plan.json` via `plan_core`).
**No upstream files touched** — `FORK_DIFF.md` unchanged. (One in-crate hygiene touch: added
`#![allow(clippy::disallowed_methods)]` to the three `plan_server` stdio integration tests so
`./script/clippy -p plan_server --all-targets` passes; the lint guards async-blocking, which
these synchronous subprocess-driving tests don't do.)

## Definition of done
- [x] `plan_core::rev` stage/apply/reject/resolve correct + test-first (8 tests).
- [x] `plan_propose_revision` present + correct; staging inert; one end-to-end MCP call.
- [x] Tab: staged-revision card renders; per-hunk Apply/Reject + `Apply all`; one rev bump; provenance.
- [x] Approve gated (disabled + tooltip while a blocker is open; enabled at zero → approved).
- [x] Builds green (`plan_core`/`plan_server`/`plan_ui`/`zed`); clippy clean; pure logic test-first.
- [x] `docs/milestones/M5b.md` written; FORK_DIFF unchanged.
- [ ] **§13 visual verification under One Dark** — developer gate (protocol above).

## Next: M5c — the policy.json lint engine
`plan_core::lint` rule engine over `.plans/policy.json` (Appendix B) reused by the server:
every-task-has-tests · criteria-link-tasks · files-must-exist · prod-requires-gate ·
max-files-per-task · ticket-coverage + git format rules; agent auto-fixes what it can, the rest
become auto-flags (author: plan-lint), blocker-severity findings gate Approve (wires into the
gating built here). Then M6 execution.
