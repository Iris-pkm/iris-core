//! The FFI-facing `Node` DTO (ADR-031).
//!
//! `types::Node` can't cross the UniFFI boundary as-is: `chrono::DateTime<Utc>`/
//! `NaiveDate` and `serde_yaml::Value` (inside `AnnotationAnchor`) have no
//! built-in UniFFI representation. `FfiNode` mirrors `Node` field-for-field
//! with those replaced by boundary-safe types — timestamps and dates as
//! RFC3339 / ISO-8601 strings (readable and parseable natively on every
//! target: Swift, C#, Kotlin, GTK), and `crdt_position` as its YAML text.
//!
//! `Node` stays the type all internal logic (engine, cache, search, …) uses;
//! `FfiNode` exists only at the boundary, built with `From<&Node>` and
//! converted back with `TryFrom<FfiNode>` (fallible: a native caller could
//! hand back a malformed date/timestamp string).

use crate::types::{AnnotationAnchor, Node, Recurrence};
use chrono::{DateTime, NaiveDate, Utc};

/// Errors converting an `FfiNode` (native-side data) back into a `Node`.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiConversionError {
    #[error("invalid RFC3339 timestamp in field `{field}`: {value}")]
    InvalidTimestamp { field: String, value: String },
    #[error("invalid ISO-8601 date in field `{field}`: {value}")]
    InvalidDate { field: String, value: String },
    #[error("invalid YAML in `crdt_position`: {0}")]
    InvalidYaml(String),
}

fn to_rfc3339(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn from_rfc3339(field: &str, value: &str) -> Result<DateTime<Utc>, FfiConversionError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| FfiConversionError::InvalidTimestamp {
            field: field.to_string(),
            value: value.to_string(),
        })
}

fn to_iso_date(d: &NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn from_iso_date(field: &str, value: &str) -> Result<NaiveDate, FfiConversionError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| FfiConversionError::InvalidDate {
        field: field.to_string(),
        value: value.to_string(),
    })
}

/// `AnnotationAnchor` with `crdt_position` as YAML text instead of
/// `serde_yaml::Value` (which has no UniFFI representation).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiAnnotationAnchor {
    pub text_fragment: Option<String>,
    pub crdt_position_yaml: Option<String>,
}

impl From<&AnnotationAnchor> for FfiAnnotationAnchor {
    fn from(a: &AnnotationAnchor) -> Self {
        FfiAnnotationAnchor {
            text_fragment: a.text_fragment.clone(),
            // A `serde_yaml::Value` always re-serializes; this can't fail.
            crdt_position_yaml: a
                .crdt_position
                .as_ref()
                .map(|v| serde_yaml::to_string(v).expect("Value always serializes")),
        }
    }
}

impl TryFrom<FfiAnnotationAnchor> for AnnotationAnchor {
    type Error = FfiConversionError;

    fn try_from(a: FfiAnnotationAnchor) -> Result<Self, Self::Error> {
        let crdt_position = a
            .crdt_position_yaml
            .map(|s| {
                serde_yaml::from_str(&s).map_err(|e| FfiConversionError::InvalidYaml(e.to_string()))
            })
            .transpose()?;
        Ok(AnnotationAnchor {
            text_fragment: a.text_fragment,
            crdt_position,
        })
    }
}

/// `Recurrence` with `until`'s `NaiveDate` as an ISO-8601 string, same reason
/// as everywhere else in this module: `uniffi` has no `NaiveDate` support.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRecurrence {
    Fixed {
        interval: String,
        until: Option<String>,
        count: Option<u32>,
    },
    Flexible {
        interval: String,
        until: Option<String>,
        count: Option<u32>,
    },
    Rrule {
        rrule: String,
        dtstart: String,
    },
}

