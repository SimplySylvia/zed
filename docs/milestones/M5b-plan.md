# M5b · Review loop — staged revisions + Approve gating — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the `rev` module + server tool are test-first over the
> LED-212 fixture); and the `plan-ui-design` skill + `plan-ui-compliance.md` (§3.4 staged-
> revision card, §3.1 toolbar/primary, §9 state matrix) for the review UI.

**Status:** awaiting sign-off (not started).
**Feature IDs:** F9.3 (staged revisions — pending diff, per-hunk Apply/Reject), F3.5 (change
traceability — old→new + caused-by provenance), F3.6 (Approve — enabled only at zero open
blockers), F6.2 (server tool `plan_propose_revision(hunks)`).
**Goal:** the staged-revision round-trip — the agent proposes a plan change via
`plan_propose_revision(hunks)`; it lands as a **pending diff** (not applied); the Plan tab
renders a staged-revision change card with per-hunk **Apply / Reject**; applying writes the
new text to the target block, records provenance back to the source comment, and bumps `rev`
once when the revision resolves; and the toolbar **Approve** primary is gated — disabled with
a tooltip while any open blocker exists, enabled at zero, transitioning the plan to `approved`.

**Architecture:** unchanged from 5a. The **UI writes `plan.json` directly** via `plan_core`
(PRD §6: UI writes, the server writes for the agent); the **server writes for the agent**.
The pending-revision apply/reject logic lives in a new `plan_core::rev` module (shared +
test-first), reused by `plan_ui`. The agent-facing `plan_propose_revision` only *stages*;
per F9.3 Apply/Reject is the user's action (mirrors Zed's Keep/Reject) — the same "proposed
diff" primitive that execution-time amendments (F4.7, M6) will reuse.

**Tech stack:** Rust — `plan_core::rev` (stage/apply/reject/resolve), `plan_server`
(`plan_propose_revision`), `plan_ui` (staged-revision card + Approve gating over the M5a tab).
Pure logic test-first; GPUI = smoke + §13.

Build env as before. Commit prefix `[PLAN-M5b]`.

---

## Tasks

### T1 — `plan_core::rev` — pending-revision apply/reject (TDD)
`[PLAN-M5b]: add pending-revision apply/reject to plan_core`
New module `crates/plan_core/src/rev.rs` (PRD Part II §2 repo layout). Pure functions on
`&mut Plan`, reusing `anchor::set_block_text` (the shared block-write helper) for target
addressing:
- `stage_revision(plan, hunks)` — set `plan.pending_revision` with hunks in state `"pending"`,
  each assigned an id (`h1`..); `pending_revision.rev = plan.rev + 1` (the rev the revision
  *will* become). **Does not** mutate blocks or bump `rev` — staging is inert.
- `apply_hunk(plan, hunk_id)` — write the hunk's `new` text to its `target` block via
  `anchor::set_block_text`; mark the hunk `"applied"`. Returns false on unknown id / unresolved
  target.
- `reject_hunk(plan, hunk_id)` — mark the hunk `"rejected"`; no text change.
- `resolve_revision(plan)` — when no hunk remains `"pending"`: if any was applied, bump `rev`
  to `pending_revision.rev`, stamp `history` (`kind: "revision"`), and record provenance —
  for each applied hunk with a `from` (e.g. `c1`), push the hunk target into that comment's
  `caused_changes` (F3.5) and mark the source comment `"addressed"`. Clear `pending_revision`.
  If every hunk was rejected, clear with no rev bump. Returns the committed rev (or None).
- Convenience: `apply_all` / `reject_all` (fold over hunks then `resolve_revision`).
- **Tests first** over the LED-212 fixture: stage 2 hunks → `pending_revision` set, blocks
  untouched, `rev` unchanged; apply one + reject one → target block gets `new` text, rejected
  block unchanged, one `rev` bump on resolve, history stamped, `caused_changes` recorded on the
  source comment, `pending_revision` cleared; apply-all path; reject-all clears with no bump;
  unknown-id / unresolved-target guards return false.
- **Stop-and-ask:** `set_block_text` currently addresses tasks + steps but **not** acceptance
  criteria (it reads them but has no writer). If a revision must target an acceptance criterion,
  extend `set_block_text` to write acceptance `when`/`shall`; if the addressing model doesn't
  generalize cleanly, STOP and reconsider before building the card on it.

### T2 — `plan_server` `plan_propose_revision` tool (TDD)
`[PLAN-M5b]: add plan_propose_revision MCP tool`
- Agent-facing `plan_propose_revision(id, hunks)` reusing T1's `stage_revision`. Hunk input =
  `[{ target, old, new, from }]`; the tool assigns hunk ids and `"pending"` state and persists.
  **Apply/Reject stay user-side** (F9.3) — the agent only proposes; the tool never applies.
- Note in the tool doc that amendments (F4.7, M6) are the same primitive staged during
  execution.
- **Tests:** in-process over a temp `.plans` — propose a revision → `pending_revision`
  persisted with hunks pending, `rev` unchanged; one end-to-end MCP `tools/call` (mirroring the
  M5a handshake test).

### T3 — Staged-revision change card + per-hunk Apply/Reject UI (compliance §3.4) [5b]
`[PLAN-M5b]: add staged-revision change cards to the Plan tab`
- Render `plan.pending_revision` as a **staged-revision card** (design-spec §3.4, shared card
  chassis): modified/amber header band with the note "plan unchanged until applied"; one row
  per hunk — target + provenance chip ("from c1") + `old` line (struck, `deleted` bg) / `new`
  line (`created` bg) + an **Apply / Reject** pair, swapping to `✓ Applied` / `Rejected` once
  the hunk resolves.
