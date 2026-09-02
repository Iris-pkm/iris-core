//! iris-core — the correctness-critical engine for Iris.
//!
//! This crate provides:
//! - **Parser** — lossless frontmatter parsing (ADR-019)
//! - **Types** — the typed node model (SCHEMA_SPEC.md)
//! - **Vault engine** — create/open/read/write vaults (to come)
//! - **SQLite cache** — derived, rebuildable index (to come)
//! - **Git integration** — vault-as-repository (to come)

pub mod cache;
pub mod error;
pub mod parser;
pub mod types;
pub mod vault;
