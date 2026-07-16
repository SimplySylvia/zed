# Next session — Plan feature handoff

**How to use:** point a fresh agent at this file (or paste the brief below). It's the kickoff
for continuing the **Plan** feature. Kept current at the end of each session.

_Last updated: after **fidelity-pass-2** (2026-07-16) — MVP (M0–M9) complete, and now a full
**plan_ui polish pass toward the mockup** is done + §13-verified: P1 chassis/tokens (filled-pill
chips, solid-accent CTA, segmented lens switcher w/ lighter active fill, framed branch strip,
coverage segments, continuous commit-rail spine), P2 render-existing-data (sha diffstat, tests
count/failed, guard receipts, amendment tag + GATE badge, anchor file:line, shows_git at
Approved/Done, amber rev-behind sync receipt, pushback→red), P3 structural adds (a derived
`DisplayState` driving a matrix-accurate pill incl. the red FAILED row + done·PR#; toolbar caps
status pill + body h1 title; top-of-plan banners; gate-evidence card body; live-column caps header +
pulsing row + curated verbs; comment avatar cards; input-guard evidence panel; dedicated lint card).
All additive in `plan_ui` (+ tiny `plan_core` touches: `LINT_AUTHOR` pub, `latest_amendment_rev`) —
**no upstream files, FORK_DIFF unchanged.** See `docs/milestones/fidelity-pass-2.md` (+ its `-plan`)
for the commit-by-commit record and the recorded deferrals. On branch `plan`. Next is a **PR to
`main`** and/or the **v1 tail** (no MVP milestones remain). Dogfood seed has a
`.plans/seed-state.py <guard|failed|gate|done|executing>` switcher for §13 state walks._

---

## Agent brief (paste or reference)

You're continuing the "Plan" feature in a personal Zed fork (Rust + GPUI). All work is on the
`plan` branch. **Read first, in order:** your project memory (auto-loaded: `plan-fork-project`
+ `zed-build-environment` + **`no-coauthor-trailer`**); then `docs/PRD.md` (Part 0 context,
**Part III working agreement — it governs HOW you work**, Part II build plan + milestones, and —
for whichever v1-tail item you pick — its Part I feature section + Part IV design section (cite the
F-ids)); then the milestone notes `docs/milestones/M0.md … M9c.md`; then `docs/zed-notes.md` (Zed
internals findings, spike results — the 10 study questions cover panels, items, acp events, editor
mini-buffers, git state, settings). Confirm you've read Part III + the relevant milestone notes
before proposing anything.

**Current state: the MVP is COMPLETE (M0–M9, all §13-verified).** There is no next MVP milestone.
The next work is the **v1 tail** (see "Next up") — pick an item, write a plan for sign-off, execute.
Or the branch is ready for a **PR to `main`** when the developer wants one. Never add a
`Co-Authored-By` trailer to commits (see the Working agreement).

### Status
M0–M4, **all of M5 (5a comments · 5b staged revisions · 5c lint)**, **all of M6 (6a launch ·
6b gates/guards · 6c amendments/failure-ladder/control)**, **all of M7 (7a git core +
enforcement · 7b branch strip + commit rail)**, **all of M8 (8a ticket store/coverage/lint ·
8b ticket cards + coverage meter + drift UI)**, and **all of M9 (9a `"plan"` settings key +
consumption · 9b settings page + responsive/cycle panel · 9c `.bak` recovery + §12 drills)** are
**complete and visually verified** — **the MVP feature set is done (M0–M9)**. A broad
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
  coverage meter + drift card**) + status pill + dock panel (**responsive stack + cycle button**);
  **`plan_settings`** (the `"plan"` key: enable/dock/auto_open/default_lens/revisions).
- `plan-agent/` — planning skill (lifecycle + per-task loop + enforcement + **branch/commit git
  protocol**) + hooks + settings.

**Upstream footprint (all tracked in `FORK_DIFF.md`):** M0 registration (root+zed `Cargo.toml`,
`main.rs`, `zed.rs`) + M4 pill line, **plus M9's settings touch** — `settings_content.rs`
(+`PlanSettingsContent`), `assets/settings/default.json` (+defaults), `settings/vscode_import.rs`
(+`plan: None`), `settings_ui/page_data.rs` (+`plan_page`), and the `zed.rs` `plan_enabled(cx)`
signature. M9 was the deliberate, well-trodden non-additive milestone (mirrors the Git Panel
template). M7/M8 added **no** upstream touch (git via already-`pub` `project::git_store`; tickets
are pure `plan_core` + server tools). Watch these files on weekly upstream merges.

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

