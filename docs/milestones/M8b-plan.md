# M8b · Tickets — cards + coverage meter + drift/resync UI — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans; the
> **`plan-ui-design` skill** contract (`.claude/skills/plan-ui-design/plan-ui-design-SKILL.md` +
> `docs/design/plan-ui-compliance.md` **§4 ticket card**, PRD Part IV **§3.2**) — run its **§13
> protocol** under One Dark. GPUI views get smoke coverage; TDD for any pure helper. The `plan-ui-
> design` skill is NOT in the harness registry — read its file directly.

**Status:** COMPLETE — **§13 confirmed by the developer (2026-07-15)**. Closes M8. Second and
final MVP slice of M8 (M8a complete; write-back F2.4e is v1).

## Goal
Surface the M8a ticket state visually: **ticket header chip(s)** + **Spec-lens ticket cards** with
a **coverage meter**, and the **drift card** + **↻ Resync** affordance — reading `plan.tickets` +
`plan_core::tickets::coverage`. This is the §13 visual slice.

**Feature IDs:** F2.4b (ticket rendering + coverage meter), F2.4g (drift card + resync UI). MVP
visuals only.

**Architecture:** `plan_ui` **reads** `plan.tickets` + `tickets::coverage(plan)` and renders; it
does not fetch (the agent's Jira MCP does) and does not mutate tickets. The coverage meter is pure
data from `plan_core::tickets` (M8a). Like revert (M7b), a **manual ↻ resync is agent-routed** —
the button is a visual affordance; the agent re-fetches at the F2.4g checkpoints.

**Tech stack:** Rust + GPUI (`plan_ui`), plus a small pure `plan_core::tickets` drift-payload
enrichment (T2, test-first). Commit prefix `[PLAN-M8b]`. Additive — `FORK_DIFF.md` unchanged.

---

## Tasks

### T1 — Ticket header chip + Spec-lens ticket cards + coverage meter (§4 / §3.2)
`[PLAN-M8b]: add ticket cards and the coverage meter to the Spec lens`
- **Header chip:** a compact `⛓ KEY` chip (mono, accent) per ticket next to the plan title in
  `render_header` (ticketless → none, per §7 "no ticket chips render anywhere").
- **Spec-lens ticket cards** at the top of `render_spec` (before GOAL), one per ticket: panel-bg
  card, row = `⛓ KEY` (mono/accent) · type/priority/source (muted) · **status chip** (To Do
  `text.muted` / In Progress `info` / Done `created`) · right-aligned **coverage meter**.
- **Coverage meter** from `tickets::coverage(plan)`: one 22×6px segment per ticket AC —
  `Covered`→`created`, `NeedsUpdate`→`modified`, `Unmapped`→empty (`element` bg + border) — plus
  the label **"ticket AC n/m covered"** (n = covered count, m = total).
- **Sync stamp** (mono, 9.5px) + **↻** affordance (agent-routed — q1). Reads `ticket.fetched_at`
  if present, else a neutral stamp.
- Colors via `cx.theme()`; purple identity via `syntax()` keyword where needed.
- **Smoke + §13:** a ticketed plan shows the header chip + a Spec-lens card with a meter whose
  segments match coverage (fixture = 4/4 covered → all `created`); a ticketless plan shows none.

### T2 — Drift card + drift-payload enrichment (§4 / §3.2, F2.4g)
`[PLAN-M8b]: add the ticket drift card and enrich the drift payload with old→new`
- **`plan_core` (TDD):** enrich `ticket_drift` + the resync stamp so the drift payload carries the
  **old→new** values the card needs (status old→new; changed/added AC old→new), not just the
  boolean flags. Small, additive; keep the existing flags. Update `plan_server::resync_ticket` to
  stamp the richer payload. Tests over the new payload.
- **Ticket card drift variant:** when `ticket.drift` is present → card border `modified`, the
  changed AC segment(s) `modified`, stamp reads "drift · resynced …".
- **Drift card** (follows the ticket card): amber header `⛓ Ticket drift`, body shows old line
  (struck, `deleted`/error-bg) → new line (`created`-bg) from the enriched payload; actions
  **`Discuss`** (adds a comment, reuse the M5 comment path) and **`Apply rev n`** *only when a
  `pending_revision` exists* (a scope-change staged by the skill — reuse the M5b apply path).
- **Smoke + §13:** set `ticket.drift` → the card gets the modified border + a drift card with
  old→new; clearing drift removes it.

### T3 — Milestone note + §13 + FORK_DIFF
`[PLAN-M8b]: add M8b milestone note`
`docs/milestones/M8b.md`; full `plan_ui` + `zed` build; **§13 visual verification handoff** (the
gate). FORK_DIFF unchanged (additive).

---

## Definition of done
- [ ] Ticket header chip + Spec-lens ticket cards render per §4/§3.2; ticketless shows none.
- [ ] Coverage meter segments + "n/m covered" label match `tickets::coverage`.
- [ ] Drift card + card drift variant per §4; drift payload carries old→new (test-first).
- [ ] Smoke tests + any pure-helper TDD green; clippy clean; full `zed` build; `docs/milestones/M8b.md`.
- [ ] **§13 visual verification under One Dark — developer gate.**

## Open questions (recommendations in parens)
1. **Manual ↻ Resync wiring.** *(The button is a **visual affordance**; actual re-fetch is
   **agent-routed** — the skill resyncs at the F2.4g checkpoints (session start / open / gates /
   Done), same pattern as M7b's agent-routed revert. Wiring a UI→agent manual trigger is deferred;
   flag. The stamp still shows the last `fetched_at`.)*
2. **Drift card old→new source.** *(Enrich the M8a drift payload (T2) to carry old→new for changed
   fields — the flags alone can't render the struck/created-bg lines §4 wants. Small additive
   `plan_core` change, test-first. Alternative — show changed-field names only — is less faithful;
   rejected.)*
3. **`Apply rev n` vs `Discuss`.** *(`Apply` shows only when the skill has staged a
   `pending_revision` for the scope change (reuse M5b apply); an AC-only change re-runs the
   coverage meter and offers `Discuss` (a comment) — turning a scope change into a staged revision
   is skill-driven, not a UI action.)*
4. **Where the cards live.** *(Spec-lens, top of `render_spec` (before GOAL) + a compact header
   chip; matches "header chips + Spec-lens cards" F2.4b.)*

## Scope guard
Write-back (F2.4e — In Progress / Done summary / Abandon reason) is **v1** — flag, don't build.
Peek-on-hover for ticket chips (F0.5b) is v1. Keep the manual-resync trigger agent-routed.

## Heads-up
The task-card ticket chip + the acceptance `ticket_ac` chip already render (M3/M5) — T1 is the
plan-header chip + the Spec-lens **cards + meter**, not those. Most of M8b is faithful rendering of
M8a data; the only new logic is the T2 drift-payload enrichment. Expect a checkpoint after **T1**
(cards + meter visible) before the drift card.
