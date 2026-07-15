# M5a · Review loop — comments round-trip — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (anchor + core writes + server tools are test-first,
> and the anchor gets **property tests** per PRD §7); and the `plan-ui-design` skill +
> `plan-ui-compliance.md` (§9 review primitives) for the review UI.

**Status:** awaiting sign-off (not started).
**Feature IDs:** F3.1 (selection & anchors), F3.2a/b/d (comment / suggest / quick-flag),
F3.3 (severity), F3.4 (batch → revision), F6.2 (server tools:
`list_comments`/`reply_comment`/`apply_suggestion`/`mark_addressed`). (F3.2c alternatives,
F3.6 Approve gating → 5b/later.)
**Goal:** the comment round-trip — you add a block-anchored comment/flag with a severity in
the Plan tab; it persists to `plan.json`; the agent reads it (`list_comments`), replies /
marks it addressed / applies a suggestion; the tab shows the thread + its state; and anchors
survive plan revisions via fuzzy re-anchoring.

**Architecture:** the **UI writes `plan.json` directly** via `plan_core` (PRD §6: UI writes,
the server writes for the agent — last-writer-wins at the file level). Comment mutation
helpers live in `plan_core` (shared + testable) and are reused by both `plan_ui` and
`plan_server`. `plan_core::anchor` re-anchors comments across revisions (the §7 risk — built
and property-tested **before** any UI depends on it).

**Tech stack:** Rust — `plan_core` (anchor, comment mutations), `plan_server` (rmcp review
tools), `plan_ui` (review UI over the M3 tab). Pure logic test-first; GPUI = smoke + §13.

Build env as before. Commit prefix `[PLAN-M5a]`.

---

## Tasks

### T1 — `plan_core::anchor` — fuzzy re-anchoring (SPIKE, property-test-first) §7
`[PLAN-M5a]: add fuzzy comment re-anchoring`
- `reanchor(anchor: &Anchor, plan: &Plan) -> ReanchorResult` — locate the anchored block in
  the current plan; if the `quote` still matches → anchored; if the block exists but text
  changed → mark **outdated**, preserve the original quote; if only a fuzzy match → best
  candidate + outdated; if nothing → detached.
- **Property tests first** (mutation fixtures over the LED-212 plan): block unchanged →
  stays anchored; block text edited → outdated, original quote preserved; block reordered →
  still found; block deleted → detached. Fuzz with small random edits and assert invariants.
- **§7 stop-and-ask:** if re-anchoring quality is poor (false matches / misses), STOP and
  reconsider the approach before building comment UI on it.

### T2 — `plan_core` comment mutations (TDD)
`[PLAN-M5a]: add comment mutation helpers to plan_core`
Pure functions over `&mut Plan` (reused by UI + server), each rev-bumping + stamping history:
`add_comment(kind, author, severity, anchor, text)`, `set_comment_state(id, state)`,
`reply(id, author, text, action)`, `mark_sent_batch()` (open → sent for all open),
`apply_suggestion(id)` (applies the suggestion's replacement to the target block, records
`applied: verbatim`). **Tests:** each mutation's effect on the LED-212 fixture (a comment
added, a batch marked sent, a suggestion applied to a step).

### T3 — `plan_server` review tools (TDD)
`[PLAN-M5a]: add review MCP tools`
Agent-facing tools reusing T2: `plan_list_comments` (open/sent items with anchors),
`plan_reply_comment(id, text, action)`, `plan_mark_addressed(id)`, `plan_apply_suggestion(id)`.
(Resolution stays the user's — F3.4d.) **Tests:** in-process over a temp `.plans` with seeded
comments; end-to-end MCP call for one tool.

### T4 — Review UI: add + render comments (compliance §9) [5a]
`[PLAN-M5a]: add block-anchored commenting to the Plan tab`
- Block-level affordance on Tasks-lens blocks (hover/▸ button → a small toolbar:
  💬 Comment / ✎ Suggest / ⚑ Flag). **Block anchoring for 5a** (`{lens, block, quote}`); text-
  range selection (needs mini-buffers) is deferred.
- A composer (severity ⚑/⚠/💡 + text) that writes a `Comment` to `plan.json` via `plan_core`
  (T2), then the file-watch reload re-renders.
- Render comment threads near their anchored block: state chip (open amber / addressed /
  resolved green), messages, anchor label `c1 ↪ t2.s1`, **outdated** badge when re-anchoring
  flagged it. Severity/blocker counts surface on the tab.
- **Batch send:** a "Send for revision · n" action (marks open comments sent). One rev bump.
- **Acceptance:** add a flag on a task in the running app → it persists, shows in the thread,
  and `plan_list_comments` returns it; §13 §9 checks (per available primitives).

### T5 — Milestone note + FORK_DIFF + §13
`[PLAN-M5a]: add M5a milestone note`
`docs/milestones/M5a.md`; FORK_DIFF unchanged expected (all additive); full build; §13 handoff.

---

## Definition of done
- [ ] `plan_core::anchor` re-anchors correctly (property tests green); §7 quality acceptable.
- [ ] Comment mutations (add/state/reply/batch-send/apply-suggestion) test-first in `plan_core`.
- [ ] Review MCP tools present + correct; one end-to-end MCP call verified.
- [ ] Tab: add a block-anchored comment/flag with severity → persists → thread renders →
      agent `list_comments` sees it; batch-send works.
- [ ] Builds green; pure logic test-first; `docs/milestones/M5a.md` written.
- [ ] §13 visual verification (review primitives, per available surface) — developer gate.

## Open questions (recommendations in parens)
1. **Anchoring granularity for 5a** — block-level (click a task/step/criterion) vs text-range
   selection? *(block-level for 5a; text-range + mini-buffers later — avoids pulling mini-
   buffers in now, per §6 "mini-buffers can wait")*.
2. **Who writes comments** — the UI writes `plan.json` directly via `plan_core` (PRD §6)?
   *(yes; UI load→mutate→save with a rev bump + history stamp, mirroring the server's `mutate`)*.
3. **Comment mutation helpers' home** — `plan_core` (shared by UI + server) vs duplicated?
   *(plan_core — single source, testable)*.
4. **When to re-anchor** — lazily on load/render against the current rev? *(yes; mark outdated
   when the quote moved, preserving the original)*.
5. **Fuzzy algorithm** — exact block-id + quote match first, then a simple similarity fallback
   (normalized substring / ratio threshold)? *(yes; property tests define the bar)*.
6. **Scope trim** — defer **alternatives** (F3.2c) and **Approve gating** (F3.6) to 5b? *(yes —
   keep 5a to comment/suggest/flag + severity + batch + the round-trip)*.

## Heads-up
This is the first slice of the **biggest** milestone and spans three crates. T1 (anchor) is a
real §7 risk with a stop-and-ask; T4 (review UI) is substantial. Expect to checkpoint
mid-execution (likely after T1, and after T3) rather than one straight run.
