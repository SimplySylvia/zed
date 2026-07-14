//! Atomic, crash-safe persistence for `plan.json`: temp-write + fsync + rename,
//! `.bak` rotation, and load-with-migration. See PRD §11 and F11.5.
