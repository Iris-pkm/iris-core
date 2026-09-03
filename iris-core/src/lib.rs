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
pub mod export;
pub mod ffi;
pub mod git;
pub mod import;
pub mod integrity;
pub mod parser;
pub mod search;
pub mod types;
pub mod vault;
pub mod views;

// UniFFI scaffolding — exposes a minimal slice of the API to native shells
// (ADR-031). This is a spike to prove the toolchain works end-to-end
// (including the community-maintained C# binding generator, a flagged risk
// in ADR-031), not the final FFI surface — that grows alongside Phase 1 UI work.
uniffi::setup_scaffolding!();
