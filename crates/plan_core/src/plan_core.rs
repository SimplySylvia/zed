//! Core, GPUI-free logic for the Plan feature: schema, atomic IO, file watch,
//! lint, and comment anchoring. Nothing in this crate may depend on GPUI.
//!
//! M1 implements the schema (`schema`) and atomic persistence (`store`). The
//! lint engine, comment anchoring, and pending-revision apply/reject arrive in
//! later milestones. See docs/PRD.md Appendix A and docs/milestones/.

pub mod anchor;
pub mod comments;
pub mod rev;
pub mod schema;
pub mod store;

pub use schema::*;

/// On-disk `plan.json` layout version. Bumped whenever the schema changes; it
/// drives forward migration in [`schema::migrate`]. See docs/PRD.md Appendix A.
pub const SCHEMA_VERSION: u32 = 1;