impl From<&Recurrence> for FfiRecurrence {
    fn from(r: &Recurrence) -> Self {
        match r {
            Recurrence::Fixed {
                interval,
                until,
                count,
            } => FfiRecurrence::Fixed {
                interval: interval.clone(),
                until: until.as_ref().map(to_iso_date),
                count: *count,
            },
            Recurrence::Flexible {
                interval,
                until,
                count,
            } => FfiRecurrence::Flexible {
                interval: interval.clone(),
                until: until.as_ref().map(to_iso_date),
                count: *count,
            },
            Recurrence::Rrule { rrule, dtstart } => FfiRecurrence::Rrule {
                rrule: rrule.clone(),
                dtstart: to_iso_date(dtstart),
            },
        }
    }
}

impl TryFrom<FfiRecurrence> for Recurrence {
    type Error = FfiConversionError;

    fn try_from(r: FfiRecurrence) -> Result<Self, Self::Error> {
        Ok(match r {
            FfiRecurrence::Fixed {
                interval,
                until,
                count,
            } => Recurrence::Fixed {
                interval,
                until: until
                    .as_deref()
                    .map(|v| from_iso_date("recurrence.until", v))
                    .transpose()?,
                count,
            },
            FfiRecurrence::Flexible {
                interval,
                until,
                count,
            } => Recurrence::Flexible {
                interval,
                until: until
                    .as_deref()
                    .map(|v| from_iso_date("recurrence.until", v))
                    .transpose()?,
                count,
            },
            FfiRecurrence::Rrule { rrule, dtstart } => Recurrence::Rrule {
                rrule,
                dtstart: from_iso_date("recurrence.dtstart", &dtstart)?,
            },
        })
    }
}

/// `Node`, boundary-safe: every `DateTime<Utc>` is an RFC3339 string, every
/// `NaiveDate` an ISO-8601 (`YYYY-MM-DD`) string, `anchor` uses
/// `FfiAnnotationAnchor`. Field order and names otherwise match `Node`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiNode {
    pub id: crate::types::NodeId,
    pub node_type: crate::types::NodeType,
    pub created: String,
    pub modified: String,
    pub schema_version: u32,

    pub lifecycle: Option<crate::types::Lifecycle>,
    pub archived_at: Option<String>,
    pub domain: Option<String>,
    pub tags: Vec<String>,
    pub relations: Vec<crate::types::Relation>,
    pub deleted_at: Option<String>,
    pub is_template: bool,

    pub distillation_level: Option<crate::types::DistillationLevel>,
    pub status: Option<String>,
    pub priority: Option<crate::types::Priority>,
    pub scheduled_date: Option<String>,
    pub due_date: Option<String>,
    pub estimated_pomodoros: Option<u32>,
    pub actual_pomodoros: Option<u32>,
    pub recurrence: Option<FfiRecurrence>,
    pub recurrence_occurrences: Option<u32>,
    pub checklist: Vec<crate::types::ChecklistItem>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub external_id: Option<String>,
    pub project_status: Option<crate::types::ProjectStatus>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub source_url: Option<String>,
    pub read_status: Option<String>,
    pub reminder_text: Option<String>,
    pub fire_at: Option<String>,
    pub reminder_status: Option<String>,
    pub resolved: bool,
    pub anchor: Option<FfiAnnotationAnchor>,
    pub pinned: Vec<crate::types::NodeId>,
    pub active_filter: Option<String>,
    pub default_view: Option<String>,
    pub theme: Option<String>,
    pub ink_attachment: Option<String>,
    pub date: Option<String>,
}