### Next up — the v1 tail (MVP is done)
No MVP milestones remain. The **v1 tail**, in rough order of likely appetite (PRD Part II §4 +
§14 v1 tier): **peek cards** (F0.5b) → **rehearsal** (F9.4) → **live evidence + acceptance
auto-check** incl. the **diff-view-in-task-card** the developer asked about (F4.6d — `load_commit`
+ `create_editor_diff` are ready) → **PR generation** + live PR chip (F10.4) → **worktree
isolation** (F10.2b) → **attention queue** (F5.7) → **code-anchored feedback** (F9.2) →
**write-back** (F2.4e) → **Plans browser** (F1.6). Then v1.5 distillation, v2+ stacked PRs/blame.

Also carry the **near-term deferrals** each milestone recorded (settings **presets** F12.4;
**lease enforcement** F11.3b; agent-routed **revert** + manual **resync**; **merge-conflict card**
F11.5b; the fidelity backlog).

**Pick an item, write a plan for sign-off (`docs/milestones/…`), execute task-by-task.** Or open a
**PR of `plan` → `main`** when the developer wants to land the MVP.

### M9 deferrals to honor (recorded in M9a/M9b/M9c notes)
- **Settings presets (Careful/Balanced/Fast, F12.4) deferred** — the page ships the 5 keys
  individually settable; preset *cards* need a custom multi-key-write widget (DynamicItem) — a
  polish follow-up. Policy-editor extras (regex tester / provenance / per-agent, F12.1b/F12.3b) are
  v1.
- **Enable is startup-once** — toggling `"plan".enabled` re-registers only on relaunch.
- **`from_settings` unwraps** (git-panel template) — every `PlanSettingsContent` field MUST keep a
  `default.json` default or startup panics; the `settings` crate's default tests guard it.
- **Merge-conflict card (F11.5b) deferred** — `store::load_or_recover` handles the corrupt/
  unparseable case (`.bak` recovery, wired into the single-plan fallback); a rendered conflict card
  with resolve actions is v1.
- **Recovery banner** — `load_or_recover` returns `recovered_from_backup` + `recovered_rev`; a UI
  banner surfacing "recovered from rev N" is a fidelity add (the hook is there).

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

### M6a seams (historical; status noted)
- **Git → RESOLVED in M7:** Launch now creates the branch (guarded), and the branch strip + commit
  rail read live `git_store`. Worktree isolation (F10.2b) is still v1.
- **ExitPlanMode seam (open):** the agent's ACP ExitPlanMode ↔ `plan_launch` reconciliation is
  skill-driven (like the thread-id seam); deeper ACP wiring is later.
- **Lease enforcement (still deferred):** the lease is *set* on launch; the PreToolUse gate only
  checks `status == executing` — leaseholder-only editing + stale-lease reclaim wait on ACP session
  context in the hook (unchanged through M6c/M7).
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
a visual check. Enable with `"plan".enabled: true` in settings **or** `ZED_PLAN=1 cargo run -p zed
-- .` (a throwaway `.plans/led-212.plan.json` test plan is present + a `.plans/policy.json`; a
single-plan fallback shows it. The local test plan has demo `git.pr` + a ticket `drift` set for
visual checks — reset them if you want the non-PR / non-drift states). **Do NOT commit** the
untracked/pre-existing working-tree changes that aren't your work: `.rules`, `.claude/`, `.plans/`
(local dogfood data — untracked, never committed), and `crates/plan_server/LICENSE-GPL`. Stage
explicit paths per commit.

### Decisions to honor
- The MCP server is **Rust** (`rmcp`) reusing `plan_core` — one source of truth, no
  schema/lint duplication.
- The **UI writes `plan.json` directly** via `plan_core`; the **server writes for the agent**.
- **Thread-following keys on the ACP session id** (`active_agent_thread().session_id()`), not
  `agent_ui`'s UUID `ThreadId`. Known seam: the agent must stamp `plan.thread` with the session
  id for true per-thread binding (a single-plan fallback covers it meanwhile).
- All colors via `cx.theme()`. The feature enables on the **`"plan".enabled` setting OR the
  `ZED_PLAN` env var** (M9 added the durable setting; env stays for dev/CI). Registration is
  startup-once — toggling `enabled` takes effect on the next launch.
- **`plan_ui` only reads git** (`project::git_store`); it never mutates git. A UI action that needs
  a git mutation (revert, resync) **routes through the agent**, never UI-shells git nor adds
  upstream git surface.
- **`PlanPanel::activation_priority` is `100`** — must stay unique across all panels (the dock
  panics otherwise on re-home); upstream uses 0–7.

### Deferred backlog (recorded, non-blocking)
Thread-id seam · fidelity pass (motion/pulses, mono fonts, read-only mini-buffers / text-range
selection, free-text comment composer, floating selection toolbar, full §10 pipeline chrome,
`fs().watch()` + activity event-subscription vs polling, rev-behind receipt) · alternatives
(F3.2c) · threads-sidebar plan state (F1.5, needs `agent_ui` changes) · step-level comment
rendering.

---

_When you finish a milestone, update this file's Status + Next-up + Last-updated line so the
next session stays accurate._
