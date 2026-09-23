//! `iris` — a scriptable command-line client for an Iris vault.
//!
//! Deliberately a thin wrapper: every subcommand is a handful of lines
//! calling straight into `iris_core::engine::Engine`, no logic of its own
//! (ADR-036). This is a *client* of the core, on equal footing with the
//! native GUI shells and the MCP server — not a special or lesser way to
//! use Iris.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use clap::{Parser, Subcommand};
use iris_core::engine::Engine;
use iris_core::error::IrisError;
use iris_core::search::{self, SearchFilters};
use iris_core::types::{new_node_id, Node, NodeType, CURRENT_SCHEMA_VERSION};

#[derive(Parser)]
#[command(name = "iris", version, about = "Scriptable client for an Iris vault")]
struct Cli {
    /// Vault directory (defaults to the current directory, like `git -C`).
    #[arg(short = 'C', long = "vault", global = true, default_value = ".")]
    vault: PathBuf,

    /// Print machine-readable JSON instead of a human-readable table.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new vault in the given directory (git init + .iris/ setup).
    Init,
    /// Create a new node.
    Create {
        /// Path to the new node, relative to the vault root (e.g. notes/idea.md).
        rel_path: String,
        /// Node type (note, task, project, area, resource, event, ...). Unknown
        /// values become a `Custom` type per SCHEMA_SPEC's plugin-type fallback.
        #[arg(short = 't', long = "type", default_value = "note")]
        node_type: String,
        /// Repeatable: --tag work --tag idea
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// Body text. Omit and pipe stdin instead for longer content.
        #[arg(long)]
        body: Option<String>,
    },
    /// Print a node's frontmatter and body.
    Read {
        /// Path to the node, relative to the vault root.
        rel_path: String,
    },
    /// Search nodes by text, optionally narrowed by type/domain/tag. An empty
    /// query with filters (or no arguments at all) lists everything matching.
    Search {
        /// Substring matched against path and body (case-insensitive).
        query: Option<String>,
        #[arg(short = 't', long = "type")]
        node_type: Option<String>,
        #[arg(short = 'd', long)]
        domain: Option<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Mark a task done (or, if already done, reopen it — same toggle the
    /// task-lens views use).
    Done { rel_path: String },
    /// Soft-delete a node (recoverable via `restore` or from Trash).
    Rm { rel_path: String },
    /// Recover a soft-deleted node.
    Restore { rel_path: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("iris: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), IrisError> {
    match cli.command {
        Command::Init => {
            Engine::init(&cli.vault)?;
            println!("Initialized vault at {}", cli.vault.display());
            Ok(())
        }
        Command::Create {
            rel_path,
            node_type,
            tags,
            body,
        } => cmd_create(&cli.vault, &rel_path, &node_type, tags, body),
        Command::Read { rel_path } => cmd_read(&cli.vault, &rel_path, cli.json),
        Command::Search {
            query,
            node_type,
            domain,
            tag,
        } => cmd_search(&cli.vault, query, node_type, domain, tag, cli.json),
        Command::Done { rel_path } => cmd_done(&cli.vault, &rel_path),
        Command::Rm { rel_path } => {
            let mut engine = Engine::open(&cli.vault)?;
            engine.delete_node(&rel_path)?;
            println!("Trashed {rel_path}");
            Ok(())
        }
        Command::Restore { rel_path } => {
            let mut engine = Engine::open(&cli.vault)?;
            engine.restore_node(&rel_path)?;
            println!("Restored {rel_path}");
            Ok(())
        }
    }
}

fn cmd_create(
    vault: &PathBuf,
    rel_path: &str,
    node_type: &str,
    tags: Vec<String>,
    body: Option<String>,
) -> Result<(), IrisError> {
    let node_type: NodeType = serde_yaml::from_str(node_type)
        .map_err(|e| IrisError::Validation(format!("invalid node type: {e}")))?;

    let body = match body {
        Some(b) => b,
        None => {
            let mut stdin_body = String::new();
            // No TTY check: an empty read (no piped input) just creates an
            // empty-body node, same as `git commit` with an empty editor buffer.
            std::io::stdin()
                .read_to_string(&mut stdin_body)
                .map_err(IrisError::Io)?;
            stdin_body
        }
    };
    // `Engine::create_node`'s body is appended directly after the closing
    // `---` with no newline inserted for you (see `engine.rs::render`).
    let body = format!("\n{body}\n");

    let now = Utc::now();
    let node = Node {
        id: new_node_id(),
        node_type,
        created: now,
        modified: now,
        schema_version: CURRENT_SCHEMA_VERSION,
        lifecycle: None,
        archived_at: None,
        domain: None,
        tags,
        relations: vec![],
        deleted_at: None,
        is_template: false,
        distillation_level: None,
        status: None,
        priority: None,
        scheduled_date: None,
        due_date: None,
        estimated_pomodoros: None,
        actual_pomodoros: None,
        recurrence: None,
        recurrence_occurrences: None,
        checklist: vec![],
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
        anchor: None,
        pinned: vec![],
        active_filter: None,
        default_view: None,
        theme: None,
        ink_attachment: None,
        date: None,
        symbol: None,
        entry: None,
        exit: None,
        pnl: None,
        r_multiple: None,
    };

    let mut engine = Engine::open(vault)?;
    engine.create_node(rel_path, &node, &body)?;
    println!("Created {rel_path}");
    Ok(())
}

fn cmd_read(vault: &PathBuf, rel_path: &str, json: bool) -> Result<(), IrisError> {
    let engine = Engine::open(vault)?;
    let parsed = engine.read_node(rel_path)?;
    if json {
        let out = serde_json::json!({
            "node": parsed.node,
            "body": parsed.body,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("---\n{}\n---{}", parsed.raw_frontmatter, parsed.body);
    }
    Ok(())
}

fn cmd_search(
    vault: &PathBuf,
    query: Option<String>,
    node_type: Option<String>,
    domain: Option<String>,
    tag: Option<String>,
    json: bool,
) -> Result<(), IrisError> {
    let engine = Engine::open(vault)?;
    let filters = SearchFilters {
        node_type: node_type.as_deref(),
        domain: domain.as_deref(),
        tag: tag.as_deref(),
    };
    let results = search::search(engine.cache(), query.as_deref().unwrap_or(""), &filters)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    } else if results.is_empty() {
        println!("No matches.");
    } else {
        for n in &results {
            let status = n.status.as_deref().unwrap_or("-");
            println!("{}\t{}\t{}", n.node_type, status, n.path);
        }
    }
    Ok(())
}

fn cmd_done(vault: &PathBuf, rel_path: &str) -> Result<(), IrisError> {
    let mut engine = Engine::open(vault)?;
    engine.complete_task(rel_path)?;
    println!("Toggled {rel_path}");
    Ok(())
}
