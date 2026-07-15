# M5a · Review loop — comments round-trip — milestone note

**Status:** code complete; `plan_ui` + full `zed` build green. **§13 visual verification under
One Dark pending developer confirmation.**

**Feature IDs:** F3.1 (anchors), F3.2a/b/d (comment/suggest/flag), F3.3 (severity), F3.4
(batch → revision), F6.2 (review tools). Alternatives (F3.2c) + Approve gating (F3.6) → 5b.

## What works (built)

- **Fuzzy re-anchoring** (`plan_core::anchor`, the §7 spike) — `reanchor` classifies an
  anchor as Anchored / Outdated / Moved / Detached across revisions; **5 property tests**.
- **Comment mutations** (`plan_core::comments`, shared by UI + server) — add_comment,
  add_suggestion, reply, set_comment_state, mark_sent_batch, apply_suggestion (applies the
  replacement to the anchored block); each rev-bumps + stamps history. **5 tests**.
- **Review MCP tools** (`plan_server`) — `plan_list_comments` (with re-anchor status),
  `plan_reply_comment`, `plan_mark_addressed`, `plan_apply_suggestion`. **4 tests**.
- **Review UI** (Plan tab) — each task card has a ⚑ flag affordance that writes a blocker
  comment (anchored to the task) to `plan.json`; anchored comments render under their block
  (severity glyph + text + state chip + **outdated** badge); header shows **Send for
  revision · n** (mark_sent_batch). The UI **writes `plan.json` directly** via `plan_core`
  (PRD §6) and reloads via the poll.

## The round-trip
Flag a task in the tab → it persists to `plan.json` → the agent sees it via
`plan_list_comments` → replies / marks addressed / applies a suggestion → the tab re-renders
the thread + state. Anchors survive revisions (outdated badge when the quoted text moved).

## §13 record (deviations — flagged, for a fidelity pass)
- **Block-level anchoring** for 5a (click a task → flag). **Text-range selection** (needs
  read-only mini-buffers) + a **free-text composer** + the **floating selection toolbar**
  (Comment/Suggest ⌘⇧M/⌘⇧E) are deferred; the ⚑ flag uses canned text for now.
- **Alternatives** (F3.2c) and **Approve gating** (F3.6, blocker-count → Approve disabled)
  deferred to **5b**.
- Comment threads render a compact single-line summary (severity + first message + state);
  full threaded message UI + pushback buttons come with the richer review surface.

## Fork discipline
Additive only. `plan_core` gained `anchor`/`comments` modules; `plan_server` gained review
tools; `plan_ui` writes `plan.json` via `plan_core`. **No upstream files touched** —
`FORK_DIFF.md` unchanged.

## Definition of done
- [x] `plan_core::anchor` re-anchors correctly (property tests); §7 quality acceptable.
- [x] Comment mutations test-first in `plan_core` (5).
- [x] Review MCP tools present + correct (4).
- [x] Tab: flag a task → persists → thread renders → `list_comments` sees it; batch-send works.
- [x] Builds green; pure logic test-first.
- [ ] **§13 visual verification under One Dark** — pending developer confirmation.
- [x] `docs/milestones/M5a.md` written; FORK_DIFF unchanged.

## Next: M5b — staged revisions
`plan_propose_revision(hunks)` + per-hunk Apply/Reject reusing the editor diff machinery
(git_ui hunk patterns from the M0 study), change cards with old→new + provenance, Approve
gating (enabled at zero open blockers). Then M5c — the policy.json lint engine.
