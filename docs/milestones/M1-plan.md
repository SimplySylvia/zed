# M1 · plan_core — implementation plan

> **For Claude:** REQUIRED SUB-SKILL when executing — use superpowers:executing-plans
> (task-by-task, checkpoints) and superpowers:test-driven-development for every logic task.

**Status:** awaiting sign-off (not started).
**Feature IDs:** F11.5 (integrity: validation, atomic temp+rename, `.bak`), §11 persistence,
Appendix A (schema).
**Goal:** Turn `plan.json` (PRD Appendix A) into typed, validated Rust with atomic,
crash-safe persistence — the foundation every later milestone reads/writes.

**Architecture:** Pure-Rust `plan_core` (no GPUI). `schema.rs` = serde structs mirroring
Appendix A + `schema_version` + `migrate()`, preserving unknown fields for forward-compat.
`store.rs` = atomic save (temp + fsync + rename), `.bak` rotation, load-validate. Tested
against the LED-212 demo content as a fixture. Nothing else depends on it yet.

**Tech stack:** Rust, `serde`/`serde_json`, `anyhow`; `std::fs` only (no temp-file crate).

Commit convention: one commit per task, imperative, prefixed `[PLAN-M1]`. Never commit a
red `cargo test -p plan_core`.

Build/test env (from M0): `export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"`.
`plan_core` has no GPUI/Metal in its path, so `DEVELOPER_DIR`/Xcode are **not** needed for
`cargo test -p plan_core` — this milestone is environment-light.

---

## Design decisions (baked in; see Open Questions for the ones needing your call)

- **Unknown fields are preserved**, not dropped (§11). Every major object carries
  `#[serde(flatten)] pub extra: BTreeMap<String, serde_json::Value>`. A newer writer's
  fields survive an older reader's load→save round-trip. Round-trip tests enforce this.
- **Atomic save**: write `<id>.plan.json.tmp.<pid>` in the *same directory*, `flush` +
  `sync_all`, then `fs::rename` over the target (atomic within a filesystem). Never write
  the target in place.
- **`.bak` rotation**: before overwriting, copy the current file to
  `.plans/.bak/<id>.rev<N>.plan.json`; keep the newest 10, prune older. (One of the three
  undo layers in F11.5; git is another, the atomic temp the third.)
- **Validation** (M1 scope): structural (serde parse) + light referential integrity
  (`task.depends_on`, `acceptance[].tasks`, `task.acceptance` reference existing ids).
  Full policy lint (`.plans/policy.json` rules) is deferred to M5 `lint.rs`.
- **Module layout** (per PRD §2 + CLAUDE.md — crate root is `plan_core.rs`, not `lib.rs`,
  no `mod.rs`): `src/plan_core.rs` declares `mod schema; mod store; mod error;` and
  re-exports the public API. `anchor.rs`/`rev.rs`/`lint.rs` are **not** in M1.

---

## Tasks

### Task 1 — Crate deps + module skeleton + error type
`[PLAN-M1]: add plan_core deps and module skeleton`

**Files:**
- Modify: `crates/plan_core/Cargo.toml` (add `serde`, `serde_json`, `anyhow`; dev-dep `pretty_assertions` if useful)
- Modify: `crates/plan_core/src/plan_core.rs` (declare modules + re-exports; keep `SCHEMA_VERSION`)
- Create: `crates/plan_core/src/error.rs` (a `PlanError` enum via `anyhow`/`thiserror`-free `anyhow::Error` wrappers, or a thin typed error — recommend `anyhow` + context strings for M1)
- Create empty: `crates/plan_core/src/schema.rs`, `crates/plan_core/src/store.rs`

**Steps:** add deps → declare `mod schema; mod store; mod error;` and
`pub use schema::Plan; pub use store::{load, save};` (stubs compile) → `cargo build -p plan_core`.
**Acceptance:** `cargo build -p plan_core` green; still no `gpui` in `cargo tree -p plan_core`.

### Task 2 — Author the LED-212 fixture
`[PLAN-M1]: add LED-212 plan.json fixture`

**Files:**
- Create: `crates/plan_core/tests/fixtures/led-212.plan.json`

