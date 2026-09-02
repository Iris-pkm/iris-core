//! iris-core — the correctness-critical engine for Iris.
//!
//! This crate provides:
//! - **Parser** — lossless frontmatter parsing (ADR-019)
//! - **Types** — the typed node model (SCHEMA_SPEC.md)
//! - **Vault engine** — create/open/read/write vaults (to come)
//! - **SQLite cache** — derived, rebuildable index (to come)
//! - **Git integration** — vault-as-repository (to come)

pub mod cache;
pub mod engine;
pub mod error;
pub mod git;
pub mod integrity;
pub mod parser;
pub mod search;
pub mod types;
pub mod vault;
pub mod views;
