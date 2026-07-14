//! Core, GPUI-free logic for the Plan feature: schema, atomic IO, file watch,
//! lint, and comment anchoring. Nothing in this crate may depend on GPUI.
//!
//! M0 only proves the crate builds and stays GPUI-free. The schema structs,
//! atomic store, and lint engine land in M1+ (see docs/PRD.md Appendix A and
//! docs/milestones/).

/// On-disk `plan.json` layout version. Bumped whenever the schema changes; it
/// drives forward migration in M1. See docs/PRD.md Appendix A.
pub const SCHEMA_VERSION: u32 = 1;
