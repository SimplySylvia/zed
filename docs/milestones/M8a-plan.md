# M8a · Tickets — store + coverage + `ticket-coverage` lint — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the coverage/drift helpers, the lint rule, and the
> server tools are test-first over the LED-212 fixture). M8a is **no UI**, so the
> `plan-ui-design` skill does not apply until M8b.

**Status:** awaiting sign-off (not started).

## M8 is split (like M5/M6/M7) — this plan is slice **a** of two
- **M8a (this) — Ticket store + coverage + lint (core + server, no UI).** The agent-fed ticket
  store (`plan_set_tickets` — the server **never calls Jira**; the skill's Jira MCP fetches, the
  server just stores), acceptance authoring (`plan_add_acceptance` — needed so drafted criteria
  can carry `ticket_ac`), the pure **coverage** + **drift-compare** helpers in `plan_core`, the
  **`ticket-coverage` lint rule** wired into `plan_core::lint` (retires the M8 no-op), and the
  **Done coverage hard-check** (F2.4f). *Demo: in plain Claude Code, seed tickets + acceptance,
  `plan_lint` flags uncovered ticket ACs, and `set_status → done` is refused until every ticket
  AC is covered or waived.*
- **M8b — ticket cards + coverage meter + drift/resync UI** (F2.4b/g §3.2): ticket header chips +
  Spec-lens ticket cards (type/status/priority/assignee/link) + the **coverage meter**, the
  **drift card** + **↻ Resync**, and the skill's plan-from-ticket orchestration + resync
  checkpoints. GPUI + §13. Sketched below.

**v1 — explicitly OUT of MVP M8 (flag, don't build):** **write-back** (F2.4e — Launch→In Progress,
Done→summary+PR comment, Abandon→reason) is v1. Attach-later **commit retag via rebase** (part of
F2.4d) rides the amendment/destructive flow — deferred; M8 does the ticketless→ticketed *state*
change, not the history rewrite.

**Feature IDs (M8a):** F2.4 (plan-from-ticket store), F2.4c (task↔ticket — the M7 trailer/branch
helpers already key on `task.ticket`), F2.4d (ticketless degrade — mostly done in `plan_core::git`),
F2.4f (ticket coverage: draft-time lint + Done hard-check), F2.4g (drift *detection* — the pure
compare; the card + resync UI is M8b).

**Goal:** the ticket contract is enforced without UI. The agent stores tickets it fetched, authors
acceptance criteria that map to ticket ACs, `ticket-coverage` lint flags any uncovered ticket AC at
draft, drift is detectable by comparing a fresh fetch to the stored snapshot, and a plan can't reach
`done` with uncovered, unwaived ticket ACs.

**Architecture:** unchanged. `plan_core` stays pure (coverage/drift are facts-in helpers). The
**skill orchestrates Jira** via its own MCP and feeds the server; **`plan_server` only stores**
(no Jira dependency). The UI (M8b) reads + renders.

**Tech stack:** Rust — `plan_core` (coverage/drift/lint), `plan_server` (ticket + acceptance tools,
Done gate), `plan-agent` (skill). Pure logic test-first. Commit prefix `[PLAN-M8a]`.

---

## Tasks

### T1 — `plan_core` coverage + drift helpers + `ticket-coverage` lint (TDD)
`[PLAN-M8a]: add ticket coverage and drift helpers and wire the ticket-coverage lint`
New `crates/plan_core/src/tickets.rs` (pure), plus a lint rule:
- `ticket_ac_id(key, index_1based) -> String` → `"LED-212#1"` (Appendix A shape).
- `uncovered_ticket_acs(plan) -> Vec<{ ticket_key, index, text }>` — a ticket AC is *covered* when
  some `spec.acceptance[].ticket_ac == ticket_ac_id(key, i)`. Ticketless plans → empty (F2.4d).
