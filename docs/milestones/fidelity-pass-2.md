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

## P2 — render data we already carry — PENDING
sha diffstat + accent color · tests count/failed variant + evidence chip · guard receipts (timestamp
+ what-ran) · amendment tag + GATE badge · anchor `file:line` · `shows_git` window (strip + rail at
Approved/Done) · sync-receipt amber "rev behind" variant · pushback→red state chip · drift
attribution + Apply-rev.

## P3 — rendering-only structural adds — PENDING
`DisplayState` derivation → toolbar status pill · top-of-plan banner strip · gate-evidence card body
· live-column caps header + pulsing live row + verb vocabulary · comment avatar cards · input-guard
panel body · lint card.

## Fork discipline
Additive only, all in `plan_ui`, except Task 0's design-doc edits. No upstream code touched —
`FORK_DIFF.md` unchanged.
