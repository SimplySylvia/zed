# M8b · Tickets — cards + coverage meter + drift/resync UI — milestone note

**Status:** code COMPLETE; **§13 visual verification awaiting the developer** (One Dark). `plan_ui`
tests + clippy green; full `zed` build green. This closes MVP tickets (write-back F2.4e is v1).

**Feature IDs:** F2.4b (ticket header chip + Spec-lens cards + coverage meter), F2.4g (drift card).

## What works (built)

- **Ticket header chips (T1)** — a compact `⛓ KEY` mono/accent chip per ticket next to the plan
  title (ticketless plans render none — compliance §7).
- **Spec-lens ticket cards (T1, §4/§3.2)** — a `TICKETS` sechead + one panel-bg card per ticket:
  `⛓ KEY` · type/priority/source · **status chip** (To Do `text.muted` / In Progress `info` /
  Done `created`) · right-aligned **coverage meter** · sync stamp + ↻.
- **Coverage meter (T1)** — one 22×6px segment per ticket AC from `plan_core::tickets::coverage`:
  covered=`created`, needs-update=`modified`, unmapped=empty (bordered) + "ticket AC n/m covered".
- **Drift card + variant (T2, §4)** — a resynced ticket that changed carries `drift`
  (`tickets::stamped_drift`): its card border turns `modified`, its mapped segments read
  needs-update (amber), and a **drift card** follows — amber `⛓ TICKET DRIFT` header + **old→new**
  lines (old struck on deleted-bg → new on created-bg) for the changed status / ACs.
- **`plan_core` drift enrichment (T2, TDD)** — `TicketDrift` now carries `status_from/to` +
  `ac_changes: [{from, to}]` (positional edit/add/remove), serialized onto `ticket.drift`;
  `stamped_drift` parses it back (keeps `serde_json` out of `plan_ui`). `plan_server::resync_ticket`
  stamps the full payload. **+1 `plan_core` test** (8 tickets total).

## §13 record — decisions & deviations
**Decisions (approved):**
- **↻ Resync is agent-routed** — the affordance is visual; the agent re-fetches at the F2.4g
  checkpoints (session start / open / gates / Done). Same pattern as M7b's agent-routed revert.
- **Cards live in the Spec lens** (top, before GOAL) + a compact header chip (F2.4b).

**Deviations (flagged, recorded):**
- **Drift card is informational — no `Discuss`/`Apply` buttons.** `render_spec` is a free fn (no
  entity listeners), and *applying* a scope change already goes through the global **staged-
  revision card** (M5b Apply/Reject). `Discuss` (add a comment from the card) is deferred. The
  drift card conveys what changed (old→new) + the modified border; the coverage meter re-colors.
- **PR/peek-on-hover** for ticket chips (F0.5b) is v1 — not built.

**§13 protocol (developer, under One Dark):** with a ticketed plan (the local `.plans/led-212`
fixture is 4/4 covered) — (1) the header shows `⛓ LED-212`; (2) the Spec lens shows a ticket card
with type/status + a coverage meter (all segments `created`, "ticket AC 4/4 covered"); (3) set a
ticket's `drift` (e.g. via `plan_resync_ticket` with a changed AC) → the card border turns amber,
the changed segment reads amber, and a drift card shows old→new; (4) a ticketless plan shows no
chips/cards.

## Fork discipline
Additive — `plan_ui` rendering + a pure `plan_core::tickets` enrichment (`AcChange`, drift
old→new, `stamped_drift`). **No upstream files touched — `FORK_DIFF.md` unchanged.**

## Definition of done
- [x] Ticket header chip + Spec-lens cards per §4/§3.2; ticketless shows none.
- [x] Coverage meter segments + "n/m covered" match `tickets::coverage`.
- [x] Drift card + card drift variant per §4; drift payload carries old→new (test-first).
- [x] `plan_ui` tests + clippy green; full `zed` build; `docs/milestones/M8b.md`; FORK_DIFF unchanged.
- [ ] **§13 visual verification under One Dark — developer gate (pending).**

## Next (after §13): M8 complete → M9 (settings + hardening) = the last MVP milestone.