- `coverage(plan) -> Vec<{ ticket_key, index, text, state }>` where state ∈
  `covered | needs_update | unmapped` — the data the M8b meter renders (needs_update = the mapped
  criterion is flagged by drift; unmapped = uncovered).
- `ticket_drift(stored: &Ticket, fresh: &Ticket) -> Option<TicketDrift>` — compares
  status/priority/assignee/`ac`; an **AC change** is the one that must re-run coverage, a **scope
  change** is the one M8b turns into a staged revision. Pure; the agent supplies `fresh`.
- `coverage_blocks_done(plan, severity) -> Option<String>` — uncovered, **unwaived** ticket ACs
  (respecting `acceptance[].waived`) block Done (F2.4f).
- Lint: `ticket-coverage` rule over `uncovered_ticket_acs` → one finding per uncovered AC (severity
  from the existing `policy.ticket_coverage`, default blocker). Retire the `lint.rs` no-op.
- **Fixture integration point (flagged):** the LED-212 fixture maps only 2 of 4 ticket ACs
  (`#3`/`#4` uncovered), so `ticket-coverage` (default blocker) will fire on it and break
  `clean_fixture_has_no_findings`. **Fix in T1:** map the fixture complete — add acceptance `a3`→
  `LED-212#3` and `a4`→`LED-212#4` (linked to existing tasks) so it stays the clean/complete
  baseline; re-run the whole `plan_core` suite to catch any acceptance-count ripple.
- **Tests:** covered/uncovered detection; ticketless → no findings; drift on AC vs status;
  `coverage_blocks_done` fires on uncovered / passes when covered / passes when waived; the clean
  (now fully-mapped) fixture yields no ticket-coverage findings.

### T2 — `plan_server` ticket + acceptance tools + Done coverage gate (TDD)
`[PLAN-M8a]: add ticket store, acceptance authoring, resync, and the Done coverage gate`
- `plan_set_tickets(id, tickets)` — replace `tickets[]` from agent-fetched JSON (plan-from-ticket
  seed). Stamps `fetched_at` server-side.
- `plan_add_acceptance(id, ac_id, when, shall, ticket_ac?, tasks?)` — **new**: author a spec
  acceptance criterion (there is no acceptance-writing tool today; coverage needs one so drafted
  criteria can carry `ticket_ac`). One write, rev-bump + history like the other mutators.
- `plan_resync_ticket(id, key, fresh)` — update one ticket from a fresh fetch; run
  `ticket_drift`; when drift is found, stamp `ticket.drift` (the M8b card reads it) and refresh
  `fetched_at`. (Turning a scope-change into a staged revision is M8b/skill.)
- Done gate: extend `tools::set_status` so `→ done` runs `coverage_blocks_done` and bails with the
  reason when it fires (F2.4f). Pure check from T1; the UI can reuse it.
- **Tests:** set_tickets/add_acceptance persist; resync stamps drift on an AC change; set_status→
  done blocked when a ticket AC is uncovered, allowed when covered or waived; one end-to-end MCP call.

### T3 — skill + policy (plan-agent)
`[PLAN-M8a]: teach the planning skill the ticket + coverage protocol`
- SKILL.md: **plan-from-ticket** ("plan LED-212" → fetch via the agent's own Jira MCP →
  `plan_set_tickets` → seed `spec.goal` + draft acceptance via `plan_add_acceptance` **carrying
  `ticket_ac`** so coverage is satisfied by construction); coverage awareness (uncovered ACs are
  lint blockers — cover or record a waiver, never silently drop — "descoping is a decision, not an
  omission", F2.4f); **resync checkpoints** (session start / plan open / before gates / before Done
  → `plan_resync_ticket`). The Jira MCP itself is the agent's, not the plan server's.
- `.plans/policy.json` already carries `tickets: { coverage: "strict", write_back: true }`.

