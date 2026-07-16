# M8a · Tickets — store + coverage + `ticket-coverage` lint — milestone note

**Status:** COMPLETE (core + enforcement, **no UI**). `plan_core` + `plan_server` + `plan_ui`
build green; full `zed` build green; **no §13** (M8a ships no plan_ui — the visual check lands
with M8b). All logic test-first.

**Feature IDs:** F2.4 (plan-from-ticket store), F2.4c (task↔ticket — the M7 trailer/branch helpers
already key on `task.ticket`), F2.4d (ticketless degrade), F2.4f (coverage: draft-time lint + Done
hard-check), F2.4g (drift *detection*; the card + resync UI is M8b).

## What works (built)

- **`plan_core::tickets`** (new, pure): `ticket_ac_id` (`"KEY#n"`, 1-based), `coverage` →
  `CoverageState { Covered | NeedsUpdate | Unmapped }` per ticket AC (the M8b meter data;
  `NeedsUpdate` = the ticket carries drift), `uncovered_ticket_acs`, `ticket_drift(stored, fresh)`
  → `TicketDrift { status_changed, ac_changed, fields_changed }`, and `coverage_blocks_done`
  (blocks Done only when the rule is a blocker; a mapped-but-`waived` criterion counts as a
  recorded decision, so only true omissions block). **7 tests.**
- **`plan_core::lint`** — the `ticket-coverage` rule (retires the M8 no-op): one finding per
  uncovered ticket AC, severity from `policy.ticket_coverage` (default blocker), anchored to the
  `"KEY#n"` id. **+1 test** (19 lint total).
- **Fixture completed** — the LED-212 fixture mapped only 2 of 4 ticket ACs, so a default-blocker
  `ticket-coverage` would have fired on the clean baseline. Added `a3`→`LED-212#3` and `a4`→
  `LED-212#4` (linked to `t2`); bumped the `schema_roundtrip` acceptance-count assertion 2→4. Whole
  `plan_core` suite green.
- **`plan_server`** — `plan_set_tickets` (store agent-fetched `tickets[]`; the server never talks
  to a tracker), **`plan_add_acceptance`** (the first tool that writes `spec.acceptance` — the
  enabler for coverage; carries `ticket_ac`), `plan_resync_ticket` (update the snapshot + stamp
  `ticket.drift` on change), and the **Done coverage gate** in `set_status` (`→ done` bails when a
  ticket AC is uncovered). **5 tests** incl. one end-to-end MCP call.
- **`plan-agent` SKILL.md** — a Tickets & coverage section: fetch via the agent's **own Jira MCP**
  → `plan_set_tickets` → seed goal → `plan_add_acceptance` per ticket AC (carrying `ticket_ac`);
  coverage is enforced (uncovered = blocker + blocks Done; descope = a recorded waiver, never an
  omission); resync at the F2.4g checkpoints via `plan_resync_ticket`.

## The round-trip
"plan LED-212" → the agent fetches via its Jira MCP → `plan_set_tickets` → seeds the goal and
authors one `plan_add_acceptance` per ticket AC with `ticket_ac`. `plan_lint` flags any uncovered
ticket AC (blocker); `set_status → done` is refused until every ticket AC is covered or waived.
At the F2.4g checkpoints the agent re-fetches and `plan_resync_ticket` stamps drift when the ticket
changed — which M8b will surface as a drift card + coverage-meter update.

## Decisions recorded
- **Ticket AC identity is positional** — `"KEY#n"`, 1-based into `tickets[key].ac`. Reordering a
  ticket's ACs reads as drift (desirable).
- **Coverage = mapped, not done/waived-specific** — a ticket AC is covered as soon as some
  criterion references it; a `waived` criterion still counts (descope is a recorded decision).
  Only an *unmapped* ticket AC is uncovered → lint blocker + Done block.
- **Done gate uses the `ticket-coverage` lint severity** — blocks only when blocker; enforced in
  `tools::set_status` (agent path). The UI (M8b) reuses the same pure `coverage_blocks_done`.
- **`plan_add_acceptance` is add-only** — edit/remove of acceptance criteria deferred (not needed
  for coverage; flag if the drafting surface needs it).
- **No timestamps invented** — `set_tickets`/`resync` don't stamp a real `fetched_at` (the crate
  stays time-free, matching `history.at = None`); the agent can include `fetched_at` in the ticket
  JSON. (M8b sync stamp reads what's there.)

## Deferred (honored)
- **M8b (next):** ticket header chips + Spec-lens cards + coverage meter (F2.4b), drift card + ↻
  Resync (F2.4g UI), skill plan-from-ticket orchestration surfacing.
- **v1 (out of MVP M8):** write-back (F2.4e — In Progress / Done summary / Abandon reason);
  attach-later commit **retag via rebase** (F2.4d's history rewrite — rides the amendment flow).

## Fork discipline
Additive only — new `crates/plan_core/src/tickets.rs`; extensions to `plan_core::lint`,
`plan_server::{tools,plan_server}`; the fixture + a test-count assertion; skill prose. **No
upstream files touched — `FORK_DIFF.md` unchanged.**

## Definition of done
- [x] `plan_core` coverage + drift + `coverage_blocks_done` correct + test-first; fixture mapped
      complete; whole `plan_core` suite green.
- [x] `ticket-coverage` lint wired in (no-op retired); test-first.
- [x] `plan_server`: `plan_set_tickets` + `plan_add_acceptance` + `plan_resync_ticket` + Done
      coverage gate; one end-to-end MCP call.
- [x] Skill teaches plan-from-ticket + coverage + resync.
- [x] `cargo test -p plan_core -p plan_server` green; clippy clean; full `zed` build green;
      `docs/milestones/M8a.md`; FORK_DIFF unchanged.

## Next: M8b — ticket cards + coverage meter + drift/resync UI (§13 lands there).