impl From<&Node> for FfiNode {
    fn from(n: &Node) -> Self {
        FfiNode {
            id: n.id.clone(),
            node_type: n.node_type.clone(),
            created: to_rfc3339(&n.created),
            modified: to_rfc3339(&n.modified),
            schema_version: n.schema_version,

            lifecycle: n.lifecycle.clone(),
            archived_at: n.archived_at.as_ref().map(to_rfc3339),
            domain: n.domain.clone(),
            tags: n.tags.clone(),
            relations: n.relations.clone(),
            deleted_at: n.deleted_at.as_ref().map(to_rfc3339),
            is_template: n.is_template,

            distillation_level: n.distillation_level.clone(),
            status: n.status.clone(),
            priority: n.priority.clone(),
            scheduled_date: n.scheduled_date.as_ref().map(to_iso_date),
            due_date: n.due_date.as_ref().map(to_iso_date),
            estimated_pomodoros: n.estimated_pomodoros,
            actual_pomodoros: n.actual_pomodoros,
            recurrence: n.recurrence.as_ref().map(FfiRecurrence::from),
            recurrence_occurrences: n.recurrence_occurrences,
            checklist: n.checklist.clone(),
            start: n.start.as_ref().map(to_rfc3339),
            end: n.end.as_ref().map(to_rfc3339),
            external_id: n.external_id.clone(),
            project_status: n.project_status.clone(),
            start_date: n.start_date.as_ref().map(to_iso_date),
            target_date: n.target_date.as_ref().map(to_iso_date),
            source_url: n.source_url.clone(),
            read_status: n.read_status.clone(),
            reminder_text: n.reminder_text.clone(),
            fire_at: n.fire_at.as_ref().map(to_rfc3339),
            reminder_status: n.reminder_status.clone(),
            resolved: n.resolved,
            anchor: n.anchor.as_ref().map(FfiAnnotationAnchor::from),
            pinned: n.pinned.clone(),
            active_filter: n.active_filter.clone(),
            default_view: n.default_view.clone(),
            theme: n.theme.clone(),
            ink_attachment: n.ink_attachment.clone(),
            date: n.date.as_ref().map(to_iso_date),
        }
    }
}

impl TryFrom<FfiNode> for Node {
    type Error = FfiConversionError;

    fn try_from(f: FfiNode) -> Result<Self, Self::Error> {
        Ok(Node {
            id: f.id,
            node_type: f.node_type,
            created: from_rfc3339("created", &f.created)?,
            modified: from_rfc3339("modified", &f.modified)?,
            schema_version: f.schema_version,

            lifecycle: f.lifecycle,
            archived_at: f
                .archived_at
                .as_deref()
                .map(|v| from_rfc3339("archived_at", v))
                .transpose()?,
            domain: f.domain,
            tags: f.tags,
            relations: f.relations,
            deleted_at: f
                .deleted_at
                .as_deref()
                .map(|v| from_rfc3339("deleted_at", v))
                .transpose()?,
            is_template: f.is_template,

            distillation_level: f.distillation_level,
            status: f.status,
            priority: f.priority,
            scheduled_date: f
                .scheduled_date
                .as_deref()
                .map(|v| from_iso_date("scheduled_date", v))
                .transpose()?,
            due_date: f
                .due_date
                .as_deref()
                .map(|v| from_iso_date("due_date", v))
                .transpose()?,
            estimated_pomodoros: f.estimated_pomodoros,
            actual_pomodoros: f.actual_pomodoros,
            recurrence: f.recurrence.map(Recurrence::try_from).transpose()?,
            recurrence_occurrences: f.recurrence_occurrences,
            checklist: f.checklist,
            start: f
                .start
                .as_deref()
                .map(|v| from_rfc3339("start", v))
                .transpose()?,
            end: f
                .end
                .as_deref()
                .map(|v| from_rfc3339("end", v))
                .transpose()?,
            external_id: f.external_id,
            project_status: f.project_status,
            start_date: f
                .start_date
                .as_deref()
                .map(|v| from_iso_date("start_date", v))
                .transpose()?,
            target_date: f
                .target_date
                .as_deref()
                .map(|v| from_iso_date("target_date", v))
                .transpose()?,
            source_url: f.source_url,
            read_status: f.read_status,
            reminder_text: f.reminder_text,
            fire_at: f
                .fire_at
                .as_deref()
                .map(|v| from_rfc3339("fire_at", v))
                .transpose()?,
            reminder_status: f.reminder_status,
            resolved: f.resolved,
            anchor: f.anchor.map(FfiAnnotationAnchor::try_into).transpose()?,
            pinned: f.pinned,
            active_filter: f.active_filter,
            default_view: f.default_view,
            theme: f.theme,
            ink_attachment: f.ink_attachment,
            date: f
                .date
                .as_deref()
                .map(|v| from_iso_date("date", v))
                .transpose()?,
        })
    }
}

