# M6c · Execution — amendments + failure ladder + control — milestone note

**Status:** COMPLETE. `plan_core` + `plan_server` + `plan_ui` + full `zed` build green;
**§13 visual verification under One Dark confirmed by the developer (2026-07-15)** — amendment
card + Accept/Reject, escalation, recovery, and pause/resume/stop all verified. **This closes M6
(execution).**

### UI fidelity follow-ups (verified with M6c, committed `[PLAN]`)
A run of demo-fidelity work landed alongside M6c, all `plan_ui`-only + additive:
- **First-launch fix** — the Plan tab no longer renders empty when the AgentPanel isn't ready yet
  (resolves via the sole-plan fallback + always polls; binds to the panel late).
- **Card chassis** (`card_shell`/`card_row`) — the amendment/staged-rev, gate, and escalation cards
  share the demo `.card` chassis (clipped, bold caps header band, hairline rows); primaries use the
  accent-tinted button style.
- **Document structure** — `sechead` (caps + hairline) across Spec/Design/Tasks; acceptance rows as
  WHEN(teal)/SHALL(purple) + ticket ref + evidence; Design contracts as dashed `◈` preview blocks.
- **Task cards** — §7 checkbox vocabulary with an accent **spinner** for in-progress; task number +
  right-aligned chips (ticket · system · guard · sha · tests); per-task **timeline** rows.
- **Commit rail** (§9.1) — one continuous left spine with per-task status nodes, from launch on.
- **Panel** (§4) — header/column **borders**, contextual actions (Pause/Resume/Stop/Approve
  gate/Record input), pipeline row tints + mono notes + spinner, live-column verb+detail rows.
- **Pill** — hover + solid selected state + drop shadow; open/close toggle fix.

**Feature IDs:** F4.7 (amendments), F11.4 (failure ladder), F5.1 (pause/resume/stop), F5.6 (kill),
F11.1 (interrupted-task recovery, lean), F6.3 (Stop loop guard, lean).

## What works (built)

- **`plan_core::exec`** (shared): `pause`/`resume`/`stop` (stop releases the lease; kill == stop),
  `propose_amendment` (reuses `rev::stage_revision`, tags `pending_revision.kind = "amendment"`,
  records a per-task history entry), `amendment_count`/`needs_escalation` (≥2), and lean recovery
  `mark_interrupted` + `recover_task` (resume/redo/manual). **5 tests.**
- **`plan_server`** — tools `plan_pause`/`plan_resume`/`plan_stop`, `plan_propose_amendment`,
  `plan_recover_task`; the **Stop loop guard** (`hooks::stop_loop_guard`): while `executing` with
  unfinished, unheld tasks it nudges the agent to continue, with Claude Code's `stop_hook_active`
  flag as the runaway guard. **5 tests** incl. one end-to-end MCP call.
- **Plan tab** — the staged-revision card renders as an **err amendment card** when tagged (F4.7);
  a **failure-ladder escalation card** appears when a task has ≥2 amendments (Take over manually /
  Retry, F11.4); an **interrupted task** shows a recovery row (Resume / Redo / Keep manual, F11.1);
  the toolbar gains **⏸ Pause / ▶ Resume** primaries and a **⏹ Stop** action (F5.1/F5.6).

## The round-trip
A task fails → the agent proposes an **amendment** (`plan_propose_amendment`, same staged-diff
primitive as M5b) → the tab shows the err amendment card → the user Accepts/Rejects per hunk
(reusing the M5b apply path, one rev bump on resolve). A second amendment on the same task raises
the **escalation card**. The developer can **Pause / Resume / Stop** from the toolbar; **Stop**
releases the lease. An **interrupted** task offers recovery. The Stop hook nudges the agent to
finish remaining tasks rather than stop early.

## §13 record — decisions & deviations
**Decisions (approved at sign-off):**
- **Q1 amendment marker:** tag `pending_revision.extra.kind = "amendment"` → the UI picks the err
  amendment card vs the info review card.
- **Q2 ladder counter:** count history entries (`kind:"amendment"`, `"amendment proposed for
  <task>"`) — no schema change.
- **Q3 recovery lean:** recovery **card + transitions** built; agent-death **detection** deferred
  (the agent/skill marks a task `interrupted` for now).
- **Q4 Stop guard lean:** nudge-continue with `stop_hook_active` runaway protection; full
  stuck-detection (F11.4b) is v1.
- **Q6 kill == stop** (hard halt → paused + lease released, resumable).

**Deviations (flagged, deferred):**
- **Lease enforcement + reclaim (F11.3b) deferred (Q5)** — the PreToolUse hook can't reliably
  identify the acting ACP session id, so leaseholder-only editing + stale-lease reclaim wait for
  ACP session context. The lease is set (M6a) and released on stop.
- **Escalation "descope+waiver" / "guide me"** — the card wires Take over manually + Retry; the
  waiver + guide-me options are agent-side / later.
- **Amendment card = GPUI chrome** (like M5b staged revisions); real editor-diff mini-buffers are
  the fidelity pass.
- **Agent-death detection** (the automatic path to `interrupted`) — deferred; needs ACP disconnect
  wiring.

**§13 protocol (developer, under One Dark):**
1. On an executing plan, the toolbar shows **⏸ Pause** + **⏹ Stop**; Pause → paused + **▶ Resume**;
   Stop → paused (lease released).
2. Give `pending_revision` an `"extra": { "kind": "amendment" }` (or call
   `plan_propose_amendment`) → the card renders with the **err amendment header**; Apply/Reject work.
3. Add two `history` entries `kind:"amendment"`, `summary:"amendment proposed for t1"` → the
   **escalation card** appears (Take over manually / Retry).
4. Set a task `"status":"interrupted"` → a **recovery row** (Resume / Redo / Keep manual) appears.

## Fork discipline
Additive only. `plan_core::exec` gained control + amendment + recovery helpers; `plan_server`
gained five tools + the Stop guard; `plan_ui` gained the amendment/escalation/recovery cards +
controls. **No upstream files touched** — `FORK_DIFF.md` unchanged.

## Definition of done
- [x] `plan_core::exec` control + amendment + recovery correct, test-first.
- [x] Tools + Stop loop guard present; one end-to-end MCP call.
- [x] Tab: amendment card, escalation at 2, recovery on interrupted, Pause/Resume/Stop controls.
- [x] Builds green; clippy clean; pure logic test-first; `docs/milestones/M6c.md`; FORK_DIFF unchanged.
- [x] **§13 visual verification under One Dark** — confirmed by the developer (2026-07-15).

## Next: M7 — Git (M6 execution is complete)
Branch on launch + guards for dirty tree/stale base (F10.2), commit-per-task + trailer via the
skill + commit-time hook (F10.1/F10.3), the branch strip + commit rail UI reading repo state
(zed-notes study #7 — git state is observable via `project::git_store`),
`destructive_ops: amendment_only` (F10.5c), revert-task-commit. Also picks up the deferred
lease enforcement/reclaim (F11.3b) if ACP session context lands. Then M8 tickets · M9 settings.
