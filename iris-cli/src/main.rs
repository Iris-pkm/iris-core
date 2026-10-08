//! `iris` — a scriptable command-line client for an Iris vault.
//!
//! Deliberately a thin wrapper: every subcommand is a handful of lines
//! calling straight into `iris_core::engine::Engine`, no logic of its own
//! (ADR-036). This is a *client* of the core, on equal footing with the
//! native GUI shells and the MCP server — not a special or lesser way to
//! use Iris.

mod mcp_server;

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use iris_core::engine::Engine;
use iris_core::error::IrisError;
use iris_core::search::{self, SearchFilters};
use iris_core::types::{Node, NodeType, Priority, ProjectStatus, Relation};

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
    /// Update a node's metadata fields. Leaves the body untouched unless
    /// --body is given — use `edit` to change the body interactively.
    Update {
        rel_path: String,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "clear-status")]
        clear_status: bool,
        /// Project lifecycle: someday, planned, active, paused, completed or
        /// cancelled. Projects only; goes through the ADR-018 state machine
        /// (illegal transitions are rejected). Use this, not --status, on projects.
        #[arg(long = "project-status")]
        project_status: Option<String>,
        /// urgent, high, normal, or low.
        #[arg(long)]
        priority: Option<String>,
        #[arg(long = "clear-priority")]
        clear_priority: bool,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long = "clear-domain")]
        clear_domain: bool,
        /// YYYY-MM-DD.
        #[arg(long = "scheduled-date")]
        scheduled_date: Option<String>,
        #[arg(long = "clear-scheduled-date")]
        clear_scheduled_date: bool,
        /// YYYY-MM-DD.
        #[arg(long = "due-date")]
        due_date: Option<String>,
        #[arg(long = "clear-due-date")]
        clear_due_date: bool,
        /// Repeatable: --add-tag work --add-tag q3
        #[arg(long = "add-tag")]
        add_tags: Vec<String>,
        /// Repeatable.
        #[arg(long = "remove-tag")]
        remove_tags: Vec<String>,
        /// Repeatable: --add-relation type:target-id (or type:path/to/note.md).
        #[arg(long = "add-relation")]
        add_relations: Vec<String>,
        /// Repeatable: --remove-relation type:target-id (or type:path/to/note.md).
        #[arg(long = "remove-relation")]
        remove_relations: Vec<String>,
        /// Replace the body outright (non-interactive). Use `edit` instead
        /// to open $EDITOR on the current body.
        #[arg(long)]
        body: Option<String>,
    },
    /// Open $EDITOR (or $VISUAL) on a node's body and save it back on exit,
    /// the same pattern `git commit` uses. Frontmatter is untouched.
    Edit { rel_path: String },
    /// Mark a task done (or, if already done, reopen it — same toggle the
    /// task-lens views use).
    Done { rel_path: String },
    /// Soft-delete a node (recoverable via `restore` or from Trash).
    Rm { rel_path: String },
    /// Recover a soft-deleted node.
    Restore { rel_path: String },
    /// Run an MCP server over stdio, exposing this vault to external agents
    /// (search_notes, get_note, create_note, update_note, whoami).
    McpServer {
        /// Refuse create_note/update_note: expose the vault to read-only agents.
        #[arg(long)]
        read_only: bool,
    },
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
        Command::Update {
            rel_path,
            status,
            clear_status,
            project_status,
            priority,
            clear_priority,
            domain,
            clear_domain,
            scheduled_date,
            clear_scheduled_date,
            due_date,
            clear_due_date,
            add_tags,
            remove_tags,
            add_relations,
            remove_relations,
            body,
        } => cmd_update(
            &cli.vault,
            &rel_path,
            UpdateFields {
                status,
                clear_status,
                project_status,
                priority,
                clear_priority,
                domain,
                clear_domain,
                scheduled_date,
                clear_scheduled_date,
                due_date,
                clear_due_date,
                add_tags,
                remove_tags,
                add_relations,
                remove_relations,
                body,
            },
        ),
        Command::Edit { rel_path } => cmd_edit(&cli.vault, &rel_path),
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
        Command::McpServer { read_only } => {
            let rt = tokio::runtime::Runtime::new().map_err(IrisError::Io)?;
            rt.block_on(mcp_server::serve(cli.vault, read_only))
                .map_err(|e| IrisError::Validation(e.to_string()))
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

    let mut node = Node::new(node_type);
    node.tags = tags;

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

struct UpdateFields {
    status: Option<String>,
    clear_status: bool,
    project_status: Option<String>,
    priority: Option<String>,
    clear_priority: bool,
    domain: Option<String>,
    clear_domain: bool,
    scheduled_date: Option<String>,
    clear_scheduled_date: bool,
    due_date: Option<String>,
    clear_due_date: bool,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
    add_relations: Vec<String>,
    remove_relations: Vec<String>,
    body: Option<String>,
}

fn relation_arg(engine: &Engine, spec: &str) -> Result<Relation, IrisError> {
    let (rel_type, target) = spec.split_once(':').ok_or_else(|| {
        IrisError::Validation(format!("invalid relation {spec:?} (want type:target-id)"))
    })?;
    let target = if target.contains('/') || target.ends_with(".md") {
        engine.read_node(target)?.node.id
    } else {
        target.to_string()
    };
    Ok(Relation {
        rel_type: rel_type.into(),
        target,
    })
}

fn parse_date(s: &str) -> Result<NaiveDate, IrisError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| IrisError::Validation(format!("invalid date {s:?} (want YYYY-MM-DD): {e}")))
}