/// FFI spike (ADR-031): round-trips a full `Node` — including the fields that
/// forced this DTO layer (timestamps, dates, `crdt_position` YAML) — through
/// the FFI boundary as `FfiNode`, proving the conversion is correct both ways.
#[uniffi::export]
pub fn round_trip_node(ffi: FfiNode) -> Result<FfiNode, FfiConversionError> {
    let node: Node = ffi.try_into()?;
    Ok(FfiNode::from(&node))
}

// ---------------------------------------------------------------------------
// The real Engine API surface (ADR-031) — everything above this point was
// spike scope proving the toolchain and the DTO pattern; this is native
// shells' actual entry point into iris-core. One `FfiEngine` object wraps
// `Engine` behind a `Mutex` (UniFFI objects hand out `Arc<Self>`, and every
// `Engine` method needs `&mut self` for its own write path); every method
// here is a thin translation to/from the FFI-safe types defined above and
// elsewhere in this module, with all the actual logic staying in `engine.rs`
// and the query-layer modules (`views`/`search`/`distillation`/`activation`/
// `dependencies`) unchanged.
// ---------------------------------------------------------------------------

use crate::cache::CachedNode;
use crate::engine::Engine;
use crate::integrity::IntegrityReport;
use crate::types::{DistillationLevel, NodeId, ProjectStatus};
use std::sync::{Arc, Mutex};

/// Every `Engine` method's error collapses to this at the boundary — native
/// callers get a message, not a typed variant tree that would have to mirror
/// `IrisError` exactly (and drift from it) on every target language.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiEngineError {
    #[error("{0}")]
    Failed(String),
}

impl From<crate::error::IrisError> for FfiEngineError {
    fn from(e: crate::error::IrisError) -> Self {
        FfiEngineError::Failed(e.to_string())
    }
}

/// `ParsedNode` with `node` as `FfiNode` — the read-side counterpart to
/// passing an `FfiNode` into a write method.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiParsedNode {
    pub node: FfiNode,
    pub body: String,
}

impl TryFrom<crate::parser::ParsedNode> for FfiParsedNode {
    type Error = FfiConversionError;

    fn try_from(p: crate::parser::ParsedNode) -> Result<Self, Self::Error> {
        Ok(FfiParsedNode {
            node: FfiNode::from(&p.node),
            body: p.body,
        })
    }
}

/// `Engine::restore_from_backup`'s return value can't be a UniFFI
/// constructor (constructors return `Self`/`Result<Self, E>`, not a tuple),
/// so it's a plain exported function returning this record instead.
#[derive(uniffi::Record)]
pub struct FfiRestoreResult {
    pub engine: Arc<FfiEngine>,
    pub report: IntegrityReport,
}

#[derive(uniffi::Object)]
pub struct FfiEngine {
    inner: Mutex<Engine>,
}

impl FfiEngine {
    fn wrap(engine: Engine) -> Arc<Self> {
        Arc::new(FfiEngine {
            inner: Mutex::new(engine),
        })
    }

    /// The lock is only ever held for the duration of one `Engine` call —
    /// never across an FFI round-trip — so a poisoned lock means a prior
    /// call panicked inside `iris-core` itself, a bug worth surfacing loudly
    /// rather than quietly recovering from.
    fn lock(&self) -> std::sync::MutexGuard<'_, Engine> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl FfiEngine {
    #[uniffi::constructor]
    pub fn init(path: String) -> Result<Arc<Self>, FfiEngineError> {
        Ok(Self::wrap(Engine::init(path)?))
    }

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, FfiEngineError> {
        Ok(Self::wrap(Engine::open(path)?))
    }

    // -- node CRUD --

    pub fn create_node(
        &self,
        rel_path: String,
        node: FfiNode,
        body: String,
    ) -> Result<(), FfiEngineError> {
        let node: Node = node.try_into().map_err(ffi_conv_err)?;
        Ok(self.lock().create_node(rel_path, &node, &body)?)
    }

