# Fidelity pass 2 — visual polish toward the mockup (in progress)

A cross-cutting `plan_ui` pass closing the finish gap between the built surface and the approved
mockup (`docs/design/planning-mode-demo.html`). Plan: `docs/milestones/fidelity-pass-2-plan.md`.
Three phases — **P1** chassis/tokens · **P2** render data we already carry · **P3** rendering-only
structural adds. Additive, `plan_ui`-only (no upstream files, `FORK_DIFF.md` unchanged). Commits
prefixed `[PLAN-FP2]`.

## Task 0 — design-doc assertions (signed off)
`9b556fd3e2` — formalized the top-of-plan banner strip (contract §14 + design-spec §3.9, which
lived only in the mockup HTML), added global **G9** (the shared chip chassis is a filled,
`rounded_full` tinted pill — 10%-opacity role fill + role border), and tightened §10 with the
live-column caps header clause.

## P1 — chassis & tokens — COMPLETE, §13-verified

**Status:** all six tasks done; `plan_ui` build + 10/10 tests + clippy green; full `zed` build green;
**§13 visual verification under One Dark confirmed by the developer (2026-07-16)** against the
enriched LED-212 seed (6 tasks spanning the full rail-node vocabulary).

- `c2d81ab55f` **T1 chip chassis** — `chip`/`mono_chip` are now filled full-radius tinted pills
  (G9). Added `mono_chip_ticket` (4px, no fill) for the G9 ticket-`⛓ KEY` exception; the 4 ticket-key
  sites (header, task card, ticket card, acceptance ref) use it.
- `dd906cda54` **T2 primary action** — a plan_ui-local `PrimaryButton` (`RenderOnce`) rendering a
  **solid accent fill** (`text_accent` bg/border, `background` ink), disabled dim keeps the G5
  tooltip, G8 hover. Replaces the translucent `Tinted(Accent)`.
- `c378c04d33` **T3 lens switcher chassis** + `2942486b0b` **active-lens highlight** — the three
  lenses sit in one segmented container (panel bg + border-variant + rounded); the active lens
  fills with `element_selected` (design-spec §1 segmented-on role, lighter than the container) so
  it clearly stands out; inactive lenses muted with a hover fill.
- `6bdeda194d` **T4 branch strip** — outer strip is a framed pill (panel bg + border-variant +
  rounded); the `← base` segment is placeholder-dimmed vs the branch name; the rail-foot
  `↑n ahead` is `created` (green).
- `621281a0dd` **T5 ticket radius + coverage** — ticket card `rounded_md`→`rounded_lg` (8px);
  coverage segments now all carry a matching border (covered=created, needs-update=modified,
  empty=editor-bg + border-variant) at 3px radius.
- `3494426fe7` **T6 rail spine** — one continuous 2px spine behind the node gutters, inset from
  the top by the first node's center (13.5px) so it begins at the first node.

### Mapping decisions (P1)
- **G9 chip fill:** `bg = role.opacity(0.1)`, `rounded_full`; ticket-key chip is the sole 4px
  exception (`mono_chip_ticket`).
- **Primary button — "Option A" (developer-chosen):** Zed has no solid-accent-fill button style and
  no `text_on_accent` theme role, so `PrimaryButton` is a plan_ui-local element using `text_accent`
  fill + `background` ink. **Caveat (documented):** dark-ink-on-accent is tuned for dark themes;
  light-theme contrast is a known limitation (§13 is One Dark). The theme-robust alternative (add a
  `text_on_accent` role) was declined to preserve fork discipline.
- **Lens active state:** `element_selected` (not `element_active`) for the segmented-on fill.

### Deviations / deferrals (P1)
- **Rail-spine bottom terminus (§8):** the spine begins at the first node but runs the list's full
  height *toward the foot* rather than terminating exactly at the last node's center. The per-row
  segment approach that would end precisely at the last node collapsed (GPUI doesn't stretch the
  fixed-width gutter to card height), so the proven single-line primitive is used instead. Exact
  last-node termination needs per-row measurement and stays a deferral — same "full-height" deferral
  that predates this pass. Developer accepted the continuous-line-toward-foot result in §13.

## P2 — render data we already carry — COMPLETE (developer directed to proceed)

**Status:** implemented + two-stage reviewed (spec + code-quality); `plan_ui` build + 13/13 tests +
`plan_core` exec suite + clippy green. The seed was enriched to exercise every P2 state (tests
counts, pushback, agent-behind); developer directed to move on to P3 (P2 §13 spot-check at their
discretion).

- `facd33d407` **T7/T8/T10** — sha chip recolored `text_accent` + segmented `+`/`−` diffstat;
  `tests_chip_label` (documented Value convention + unit test) rendering `✓ n tests` / `✕ n tests`;
  purple `◆ amendment · rev n` tag (via new `exec::latest_amendment_rev`, one history scan);
  amber `GATE` badge when `task.gate`. Chip order per §7.
- `7f8222dde8` **T11 + T14** — comment anchor appends `+ file:line` from the first code ref (§9);
  `comment_state_color` gains a `pushback → Error(red)` arm; toolbar blocker chip → singular
  `⚑ n blocker` token (§3).
- `f3c1773e06` **T12** — `shows_git` widened to `Approved | Executing | Paused | Gate | Amending |
  Done` so the branch strip + rail render from launch through completion (§5/§8); unit-tested.