fn cmd_update(vault: &PathBuf, rel_path: &str, fields: UpdateFields) -> Result<(), IrisError> {
    let mut engine = Engine::open(vault)?;
    let mut node = engine.read_node(rel_path)?.node;

    // `status` is the task workflow field; on a project it would be written
    // but never read by the state machine (ADR-039), so refuse instead.
    if node.node_type == NodeType::Project && (fields.status.is_some() || fields.clear_status) {
        return Err(IrisError::Validation(
            "projects use --project-status (lifecycle), not --status".into(),
        ));
    }
    // Validate relations before a project-status transition can write. Keep
    // the edited set while re-reading the project after that transition.
    let additions = fields
        .add_relations
        .iter()
        .map(|s| relation_arg(&engine, s))
        .collect::<Result<Vec<_>, _>>()?;
    let removals = fields
        .remove_relations
        .iter()
        .map(|s| relation_arg(&engine, s))
        .collect::<Result<Vec<_>, _>>()?;
    engine.edit_relations(&mut node, &additions, &removals)?;
    let relations = node.relations.clone();
    // Do the state-machine move first: it validates before writing anything,
    // so an illegal transition leaves the node untouched.
    let mut moved = None;
    if let Some(s) = &fields.project_status {
        let target: ProjectStatus = s.parse().map_err(IrisError::Validation)?;
        let activated = engine.set_project_status(rel_path, target)?;
        node = engine.read_node(rel_path)?.node;
        node.relations = relations;
        moved = Some((s.clone(), activated));
    }
    let only_project_status = moved.is_some()
        && !fields.clear_status
        && fields.priority.is_none()
        && !fields.clear_priority
        && fields.domain.is_none()
        && !fields.clear_domain
        && fields.scheduled_date.is_none()
        && !fields.clear_scheduled_date
        && fields.due_date.is_none()
        && !fields.clear_due_date
        && fields.add_tags.is_empty()
        && fields.remove_tags.is_empty()
        && fields.add_relations.is_empty()
        && fields.remove_relations.is_empty()
        && fields.body.is_none();
    if only_project_status {
        let (s, activated) = moved.unwrap();
        println!(
            "Project status -> {s}{}",
            if activated { " (activated)" } else { "" }
        );
        return Ok(());
    }

    if fields.clear_status {
        node.status = None;
    } else if let Some(s) = fields.status {
        node.status = Some(s);
    }

    if fields.clear_priority {
        node.priority = None;
    } else if let Some(p) = fields.priority {
        let priority: Priority = serde_yaml::from_str(&p)
            .map_err(|e| IrisError::Validation(format!("invalid priority: {e}")))?;
        node.priority = Some(priority);
    }

    if fields.clear_domain {
        node.domain = None;
    } else if let Some(d) = fields.domain {
        node.domain = Some(d);
    }

    if fields.clear_scheduled_date {
        node.scheduled_date = None;
    } else if let Some(d) = fields.scheduled_date {
        node.scheduled_date = Some(parse_date(&d)?);
    }

    if fields.clear_due_date {
        node.due_date = None;
    } else if let Some(d) = fields.due_date {
        node.due_date = Some(parse_date(&d)?);
    }

    node.tags.retain(|t| !fields.remove_tags.contains(t));
    for tag in fields.add_tags {
        if !node.tags.contains(&tag) {
            node.tags.push(tag);
        }
    }

    match fields.body {
        Some(b) => engine.update_node_with_body(rel_path, &node, &format!("\n{b}\n"))?,
        None => engine.update_node(rel_path, &node)?,
    }
    match moved {
        Some((s, _)) => println!("Updated {rel_path} (project status -> {s})"),
        None => println!("Updated {rel_path}"),
    }
    Ok(())
}

fn cmd_edit(vault: &PathBuf, rel_path: &str) -> Result<(), IrisError> {
    let mut engine = Engine::open(vault)?;
    let parsed = engine.read_node(rel_path)?;

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let tmp_path = std::env::temp_dir().join(format!(
        "iris-edit-{}-{}.md",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    std::fs::write(&tmp_path, &parsed.body)?;

    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .map_err(|e| IrisError::Validation(format!("failed to launch {editor}: {e}")))?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(IrisError::Validation(format!(
            "{editor} exited with {status}, not saving"
        )));
    }

    let new_body = std::fs::read_to_string(&tmp_path)?;
    let _ = std::fs::remove_file(&tmp_path);

    if new_body == parsed.body {
        println!("No changes.");
        return Ok(());
    }
    engine.update_node_with_body(rel_path, &parsed.node, &new_body)?;
    println!("Updated {rel_path}");
    Ok(())
}
