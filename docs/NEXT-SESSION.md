# Next session — Plan feature handoff

**How to use:** point a fresh agent at this file (or paste the brief below). It's the kickoff
for continuing the **Plan** feature. Kept current at the end of each session.

_Last updated: after M8 (2026-07-15) — tickets complete (8a store/coverage/lint · 8b cards +
coverage meter + drift UI). On branch `plan`. **M9 (settings + hardening) is the last MVP
milestone.**_

---

## Agent brief (paste or reference)

You're continuing the "Plan" feature in a personal Zed fork (Rust + GPUI). All work is on the
`plan` branch. **Read first, in order:** your project memory (auto-loaded: `plan-fork-project`
+ `zed-build-environment` + **`no-coauthor-trailer`**); then `docs/PRD.md` (Part 0 context,
**Part III working agreement — it governs HOW you work**, Part II build plan + milestones, and
for the next section Part I §13 + Appendix B (settings homes + policy) and Part IV §7 settings
page)); then the milestone notes `docs/milestones/M0.md … M8b.md`; then
`docs/zed-notes.md` (Zed internals findings, spike results, deferred backlog — **study #8 =
settings registration: the `"plan"` key + settings page touch ~3 UPSTREAM files, not purely
additive**). Confirm you've read Part III + the milestone notes before proposing anything.