    pub fn read_node(&self, rel_path: String) -> Result<FfiParsedNode, FfiEngineError> {
        let parsed = self.lock().read_node(rel_path)?;
        parsed.try_into().map_err(ffi_conv_err)
    }

    pub fn update_node(&self, rel_path: String, node: FfiNode) -> Result<(), FfiEngineError> {
        let node: Node = node.try_into().map_err(ffi_conv_err)?;
        Ok(self.lock().update_node(rel_path, &node)?)
    }

    pub fn delete_node(&self, rel_path: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().delete_node(rel_path)?)
    }

    pub fn restore_node(&self, rel_path: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().restore_node(rel_path)?)
    }

    pub fn instantiate_template(
        &self,
        template_rel_path: String,
        new_rel_path: String,
    ) -> Result<(), FfiEngineError> {
        Ok(self
            .lock()
            .instantiate_template(template_rel_path, new_rel_path)?)
    }

    // -- anchored comments --

    pub fn add_comment(
        &self,
        target_rel_path: String,
        text_fragment: String,
        comment_rel_path: String,
        body: String,
    ) -> Result<(), FfiEngineError> {
        Ok(self
            .lock()
            .add_comment(target_rel_path, &text_fragment, comment_rel_path, &body)?)
    }

    pub fn reply_to_annotation(
        &self,
        parent_rel_path: String,
        reply_rel_path: String,
        body: String,
    ) -> Result<(), FfiEngineError> {
        Ok(self
            .lock()
            .reply_to_annotation(parent_rel_path, reply_rel_path, &body)?)
    }

    pub fn set_annotation_resolved(
        &self,
        rel_path: String,
        resolved: bool,
    ) -> Result<(), FfiEngineError> {
        Ok(self.lock().set_annotation_resolved(rel_path, resolved)?)
    }

    // -- distillation / planning --

    pub fn set_distillation_level(
        &self,
        rel_path: String,
        level: DistillationLevel,
    ) -> Result<(), FfiEngineError> {
        Ok(self.lock().set_distillation_level(rel_path, level)?)
    }

    pub fn set_project_status(
        &self,
        rel_path: String,
        status: ProjectStatus,
    ) -> Result<bool, FfiEngineError> {
        Ok(self.lock().set_project_status(rel_path, status)?)
    }

    pub fn log_pomodoro(&self, rel_path: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().log_pomodoro(rel_path)?)
    }

    pub fn complete_task(&self, rel_path: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().complete_task(rel_path)?)
    }

    // -- Trash / retention --

    pub fn purge_expired_trash_default(&self) -> Result<u32, FfiEngineError> {
        Ok(self.lock().purge_expired_trash_default()? as u32)
    }

    pub fn purge_expired_trash_days(&self, days: u32) -> Result<u32, FfiEngineError> {
        Ok(self
            .lock()
            .purge_expired_trash(chrono::Duration::days(i64::from(days)))? as u32)
    }

    // -- undo/redo --

    pub fn undo(&self) -> Result<bool, FfiEngineError> {
        Ok(self.lock().undo()?)
    }

    pub fn redo(&self) -> Result<bool, FfiEngineError> {
        Ok(self.lock().redo()?)
    }

    pub fn can_undo(&self) -> bool {
        self.lock().can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.lock().can_redo()
    }

    // -- checkpoints / branching --

    pub fn create_checkpoint(&self, name: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().create_checkpoint(&name)?)
    }

    pub fn list_checkpoints(&self) -> Result<Vec<String>, FfiEngineError> {
        Ok(self.lock().list_checkpoints()?)
    }

    pub fn create_branch(&self, name: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().create_branch(&name)?)
    }

    pub fn list_branches(&self) -> Result<Vec<String>, FfiEngineError> {
        Ok(self.lock().list_branches()?)
    }

    pub fn current_branch(&self) -> Result<Option<String>, FfiEngineError> {
        Ok(self.lock().current_branch()?)
    }

    pub fn checkout(&self, name: String) -> Result<(), FfiEngineError> {
        Ok(self.lock().checkout(&name)?)
    }

    // -- cache / integrity --

    pub fn rebuild_cache(&self) -> Result<(), FfiEngineError> {
        Ok(self.lock().rebuild_cache()?)
    }

    pub fn check_integrity(&self) -> Result<IntegrityReport, FfiEngineError> {
        Ok(self.lock().check_integrity()?)
    }

    pub fn vault_root(&self) -> String {
        self.lock().vault_root().display().to_string()
    }

    // -- task views (ARCHITECTURE.md §12, `views.rs`) --

    pub fn inbox(&self) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::views::inbox(self.lock().cache())?)
    }

    pub fn today(&self, today: String) -> Result<Vec<CachedNode>, FfiEngineError> {
        let today = from_iso_date("today", &today).map_err(ffi_conv_err)?;
        Ok(crate::views::today(self.lock().cache(), today)?)
    }

    pub fn upcoming(&self, from: String, days: u32) -> Result<Vec<CachedNode>, FfiEngineError> {
        let from = from_iso_date("from", &from).map_err(ffi_conv_err)?;
        Ok(crate::views::upcoming(self.lock().cache(), from, days)?)
    }

    pub fn someday_maybe(&self) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::views::someday_maybe(self.lock().cache())?)
    }

    pub fn logbook(&self) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::views::logbook(self.lock().cache())?)
    }

    pub fn trash(&self) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::views::trash(self.lock().cache())?)
    }

    // -- basic search (`search.rs`) --

    pub fn search(
        &self,
        query: String,
        node_type: Option<String>,
        domain: Option<String>,
        tag: Option<String>,
    ) -> Result<Vec<CachedNode>, FfiEngineError> {
        let filters = crate::search::SearchFilters {
            node_type: node_type.as_deref(),
            domain: domain.as_deref(),
            tag: tag.as_deref(),
        };
        Ok(crate::search::search(
            self.lock().cache(),
            &query,
            &filters,
        )?)
    }

    // -- distillation queue (`distillation.rs`) --

    pub fn distillation_queue(
        &self,
        project_id: NodeId,
    ) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::distillation::queue(
            self.lock().cache(),
            &project_id,
        )?)
    }

    // -- guided project activation (`activation.rs`, ADR-023) --

    pub fn activation_environment(
        &self,
        project_id: NodeId,
    ) -> Result<crate::activation::ActivationEnvironment, FfiEngineError> {
        Ok(crate::activation::assemble(
            self.lock().cache(),
            &project_id,
        )?)
    }

    // -- task dependencies (`dependencies.rs`, ADR-017) --

    pub fn blocked_by(&self, node_id: NodeId) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::dependencies::blocked_by(
            self.lock().cache(),
            &node_id,
        )?)
    }

    pub fn blocked_by_incoming(&self, node_id: NodeId) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::dependencies::blocked_by_incoming(
            self.lock().cache(),
            &node_id,
        )?)
    }

    pub fn is_blocked(&self, node_id: NodeId) -> Result<bool, FfiEngineError> {
        Ok(crate::dependencies::is_blocked(
            self.lock().cache(),
            &node_id,
        )?)
    }

    pub fn blocks(&self, node_id: NodeId) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::dependencies::blocks(self.lock().cache(), &node_id)?)
    }

    pub fn depended_on_by(&self, node_id: NodeId) -> Result<Vec<CachedNode>, FfiEngineError> {
        Ok(crate::dependencies::depended_on_by(
            self.lock().cache(),
            &node_id,
        )?)
    }
}