### T4 — milestone note + FORK_DIFF + tests green
`[PLAN-M8a]: add M8a milestone note`
`docs/milestones/M8a.md`; FORK_DIFF expected unchanged (additive: `plan_core::tickets` + lint,
`plan_server` tools, skill); `cargo test -p plan_core -p plan_server` green; full `zed` build
sanity. No §13 (no UI).

---

## M8b sketch (separate sign-off after M8a)
- **Ticket header chips** + **Spec-lens ticket cards** (§3.2): `⛓ KEY` mono/accent · type/priority/
  source · status chip (To Do muted / In Progress info / Done success) · **coverage meter**
  (22×6px segments per ticket AC: covered=created / needs-update=modified / unmapped=empty +
  "ticket AC n/m covered") · sync stamp `synced 2m · ↻`.
- **Drift card** (§3.2): amber header `⛓ Ticket drift · edited by @who`, old line (struck) → new
  (created-bg), `Apply rev n` / `Discuss`; AC-change re-runs the coverage meter; scope-change
  arrives as a staged revision (reuse M5b).
- **↻ Resync** action on the card/meter → the skill re-fetches (manual F2.4g).
- Note + §13 under One Dark (§3.2 ticket card + coverage meter + drift states).

## Scope guard (don't pull v1 into M8)
Write-back (F2.4e) and attach-later commit **retag via rebase** (F2.4d's history rewrite) are v1 —
flag, don't build. M8 does ticket store, coverage, drift detection, and the ticketless↔ticketed
*state* change only.

## Definition of done (M8a)
- [ ] `plan_core` coverage + drift + `coverage_blocks_done` helpers correct + test-first; fixture
      mapped complete; whole `plan_core` suite green.
- [ ] `ticket-coverage` lint wired in (no-op retired); test-first.
- [ ] `plan_server`: `plan_set_tickets` + `plan_add_acceptance` + `plan_resync_ticket` + the Done
      coverage gate; one end-to-end MCP call.
- [ ] Skill teaches plan-from-ticket + coverage + resync; policy tickets block present.
- [ ] `cargo test -p plan_core -p plan_server` green; clippy clean; `docs/milestones/M8a.md`;
      FORK_DIFF unchanged.

## Open questions (recommendations in parens)
1. **Ticket AC identity.** *(`"KEY#n"`, 1-based index into `tickets[key].ac`, matching Appendix A's
   `"LED-212#1"`. `ac` is free text, so identity is positional — a reordered ticket AC list reads as
   drift, which is acceptable and even desirable.)*
2. **Where's the Done gate enforced?** *(In `tools::set_status` (agent path) via the pure
   `coverage_blocks_done`. The UI (M8b) reuses the same helper before any Done affordance. The
   PreToolUse hook is not involved — Done is a status transition, not a tool/edit.)*
3. **Fixture completion vs a separate fixture.** *(Map the existing LED-212 fixture complete (add
   `a3`/`a4`) so it stays the single clean baseline — simpler than a second fixture, and a complete
   plan *should* cover all ticket ACs. Watch for acceptance-count ripple in other tests.)*
4. **`needs_update` coverage state.** *(A mapped criterion whose ticket AC changed under drift =
   `needs_update` (amber segment, F2.4b). Computed from `ticket.drift`; until M8b wires drift, all
   mapped ACs read `covered`. Fine — the state exists in the helper now, the meter consumes it in
   M8b.)*

## Heads-up
- **No acceptance-authoring tool exists today** — T2's `plan_add_acceptance` is the enabler for the
  whole coverage story (drafted criteria must be able to carry `ticket_ac`). Flag if the drafting
  surface needs more than add (edit/remove) — MVP is add-only.
- Coverage/drift are the M8 substance; the Jira integration is entirely the **agent's** (its Jira
  MCP), so there's no network/tracker code in our crates — keep it that way (F11.6 degrade: Jira
  down is the skill's problem, surfaced as stale cards in M8b). Expect a checkpoint after **T2**.