Build a *complete, valid* `plan.json` from PRD Appendix A for the LED-212 pagination plan:
`status: "executing"`, `rev: 5`, one Jira ticket with the 4 AC, spec goal/scope/acceptance
(a1…), design contracts/decisions/risks, tasks t1/t2 with steps/files/guards/commit
messages, at least one comment with an anchor, a `pending_revision`, `git` block with a
commit, and `history`. Fill Appendix A's `"…"` placeholders with plausible LED-212 content.
**Acceptance:** the file is valid JSON (`python3 -m json.tool` or `jq . <file>` succeeds)
and includes every top-level key in Appendix A. This becomes the golden fixture for Tasks 3–6.

### Task 3 — Schema structs + round-trip (TDD)
`[PLAN-M1]: add plan.json schema structs with round-trip`

**Files:** `crates/plan_core/src/schema.rs`; `crates/plan_core/tests/schema_roundtrip.rs`

**Step 1 — failing test:** load the fixture, `serde_json::from_str::<Plan>`, re-serialize,
and assert the re-serialized value **equals the original parsed `serde_json::Value`**
(compare as `Value`, not string, to ignore key ordering/whitespace).
```rust
#[test]
fn led_212_round_trips_without_loss() {
    let raw = include_str!("fixtures/led-212.plan.json");
    let original: serde_json::Value = serde_json::from_str(raw).unwrap();
    let plan: plan_core::Plan = serde_json::from_str(raw).unwrap();
    let reserialized = serde_json::to_value(&plan).unwrap();
    assert_eq!(reserialized, original); // no field dropped, no value changed
}
```
**Step 2:** run — expect FAIL (`Plan` undefined / fields missing).
**Step 3:** implement the full struct set mirroring Appendix A: `Plan`, `Ticket`, `Spec`,
`Acceptance`, `Evidence`, `OpenQuestion`, `Design`, `Contract`, `Preview`, `Decision`,
`Task`, `Step`, `Guard`, `Artifacts`, `Comment`, `Anchor`, `CodeRef`, `Suggestion`,
`Alternatives`, `ThreadEntry`, `Pushback`, `PendingRevision`, `Hunk`, `Git`, `Commit`,
`Pr`, `HistoryEntry`. Enums with `#[serde(rename_all = "lowercase")]` (or explicit renames)
for `status`, task `status`, comment `kind`/`author`/`state`/`severity`, guard
`type`/`state`, op `C|M`, etc. Every object struct gets `#[serde(flatten)] pub extra:
BTreeMap<String, serde_json::Value>` and `#[serde(default, skip_serializing_if =
"Option::is_none")]` on nullable fields so `null`/absent round-trips faithfully.
**Step 4:** run — expect PASS. **Step 5:** add a second test asserting **unknown-field
preservation** (inject `"future_field": 42` into a copy, load→save, assert it survives).
**Step 6:** commit.
**Acceptance:** both tests green; the fixture round-trips with zero loss.

### Task 4 — schema_version + migrate() (TDD)
`[PLAN-M1]: add schema_version handling and migrate scaffold`

**Files:** `crates/plan_core/src/schema.rs`; `crates/plan_core/tests/migrate.rs`

**Step 1 — failing tests:** (a) a v1 plan passes through `migrate` unchanged; (b) a plan
with `schema_version` **newer** than `SCHEMA_VERSION` returns a clear error (don't silently
mangle); (c) a plan **missing** `schema_version` is treated as v1 (or errors — decide per
Open Q). **Step 2:** run — FAIL. **Step 3:** implement `pub fn migrate(mut value:
serde_json::Value) -> anyhow::Result<serde_json::Value>` that inspects `schema_version`,
applies ordered migrations (none yet → identity for v1), and errors on unknown-future.
`load` calls `migrate` on the raw `Value` *before* typed deserialization. **Step 4:** PASS.
**Step 5:** commit.
**Acceptance:** version handling covered; a future-version fixture errors gracefully.

### Task 5 — store: atomic save + `.bak` + load (TDD)
`[PLAN-M1]: add atomic save, bak rotation, and load`

**Files:** `crates/plan_core/src/store.rs`; `crates/plan_core/tests/store.rs`