/// The git half of restore-from-backup can't be a constructor (see
/// `FfiRestoreResult`'s doc comment), so it's exported as a plain function.
#[uniffi::export]
pub fn restore_from_backup(
    remote_url: String,
    dest_path: String,
) -> Result<FfiRestoreResult, FfiEngineError> {
    let (engine, report) = Engine::restore_from_backup(&remote_url, &dest_path)?;
    Ok(FfiRestoreResult {
        engine: FfiEngine::wrap(engine),
        report,
    })
}

fn ffi_conv_err(e: FfiConversionError) -> FfiEngineError {
    FfiEngineError::Failed(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{new_node_id, NodeType};

    fn sample_node() -> Node {
        Node {
            id: new_node_id(),
            node_type: NodeType::Custom("trading-journal-entry".to_string()),
            created: Utc::now(),
            modified: Utc::now(),
            schema_version: crate::types::CURRENT_SCHEMA_VERSION,
            lifecycle: Some(crate::types::Lifecycle::Active),
            archived_at: None,
            domain: Some("finance".to_string()),
            tags: vec!["a".to_string(), "b".to_string()],
            relations: vec![crate::types::Relation {
                rel_type: "related-to".to_string(),
                target: "01ABCDEF".to_string(),
            }],
            deleted_at: None,
            is_template: false,
            distillation_level: Some(crate::types::DistillationLevel::Bolded),
            status: None,
            priority: Some(crate::types::Priority::High),
            scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 9, 3).unwrap()),
            due_date: None,
            estimated_pomodoros: Some(4),
            actual_pomodoros: None,
            recurrence: Some(crate::types::Recurrence::Fixed {
                interval: "P1M".to_string(),
                until: Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
                count: Some(12),
            }),
            recurrence_occurrences: Some(2),
            checklist: vec![crate::types::ChecklistItem {
                text: "step 1".to_string(),
                done: true,
            }],
            start: None,
            end: None,
            external_id: None,
            project_status: None,
            start_date: None,
            target_date: None,
            source_url: None,
            read_status: None,
            reminder_text: None,
            fire_at: None,
            reminder_status: None,
            resolved: false,
            anchor: Some(AnnotationAnchor {
                text_fragment: Some("some text".to_string()),
                crdt_position: Some(serde_yaml::from_str("pos: [1, 2, 3]").unwrap()),
            }),
            pinned: vec![],
            active_filter: None,
            default_view: None,
            theme: None,
            ink_attachment: None,
            date: None,
        }
    }

    #[test]
    fn node_round_trips_through_ffi_dto_unchanged() {
        let original = sample_node();
        let ffi = FfiNode::from(&original);
        let back: Node = ffi.try_into().expect("valid FfiNode converts back");
        assert_eq!(original, back);
    }

    #[test]
    fn round_trip_node_fn_matches_manual_conversion() {
        let original = sample_node();
        let ffi = FfiNode::from(&original);
        let result = round_trip_node(ffi.clone()).expect("round trip succeeds");
        assert_eq!(result, ffi);
    }

    #[test]
    fn invalid_timestamp_string_is_rejected_not_panicked() {
        let mut ffi = FfiNode::from(&sample_node());
        ffi.created = "not-a-timestamp".to_string();
        let result: Result<Node, _> = ffi.try_into();
        assert!(matches!(
            result,
            Err(FfiConversionError::InvalidTimestamp { field, .. }) if field == "created"
        ));
    }

    #[test]
    fn invalid_date_string_is_rejected_not_panicked() {
        let mut ffi = FfiNode::from(&sample_node());
        ffi.scheduled_date = Some("2026-13-99".to_string());
        let result: Result<Node, _> = ffi.try_into();
        assert!(matches!(
            result,
            Err(FfiConversionError::InvalidDate { field, .. }) if field == "scheduled_date"
        ));
    }

    // -----------------------------------------------------------------
    // FfiEngine — the real surface, exercised the way a native shell would:
    // through Arc<FfiEngine>, FfiNode in and out, no direct Engine access.
    // -----------------------------------------------------------------

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-ffi-engine-test-{label}-{nanos}"));
            TempDir(path)
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn ffi_engine_create_read_update_round_trip() {
        let dir = TempDir::new("crud");
        let engine = FfiEngine::init(dir.path().to_string_lossy().into_owned()).unwrap();

        let node = FfiNode::from(&sample_node());
        engine
            .create_node(
                "notes/a.md".to_string(),
                node.clone(),
                "\n\nHello.\n".to_string(),
            )
            .unwrap();

        let read = engine.read_node("notes/a.md".to_string()).unwrap();
        assert_eq!(read.node.id, node.id);
        assert!(read.body.contains("Hello."));

        let mut updated = read.node.clone();
        updated.domain = Some("iris-dev".to_string());
        engine
            .update_node("notes/a.md".to_string(), updated)
            .unwrap();
        let after = engine.read_node("notes/a.md".to_string()).unwrap();
        assert_eq!(after.node.domain.as_deref(), Some("iris-dev"));
    }

    #[test]
    fn ffi_engine_undo_redo_and_checkpoints() {
        let dir = TempDir::new("undo-checkpoint");
        let engine = FfiEngine::init(dir.path().to_string_lossy().into_owned()).unwrap();

        engine
            .create_node(
                "notes/a.md".to_string(),
                FfiNode::from(&sample_node()),
                "\n".to_string(),
            )
            .unwrap();
        assert!(engine.can_undo());

        engine.create_checkpoint("v1".to_string()).unwrap();
        assert_eq!(engine.list_checkpoints().unwrap(), vec!["v1".to_string()]);

        assert!(engine.undo().unwrap());
        assert!(engine.read_node("notes/a.md".to_string()).is_err());
        assert!(engine.redo().unwrap());
        assert!(engine.read_node("notes/a.md".to_string()).is_ok());
    }

    #[test]
    fn ffi_engine_views_and_search_reflect_created_nodes() {
        let dir = TempDir::new("views");
        let engine = FfiEngine::init(dir.path().to_string_lossy().into_owned()).unwrap();

        let mut task = sample_node();
        task.node_type = NodeType::Task;
        engine
            .create_node(
                "tasks/a.md".to_string(),
                FfiNode::from(&task),
                "\n\nfindme\n".to_string(),
            )
            .unwrap();

        assert_eq!(engine.inbox().unwrap().len(), 1);
        assert_eq!(
            engine
                .search("findme".to_string(), None, None, None)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn ffi_engine_project_activation_and_distillation_queue() {
        let dir = TempDir::new("activation");
        let engine = FfiEngine::init(dir.path().to_string_lossy().into_owned()).unwrap();

        let mut project = sample_node();
        project.node_type = NodeType::Project;
        engine
            .create_node(
                "projects/p.md".to_string(),
                FfiNode::from(&project),
                "\n".to_string(),
            )
            .unwrap();
        let project_id = engine
            .read_node("projects/p.md".to_string())
            .unwrap()
            .node
            .id;

        let mut note = sample_node();
        note.relations = vec![crate::types::Relation {
            rel_type: "parent_project".to_string(),
            target: project_id.clone(),
        }];
        engine
            .create_node(
                "notes/raw.md".to_string(),
                FfiNode::from(&note),
                "\n".to_string(),
            )
            .unwrap();

        let queue = engine.distillation_queue(project_id.clone()).unwrap();
        assert_eq!(queue.len(), 1);

        let env = engine.activation_environment(project_id).unwrap();
        assert_eq!(env.distillation_queue.len(), 1);
    }

    #[test]
    fn ffi_restore_from_backup_clones_and_reports_clean() {
        let source_dir = TempDir::new("restore-source");
        let source = FfiEngine::init(source_dir.path().to_string_lossy().into_owned()).unwrap();
        // A clean node, not `sample_node()` — that fixture deliberately
        // carries a dangling relation for the DTO round-trip tests above,
        // which would (correctly) fail this test's integrity check.
        let mut clean_node = sample_node();
        clean_node.relations = vec![];
        source
            .create_node(
                "notes/a.md".to_string(),
                FfiNode::from(&clean_node),
                "\n\nBackup me.\n".to_string(),
            )
            .unwrap();

        let dest_dir = TempDir::new("restore-dest");
        let result = restore_from_backup(
            source_dir.path().to_string_lossy().into_owned(),
            dest_dir.path().to_string_lossy().into_owned(),
        )
        .unwrap();

        assert!(result.report.is_clean());
        let node = result.engine.read_node("notes/a.md".to_string()).unwrap();
        assert!(node.body.contains("Backup me."));
    }
}