**Current section: M9 (Settings + hardening) is next — the LAST MVP milestone. Start by writing
the M9 plan for sign-off; no code until approved.** Note M9 is the one place the fork touches
upstream beyond registration lines (study #8) — flag the conflict surface. Never add a
`Co-Authored-By` trailer to commits (see the Working agreement).

### Status
M0–M4, **all of M5 (5a comments · 5b staged revisions · 5c lint)**, **all of M6 (6a launch ·
6b gates/guards · 6c amendments/failure-ladder/control)**, **all of M7 (7a git core +
enforcement · 7b branch strip + commit rail)**, and **all of M8 (8a ticket store/coverage/lint ·
8b ticket cards + coverage meter + drift UI)** are **complete and visually verified**. A broad
`plan_ui` **demo-fidelity pass** is in too (card chassis, sechead + WHEN/SHALL acceptance +
preview blocks, task-card checkbox/spinner + chip placement + per-task timeline, continuous
commit rail, panel borders + contextual actions, pill states, first-launch fix). Crates:
- `plan_core` — schema / store / validate / **anchor** / **comments** / **rev** / **lint**
  (incl. **git-commit-format** + **ticket-coverage**) / **exec** (launch + lease + guard/gate
  lifecycle + `GuardPolicy` + pause/resume/stop + amendments + failure ladder + recovery) / **git**
  (policy + branch/base/trailer/commit-format/launch-guard/destructive helpers, pure) / **tickets**
  (coverage + drift old→new + `coverage_blocks_done`, pure).
- `plan_server` — Rust `rmcp` MCP server + Claude Code hooks; reuses `plan_core`. Tools: review +
  `plan_propose_revision` + `plan_lint` + `plan_launch` (now branch-guarded, stamps branch) +
  `plan_set_branch` + `plan_record_commit` + `plan_set_tickets` + `plan_add_acceptance` +
  `plan_resync_ticket` (Done gated on ticket coverage) + guard/gate
  (`plan_hold_guard`/`plan_clear_guard`/`plan_approve_gate`) + control
  (`plan_pause`/`plan_resume`/`plan_stop`/`plan_propose_amendment`/`plan_recover_task`);
  **PreToolUse gate** (no-plan-edit + guard/gate holds + `require_on` + **commit-format/trailer +
  destructive-op block**) + **Stop loop guard**; `plan_server::git` shells `git` (dirty/behind).
- `plan_ui` — Plan tab (lenses, cards, task cards w/ commit rail + timeline, review UI, launch +
  guards + amendments + escalation + recovery + pause/resume/stop, **branch strip + commit rail
  from live `git_store` + GitHub PR button**, **ticket header chip + Spec-lens ticket cards +
  coverage meter + drift card**) + status pill + dock panel.
- `plan-agent/` — planning skill (lifecycle + per-task loop + enforcement + **branch/commit git
  protocol**) + hooks + settings.

Upstream footprint is still only the M0 registration lines + the M4 pill line — all tracked in
`FORK_DIFF.md` (M7 added **no** upstream touch; git state is read via already-`pub`
`project::git_store`).

### Working agreement (PRD Part III — follow it)
One milestone at a time. Before coding, write a short plan
(`docs/milestones/M<n>-plan.md`: tasks, files, tests, feature IDs, open questions) and get the
user's sign-off; then execute task-by-task. **Test-first** for `plan_core`/`plan_server` logic;
GPUI views get smoke coverage + a **§13 visual verification that only the user can do** (run
under One Dark and eyeball it) — build up the code, then hand the visual check to them. One
commit per task, imperative, prefixed `[PLAN-M<n>]`. **Do NOT add a `Co-Authored-By` trailer
(or any co-author line) to commits** — the user directed this and the `plan` history was
rewritten to strip it.
**Stop and ask** when a spike fails, an upstream API isn't as the notes assumed, or the design
docs are silent/contradictory. All `plan_ui` work is governed by the `plan-ui-design` skill +
`docs/design/plan-ui-compliance.md` (run its §13 protocol per component). Stay in MVP scope
(PRD §14); flag v1 pulls instead of building them.

### Next up
M5–M8 are done. **Last MVP section:**
- **M9 — Settings + hardening.** The `"plan"` settings key + settings-page section (F12.1/F12.2c)
  with presets (Careful/Balanced/Fast), policy.json severity wiring, and the **§12 failure drills**
  (kill the server mid-write · kill Zed mid-execution · corrupt the file · edit plan.json
  externally — each must land in a designed state, not a surprise). **Fork caveat (zed-notes study
  #8):** the settings key + page touch **~3 upstream files** (`settings_content`, `assets/settings/
  default.json`, `settings_ui/page_data.rs`) — the one non-additive milestone; flag + track the
  conflict surface in FORK_DIFF. Interim fallback exists (raw-JSON settings work today), so the
  structured page can degrade/defer if needed.
- After M9, **MVP is complete**; then the v1 tail (peek cards → rehearsal → live evidence + diff
  view → PR generation → worktree isolation → attention queue → …).

**Start by writing the M9 plan for the user's sign-off — no code until approved.** M9 likely splits
(settings key + `from_settings` wiring · settings-page section + presets · §12 hardening drills).
The settings-page slice is where the upstream touch lands — call it out.

### M8 deferrals to honor (recorded in M8a/M8b notes)
- **Write-back (F2.4e) is v1** — Launch→In Progress, Done→summary+PR comment, Abandon→reason.
- **Attach-later commit retag via rebase (F2.4d)** — the history rewrite rides the amendment flow;
  M8 does the ticketless↔ticketed *state* change only.
- **`plan_add_acceptance` is add-only** — edit/remove of criteria deferred.
- **Drift card is informational** — no Discuss/Apply buttons (`render_spec` is a free fn; applying a
  scope change goes through the global staged-revision card); manual **↻ resync is agent-routed**.
- **No invented timestamps** — `set_tickets`/`resync` don't stamp real `fetched_at` (crate is
  time-free); the agent supplies it.

### M7 deferrals to honor (recorded in M7a/M7b notes)
- **revert-task-commit** — omitted by decision: a future UI revert **routes through the agent**
  (it runs `git revert`), never `plan_ui` shelling git or upstream git additions. No `git revert`-
  a-commit API exists in the git surface.
- **Lease enforcement/reclaim (F11.3b)** — still deferred (the PreToolUse hook can't identify the
  acting ACP session id); unchanged since M6c.
- **`require_tests_green`** — enforced by the **skill**, not the commit hook (the hook sees the
  command, not the result). The commit hook enforces format/trailer + destructive-op block.
- **base feature-vs-hotfix** — `base_for` always uses the feature base (no plan-`type` signal yet).
- **Live-evidence diff view (F4.6d, v1)** — click a task/sha → git diff in a read-only mini-buffer
  (`load_commit(sha)` + `create_editor_diff`, both ready). Developer asked; kept in the **v1 tail**
  (not MVP). Diffstat popover vs full mini-buffer TBD when pulled.
- **Amendment rail node** — a purple **square**, not a 45°-rotated diamond + branch curve (GPUI
  rotates only svg/img). **PR chip** links to a recorded `git.pr.url`; PR *population* is v1/F10.4.

### M6 deferrals to honor (recorded in M6a/M6b/M6c notes)
- **Lease enforcement + reclaim** (F11.3b) — lease is set on launch but the hook only checks
  `status == executing`; leaseholder-only editing + stale-lease reclaim → M7-ish.
- **ExitPlanMode ↔ `plan_launch`** ACP reconciliation is skill-driven (a seam, like thread-id).
- **✋ input free-text field**, agent-death detection (auto `interrupted`), escalation
  descope-waiver / guide-me, and staged-revision/amendment **mini-buffers** → fidelity/later.
- **Fidelity backlog:** contract `input→output` grids + ui-states galleries as mini-buffers;
  rail spine currently full-height (spans slightly beyond first/last node).

### M6a seams to honor (documented, non-blocking)
- **Git → M7:** Launch doesn't create the branch/worktree yet; it launches in the current tree.
- **ExitPlanMode seam:** the agent's ACP ExitPlanMode ↔ `plan_launch` reconciliation is handled
  by the skill for now (like the thread-id seam); deeper ACP wiring is later.
- **Lease enforcement → M6c:** the lease is *set* on launch; the PreToolUse gate still only checks
  `status == executing` (leaseholder-only editing + stale-lease reclaim are M6c).
- **No UI task-advance:** with no live agent, nothing moves a task to `in_progress` (agent-driven
  via `task_update`) — a demo/testing limitation, not a gap.

### Deferred backlog from M5 (recorded in the milestone notes)
- **M5b:** staged-revision **mini-buffers** (real editor-diff vs the current GPUI chrome),
  **pushback UI** (F3.4c), inline **Δ chips** (F3.5), **acceptance-criterion revision targets**
  (`set_block_text` writes task/step only), and the **auto-resolve question** (should applying a
  hunk `from` a blocker auto-`resolve` it rather than mark `addressed`?).
- **M5c:** the dedicated **§6 Lint card** (auto-fixed dimming, inline fix buttons) + **Spec-lens
  finding rendering**; **`prod-requires-gate`** (needs a prod signal — revisit in M6/M7); a lint
  **waiver** path (F2.4f); and batch-send should **skip `plan-lint` comments** (currently sweeps
  them `open → sent`).

### Build / run
Every build needs:
```sh
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
```
(cargo via rustup; `cmake` is in `~/.local`; full Xcode's `metal` compiler is required for
`gpui_macos`). Fast loops: `cargo build -p plan_ui` (~seconds), `cargo test -p plan_core -p
plan_server`. A full `cargo build -p zed` is only needed for `crates/zed/**` changes and before
a visual check. The feature is gated behind `ZED_PLAN`; run with `ZED_PLAN=1 cargo run -p zed
-- .` (a throwaway `.plans/led-212.plan.json` test plan is present; a single-plan fallback
shows it). **Do NOT commit** the pre-existing `.rules` / `.claude/` changes — not part of this
work.

### Decisions to honor
- The MCP server is **Rust** (`rmcp`) reusing `plan_core` — one source of truth, no
  schema/lint duplication.
- The **UI writes `plan.json` directly** via `plan_core`; the **server writes for the agent**.
- **Thread-following keys on the ACP session id** (`active_agent_thread().session_id()`), not
  `agent_ui`'s UUID `ThreadId`. Known seam: the agent must stamp `plan.thread` with the session
  id for true per-thread binding (a single-plan fallback covers it meanwhile).
- All colors via `cx.theme()`. Feature flag is the `ZED_PLAN` env var (the durable `"plan"`
  setting is M9).

### Deferred backlog (recorded, non-blocking)
Thread-id seam · fidelity pass (motion/pulses, mono fonts, read-only mini-buffers / text-range
selection, free-text comment composer, floating selection toolbar, full §10 pipeline chrome,
`fs().watch()` + activity event-subscription vs polling, rev-behind receipt) · alternatives
(F3.2c) · threads-sidebar plan state (F1.5, needs `agent_ui` changes) · step-level comment
rendering.

---

_When you finish a milestone, update this file's Status + Next-up + Last-updated line so the
next session stays accurate._