**Step 1 — failing tests** (use `tempfile`-free `std::env::temp_dir()` + a unique subdir,
cleaned up):
- `save` then `load` returns an equal `Plan`.
- `save` writes atomically: no `*.tmp.*` left behind; target exists and parses.
- Overwriting an existing plan copies the prior file to `.plans/.bak/<id>.rev<N>.plan.json`.
- `.bak` keeps at most 10; the 11th save prunes the oldest.
- `load` of malformed JSON returns an `Err` with useful context, not a panic.
**Step 2:** run — FAIL. **Step 3:** implement:
```rust
pub fn save(plans_dir: &Path, plan: &Plan) -> anyhow::Result<()> {
    let target = plans_dir.join(format!("{}.plan.json", plan.id));
    if target.exists() { backup(plans_dir, plan)?; }          // .plans/.bak rotation
    let tmp = plans_dir.join(format!("{}.plan.json.tmp.{}", plan.id, std::process::id()));
    let bytes = serde_json::to_vec_pretty(plan)?;
    { let mut f = fs::File::create(&tmp)?; f.write_all(&bytes)?; f.flush()?; f.sync_all()?; }
    fs::rename(&tmp, &target)?;                               // atomic within the fs
    Ok(())
}
pub fn load(plans_dir: &Path, id: &str) -> anyhow::Result<Plan> {
    let raw = fs::read_to_string(plans_dir.join(format!("{id}.plan.json")))?;
    let value = schema::migrate(serde_json::from_str(&raw)?)?;
    Ok(serde_json::from_value(value)?)
}
```
(+ `backup` helper: ensure `.plans/.bak/` exists, copy, prune to newest 10.)
**Step 4:** PASS. **Step 5:** commit.
**Acceptance:** all store tests green; verify (in a test) no temp file remains after `save`.

### Task 6 — validate: referential integrity (TDD)
`[PLAN-M1]: add referential-integrity validation`

**Files:** `crates/plan_core/src/schema.rs` (a `Plan::validate(&self) -> Result<(), Vec<String>>`);
`crates/plan_core/tests/validate.rs`

**Step 1 — failing tests:** the LED-212 fixture validates clean; a mutated copy with
`acceptance[0].tasks = ["t99"]` (nonexistent) fails; `task.depends_on` / `task.acceptance`
pointing at missing ids fail; each error names the offending id. **Step 2:** FAIL.
**Step 3:** implement `validate` checking: every `acceptance[].tasks` id ∈ task ids; every
`task.acceptance` id ∈ acceptance ids; every `task.depends_on` id ∈ task ids; `executor.thread
== thread` when present. Wire an optional `load`-time validate (behind a `load_validated`
that returns both the plan and any warnings — keep hard structural errors fatal, referential
issues as collected warnings for now). **Step 4:** PASS. **Step 5:** commit.
**Acceptance:** validation catches dangling references with clear messages; fixture is clean.

### Task 7 — Milestone note
`[PLAN-M1]: add M1 milestone note`

**Files:** `docs/milestones/M1.md` — what works, deferred (`anchor`/`rev`/`lint` → M5),
surprises, and the DoD checklist.

---

## Definition of done (Part III §4)
- [ ] `cargo test -p plan_core` green; test-first followed for Tasks 3–6.
- [ ] LED-212 fixture round-trips with zero field loss; unknown fields preserved.
- [ ] Atomic save (temp+rename) + `.bak` rotation proven by tests; no temp residue.
- [ ] Referential-integrity validation catches dangling ids.
- [ ] `schema_version`/`migrate` scaffold in place; future version errors gracefully.
- [ ] `docs/milestones/M1.md` written; no upstream files touched (FORK_DIFF unchanged).

## Open questions (need a decision)
1. **Missing `schema_version` on load** — treat as v1 (lenient, good for hand-written
   fixtures) or hard-error? *Recommend lenient→v1 for M1.*
2. **`.bak` retention** — keep newest **10** by rev, prune older? Or time-based? *Recommend
   keep-10-by-rev (simple, bounded).*
3. **Validation severity** — should referential-integrity failures be **fatal on load** or
   **collected warnings** (load still succeeds)? *Recommend warnings in M1 (the UI/agent
   decide); hard structural/serde errors stay fatal.* Fatal-on-load risks bricking a plan
   the user could otherwise repair.
4. **Fixture prose source** — I'll author `led-212.plan.json` from Appendix A with plausible
   content. If the HTML demo (`planning-mode-demo-e2e-pagination.html`) has canonical prose
   you want mirrored exactly, share it and I'll match it; otherwise Appendix A is the source
   of truth. *Recommend: proceed from Appendix A.*
5. **Error type** — `anyhow` throughout (fast, M1-appropriate) vs a typed `PlanError` enum
   (better for the UI layer later). *Recommend `anyhow` now; introduce a typed error when
   `plan_ui`/`plan-server` need to branch on error kind (M3+).*
