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

use crate::types::{AnnotationAnchor, Node};
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
    pub recurrence: Option<crate::types::Recurrence>,
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
            recurrence: n.recurrence.clone(),
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
            recurrence: f.recurrence,
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
            recurrence: Some(crate::types::Recurrence::Rrule {
                rrule: "FREQ=DAILY".to_string(),
            }),
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
}