- `00e4ac1593` **T13** — `sync_receipt` returns `(text, behind)`; amber "agent synced rev N · <at>
  — rev M syncs before next task" when the latest agent revision trails the plan rev (§10);
  behind-state test added.

### Mapping decisions (P2)
- **`tests` Value convention** (schema leaves `artifacts.tests` freeform): number→`✓ N tests`; bool
  true/false→`✓ tests`/`✕ tests`; object `{failed,passed}`→failed>0 red `✕ {failed}` else green
  `✓ {passed}`; string→`✓ {s}`; else→`✓ tests`.
- **Agent-behind signal:** the latest history entry with `by=="agent"` and `kind=="revision"`; its
  `rev` < `plan.rev` ⇒ behind (no dedicated schema field needed).
- **`⚑ n blocker`** follows the contract's compact singular token even for n>1 (not English plural).

### Deferrals (P2)
- **T15 drift attribution + Apply-rev — DEFERRED (recorded):** `TicketDrift` carries no author/when,
  so attribution needs a schema field + agent support (not "render existing data"); and Apply-rev
  conflicts with the M8b decision that the drift card is informational and applying/resync is
  agent-routed (`render_drift_card` is a free fn with no action plumbing). Pursuing it is
  schema-change + agent-flow work, not UI polish. The drift card stays informational.
- **Evidence chip (teal `⛨ guards n/n` / `n evidence items`) — DEFERRED:** the "n evidence items"
  source overlaps the existing guard-summary chip and isn't cleanly specified; flagged out of the
  T8 batch.

## P3 — rendering-only structural adds — COMPLETE (code); §13 pending

**Status:** all tasks implemented + reviewed (two-stage on the logic-heavy ones, diff-verified on the
pure-render ones); `plan_ui` build + 22 tests + `plan_core` suites + clippy green. §13 visual check
outstanding (batch).

- `4ec4bb465d` **T16 + T17a** — `pub(crate) DisplayState` enum + `display_state(plan)` (derives the
  §9 matrix row from plan contents: hold-gate → hold-nongate → status, with InReview/Lint/RevStaged
  sub-branching). `pill_fragment` rewired onto it — incl. the previously-missing **red TaskFailed**
  row and **done · PR #n**. Helpers `role()`/`caps_label()`/`pulses()` single-source the color.
- `79c982f62e` **T17b + T18** — toolbar leads with a tinted **caps status pill** (role bg/border,
  pulses at gate/guard-hold); the redundant toolbar title moved to a body **h1** (`plan.title`);
  **top-of-plan banner strip** (`render_banners`) for GuardHold/TaskFailed/Gate/Done per §14.
- `42ae330522` **T19** — **gate-evidence card body**: one row per acceptance criterion with evidence
  (✓/claim + mono evidence chips, green/amber by done+stale) + actions `✓ Approve & finish` /
  `Re-verify all` / `Request changes`.
- `8f1fc52f77` **T20** — live column: **caps section header** (from `display_state`), **pulsing
  accent dot** on the running row, and a curated **verb mapping** (tool-kind → read/edit/run/plan/
  fetch via serde canonical names, no new dep).
- `c0c06f961d` **T21** — comments render as **editor-bg cards** with the full multi-message thread,
  14px author **avatars** (you-amber / agent-purple / lint), and `rev N` provenance chips.
- `4b6e730260` **T22** — **input-guard evidence panel**: caps evidence-target label (from
  `evidence_for`), input-field chrome, `Record & continue ⏎` + `Pause — I'll verify later` (wired to
  `pause_plan`), and the hook hint.
- `965b504a68` **T23** — lint findings render in a dedicated **§6 lint card** (warn band, severity
  glyph + finding + mono rule id, resolved → dimmed + `✓ auto-fixed` pill); lint comments are
  filtered out of the inline per-task rows (no double render).

### Mapping decisions (P3)
- **`DisplayState` is the single matrix source** for pill fragment/color, toolbar pill, and banners;
  `role()` centralizes the color so the 4 consumers don't drift. Intake-answered (`questions==0`) →
  Muted; asking → Warning.
- **Curated verbs** come from the ACP tool-call feed via serde canonical names (edit/run/read/fetch/
  plan). Fork-clean (no `agent_client_protocol` dep added).
- **Lint = a card, not inline comments** (§6); the card is the single place lint findings appear.

### Deferrals (P3, recorded)
- **Rehearsed matrix row** — no `plan_core` rehearsal/mismatch data; `DisplayState` has no Rehearsed
  variant (would need schema + agent support).
- **Live plan-semantic verbs** `guard/hook/ev/drift/plan` — not derivable from the ACP tool-call feed
  (would need plan-lifecycle event plumbing); only tool-derived verbs render.
- **Gate secondary actions** `Re-verify all` / `Request changes` — rendered but agent-routed, handlers
  deferred (like revert/resync).
- **Lint "passed n/n" pass card** — no reliable "lint ran, 0 findings" signal; the card renders only
  when findings exist.
- **Input-field live text capture** — chrome only; live capture needs an editor entity (F4.5b).
- **Banner/pill copy nuances** — gate banner's task-number + "staging evidence attached" phrasing, the
  §9 pill counts ("2 need you"), and the rev-staged `· pushback` suffix are §14/§9 copy gaps.

## Fork discipline
Additive only, all in `plan_ui`, except Task 0's design-doc edits. No upstream code touched —
`FORK_DIFF.md` unchanged.
