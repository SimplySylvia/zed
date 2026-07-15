# Fidelity pass 1 — visual polish toward the design docs

A cross-cutting `plan_ui` pass (not a numbered milestone) tightening the built surfaces
(M3 tab · M4 pill/panel · M5 review) against `plan-ui-compliance.md` + `plan-ui-design-spec.md`.
Scope agreed with the developer: **Tier 1 (mono metadata, comment fidelity, pill polish) +
animations (pulse)**. Strictly aligning code to the documented design — no design invention.

**Status:** COMPLETE. `plan_ui` tests + clippy + full `zed` build green; **§13 visual
verification under One Dark confirmed by the developer (2026-07-15)** — mono metadata, colored
comment state chips + mono anchor labels, the tinted pill, and the pulse (tab/panel dots +
gate-only pill, reduced-motion honored) all verified against a rich seed plan. Deferred items
(below) remain.

## What changed

- **Mono metadata (G2/G7)** — a shared `mono_chip` helper (bordered pill with a `buffer_font`
  label) + `Label::buffer_font(cx).size(XSmall)` for plain metadata. Applied to: toolbar `rev`,
  panel `done/total` + sync receipt, task-card SHA chip, staged-hunk target + `rev N` note +
  `from cN` provenance, and lint rule-id chips. Metadata is now mono/placeholder per the contract
  instead of the UI font.
- **Comment fidelity (§9)** — the thread state now renders as a **colored chip**
  (`comment_state_color`: open/sent/reopened → modified·amber, addressed → info, resolved →
  created·green), plus a **mono anchor label `c1 ↪ t2.s1`** on user/agent comments (lint comments
  show their rule-id chip instead).
- **Pill polish (§1)** — the status pill is now a **rounded (`rounded_full`) pill tinted by the
  state family** (border + 10%-opacity fill), not a bare label.
- **Pulse animation (G4)** — shared `pulse` + `status_dot` helpers (GPUI
  `pulsating_between(0.4, 0.95)`, 2s repeat, honoring reduced-motion). Status **dots** pulse for
  drafting/executing/gate (design-spec §2/§9); the **pill** pulses only for gate/guard-hold
  (contract §1). Helpers live in `plan_ui.rs` (`pub(crate)`), reused by tab/panel/pill.

## Mapping decisions
- Metadata color: `Placeholder` (timestamps/rev/targets) or the semantic role for chips
  (SHA→created, provenance→info), all mono via `buffer_font`.
- `comment_state_color`: `addressed` maps to `Info` (agent acted, awaiting the user) — the
  contract names open/pushback/resolved; addressed sits between open and resolved.
- Dot vs pill pulse rules differ intentionally (dots: drafting/executing/gate per §2; pill:
  gate/guard-hold per §1).

## §13 protocol (developer, under One Dark)
1. **Metadata** reads mono/muted (rev, SHAs, sync receipt, hunk targets, `from cN`, rule ids,
   `c1 ↪ block`) — not the UI font. (G2/G7)
2. **Pulse**: a drafting/executing/gate plan's tab + panel dot pulses; the pill pulses at a gate.
   Enable reduced-motion → pulsing stops. (G4)
3. **Comment rows**: state chip is color-correct (green resolved / amber open); anchor label is
   mono. **Pill** is a rounded tinted chip.
4. Regression (contract §13.4): re-check §0 globals + the surfaces consuming the shared chips
   (task card, staged-rev card, lint rows, panel).

## Deferred (remaining fidelity, recorded)
- **Panel (§10, Tier 2 #5)** — pipeline row tints + right-side mono status note; live-column
  teal mono **verb column**.
- **Dedicated §6 Lint card** (auto-fixed dimming, inline fix buttons) — findings still render as
  comment rows.
- **Mini-buffers** for staged-rev/preview diffs; Spec-lens finding rendering; peek cards.
- Pill/toolbar fragment text aligned to the full §9 matrix (live counts).

## Fork discipline
Additive only, all in `plan_ui`. No upstream files touched — `FORK_DIFF.md` unchanged.
