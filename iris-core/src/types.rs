//! Core types for Iris — the typed node model.
//!
//! Every piece of content in Iris is a **node**: a markdown file with YAML frontmatter.
//! See `SCHEMA_SPEC.md` for the full specification.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// A unique node identifier — a ULID (sortable by creation time).
pub type NodeId = String;

/// Schema version for format migration support (SCHEMA_SPEC §9).
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Node type enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeType {
    Note,
    Task,
    Event,
    Project,
    Area,
    Resource,
    Space,
    Annotation,
    InkNote,
    Reminder,
    DailyNote,
    TradingJournalEntry,
    MusicIdea,
    ReadingItem,
    #[serde(untagged)]
    Custom(String),
}

// ---------------------------------------------------------------------------
// Lifecycle (ADR-016)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Active,
    Archived,
}

impl Default for Lifecycle {
    fn default() -> Self { Self::Active }
}

// ---------------------------------------------------------------------------
// Distillation level
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistillationLevel {
    Raw,
    Bolded,
    Highlighted,
    Summarized,
}

impl Default for DistillationLevel {
    fn default() -> Self { Self::Raw }
}

// ---------------------------------------------------------------------------
// Relations (ADR-017 — canonical direction only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: NodeId,
}

// ---------------------------------------------------------------------------
// Recurrence (provisional)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Recurrence {
    Fixed { interval: String },
    Flexible { interval: String },
    Rrule { rrule: String },
}

// ---------------------------------------------------------------------------
// Small supporting types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Urgent,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Someday,
    Planned,
    Active,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationAnchor {
    pub text_fragment: Option<String>,
    pub crdt_position: Option<serde_yaml::Value>,
}

// ---------------------------------------------------------------------------
// The typed Node (deserialized from YAML frontmatter for convenient access)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    // -- required shared --
    pub id: NodeId,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    // -- optional shared --
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<Lifecycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_template: bool,

    // -- type-specific (all optional at this level; validation ensures correctness per type) --
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distillation_level: Option<DistillationLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_pomodoros: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_pomodoros: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Recurrence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklist: Vec<ChecklistItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_status: Option<ProjectStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder_status: Option<String>,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<AnnotationAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pinned: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ink_attachment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDate>,
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}