- Apply/Reject call T1 via `plan_core`, `store::save`, and reload via the existing poll (as
  M5a's `add_flag` does). When the last pending hunk resolves, `resolve_revision` commits/clears.
- **Toolbar primary in the rev-staged state (§9 matrix):** `Apply all` when a `pending_revision`
  exists (Approve is disabled in this state per the matrix).
- **Rendering fidelity (open q #1):** render the diff as **GPUI chrome** (old struck / new
  `created`-bg lines), matching M5a's comment fidelity and PRD §6 "mini-buffers can wait through
  M5". Real editor-diff mini-buffers (`create_editor_diff` / git_ui hunk machinery, zed-notes
  study #5/#7) are deferred to the fidelity pass (already in the deferred backlog).
- **Acceptance:** agent proposes a revision → card appears → Apply one hunk / Reject another →
  target block text updates, `rev` bumps once, provenance chip links the source comment, card
  clears; §13 §3.4 checks under One Dark.

### T4 — Approve gating (F3.6) [5b]
`[PLAN-M5b]: add Approve gating to the Plan toolbar`
- Toolbar per §3.1 order: add the **⚑ n blocker chip** (count of open `blocker`-severity
  comments) and the state-driven **primary** (§9 matrix): in `in_review` / lint states show
  **Approve**, disabled while any open blocker exists with a tooltip naming the count (§8
  "disabled primaries always carry a tooltip"); at zero open blockers → enabled; clicking sets
  status → `approved` via `plan_core` (rev/history stamped), flipping the tab dot to `created`
  (§9). Coexists with the M5a "Send for revision · n" action and T3's "Apply all".
- **Boundary:** F3.6 says Approve "integrates ExitPlanMode" — that ACP/launch integration is
  **M6** (Execution: "Launch flow + ExitPlanMode integration"). 5b implements the UI gating +
  the `in_review → approved` status transition only; flag the ExitPlanMode seam in M5b.md.
- **Acceptance:** with an open blocker flag, Approve is disabled + tooltip; reject/resolve the
  blocker → Approve enables → click → status `approved`, tab dot success; §13 §9 row checks.

### T5 — Milestone note + FORK_DIFF + §13
`[PLAN-M5b]: add M5b milestone note`
`docs/milestones/M5b.md` (what works / deferred / surprises); FORK_DIFF expected unchanged (all
additive — a new `plan_core` module, a new server tool, `plan_ui` additions); full `plan_ui` +
`zed` build; §13 visual-verification handoff for the staged-revision card + Approve gating +
the rev-staged/approved state-matrix rows under One Dark.

---

## Definition of done
- [ ] `plan_core::rev` stage/apply/reject/resolve correct and test-first (fixture tests green,
      incl. provenance + single rev-bump-on-resolve + reject-all-no-bump + guards).
- [ ] `plan_propose_revision` present + correct; one end-to-end MCP call verified; staging is
      inert (no rev bump, no block change until the user applies).
- [ ] Tab: agent proposes → staged-revision card renders → per-hunk Apply/Reject → block text
      updates + one rev bump + provenance chip; `Apply all` works.
- [ ] Approve gated: disabled + tooltip while a blocker is open; enabled at zero → status
      `approved`, tab dot flips per §9.
- [ ] Builds green; pure logic test-first; `docs/milestones/M5b.md` written; FORK_DIFF unchanged.
- [ ] §13 visual verification (staged-revision card §3.4, toolbar/blocker chip §3.1, rev-staged
      + approved matrix rows §9) under One Dark — developer gate.

## Open questions (recommendations in parens)
1. **Change-card rendering** — GPUI chrome (old struck / new `created`-bg) vs real editor-diff
   mini-buffers now? *(GPUI chrome for 5b — consistent with M5a and PRD §6 "mini-buffers can
   wait through M5"; real `create_editor_diff` mini-buffers deferred to the fidelity pass. The
   Apply/Reject **logic** is real in `plan_core` regardless; only the rendering is chrome.)*
2. **Pushback (F3.4c)** — the handoff's 5b scope lists only propose-revision + change cards +
   Approve gating and omits pushback; M5a also deferred pushback buttons. *(Defer the pushback
   UI — `[Change anyway]/[Keep as planned]` — to a later review-fidelity slice; the data path
   already exists (`plan_reply_comment` action=`pushback`). Flag it, don't build it in 5b.)*
3. **Inline Δ chips on changed blocks (F3.5)** — render `Δ rev n · c1` chips on blocks a
   committed revision touched? *(Stretch — provenance is captured in `caused_changes`; render
   the chip if cheap after T3, otherwise defer. The staged-revision card's "from c1" chip is the
   MVP provenance surface.)*
4. **Who applies hunks** — UI only, per F9.3? *(Yes — `plan_propose_revision` stages; Apply/Reject
   is the user in the tab, mirroring Keep/Reject. The `staged | auto_apply` per-plan/agent setting
   is M9; default staged.)*
5. **Does staging clear on external plan edits?** — if the user edits `plan.json` under a pending
   revision. *(Out of scope for 5b — last-writer-wins at the file level per §6; the card renders
   whatever `pending_revision` is on disk. Note as a §12 hardening item for M9.)*

## Heads-up
Second slice of the biggest milestone. T1 (`rev`) is the load-bearing logic and gets the
test-first treatment; the `set_block_text` acceptance-criterion gap (T1 stop-and-ask) is the one
place the block-addressing model might not stretch. T3 is the substantial UI piece. Expect a
checkpoint after T1 (and after T2) before the UI work. M5c (the `policy.json` lint engine) is the
remaining M5 slice after this.
