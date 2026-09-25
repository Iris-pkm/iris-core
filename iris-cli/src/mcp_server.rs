//! MCP server exposing a vault to external agents (ARCHITECTURE.md §8,
//! ADR-037) — the reverse direction of Iris acting as an MCP *client*. A
//! thin adapter over `iris_core::Engine`, mirroring the same read/write
//! shape `iris-cli`'s own commands and the FFI layer already expose: no new
//! backend logic lives here.
//!
//! Two different MCP clients connecting to a server started against the
//! same vault path are, by construction, reading and writing the same
//! graph — the literal mechanism behind `OVERVIEW.md`'s "single source of
//! truth for AI" claim (ADR-035), not a slogan.

use std::path::PathBuf;

use iris_core::engine::Engine;
use iris_core::error::IrisError;
use iris_core::search::{self, SearchFilters};
use iris_core::types::{Node, NodeType, Priority};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_handler, tool_router,
    transport::stdio, ErrorData as McpError, ServerHandler, ServiceExt,
};

fn to_mcp_err(e: IrisError) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Substring matched against path and body. Omit (or empty) to list
    /// everything matching the other filters.
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub node_type: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PathParams {
    /// Path to the note, relative to the vault root.
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateNoteParams {
    pub path: String,
    /// note, task, project, area, resource, event, ... Unknown values
    /// become a `Custom` type per SCHEMA_SPEC's plugin-type fallback.
    #[serde(default = "default_note_type")]
    pub node_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub body: String,
}
fn default_note_type() -> String {
    "note".to_string()
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateNoteParams {
    pub path: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub clear_status: bool,
    /// urgent, high, normal, or low.
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub clear_priority: bool,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub clear_domain: bool,
    #[serde(default)]
    pub add_tags: Vec<String>,
    #[serde(default)]
    pub remove_tags: Vec<String>,
    /// Replace the body outright. Omit to leave it untouched.
    #[serde(default)]
    pub body: Option<String>,
}

// No stored `ToolRouter` field: `#[tool_handler]`'s default `router` param is
// `Self::tool_router()`, called fresh per request — nothing in this crate's
// pinned rmcp version reads a cached instance from `self`, confirmed by
// reading `rmcp-macros::tool_handler`'s actual expansion before adding one.
#[derive(Clone)]
pub struct IrisMcpServer {
    vault_root: PathBuf,
}

#[tool_router]
impl IrisMcpServer {
    pub fn new(vault_root: PathBuf) -> Self {
        Self { vault_root }
    }

    fn open(&self) -> Result<Engine, McpError> {
        Engine::open(&self.vault_root).map_err(to_mcp_err)
    }

    #[tool(description = "Search notes by text, optionally filtered by node type, domain, or tag.")]
    fn search_notes(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let engine = self.open()?;
        let filters = SearchFilters {
            node_type: p.node_type.as_deref(),
            domain: p.domain.as_deref(),
            tag: p.tag.as_deref(),
        };
        let results = search::search(engine.cache(), p.query.as_deref().unwrap_or(""), &filters)
            .map_err(to_mcp_err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string(&results).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Read a note's frontmatter and body by vault-relative path.")]
    fn get_note(&self, Parameters(p): Parameters<PathParams>) -> Result<CallToolResult, McpError> {
        let engine = self.open()?;
        let parsed = engine.read_node(&p.path).map_err(to_mcp_err)?;
        let out = serde_json::json!({"node": parsed.node, "body": parsed.body});
        Ok(CallToolResult::success(vec![ContentBlock::text(
            out.to_string(),
        )]))
    }

    #[tool(description = "Create a new note. Fails if the path already exists.")]
    fn create_note(
        &self,
        Parameters(p): Parameters<CreateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut engine = self.open()?;
        let node_type: NodeType = serde_yaml::from_str(&p.node_type)
            .map_err(|e| McpError::invalid_params(format!("invalid node type: {e}"), None))?;
        let mut node = Node::new(node_type);
        node.tags = p.tags;
        let body = format!("\n{}\n", p.body);
        engine
            .create_node(&p.path, &node, &body)
            .map_err(to_mcp_err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "Created {}",
            p.path
        ))]))
    }

    #[tool(description = "Update an existing note's status, priority, domain, tags, or body.")]
    fn update_note(
        &self,
        Parameters(p): Parameters<UpdateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut engine = self.open()?;
        let mut node = engine.read_node(&p.path).map_err(to_mcp_err)?.node;

        if p.clear_status {
            node.status = None;
        } else if let Some(s) = p.status {
            node.status = Some(s);
        }

        if p.clear_priority {
            node.priority = None;
        } else if let Some(pr) = p.priority {
            let priority: Priority = serde_yaml::from_str(&pr)
                .map_err(|e| McpError::invalid_params(format!("invalid priority: {e}"), None))?;
            node.priority = Some(priority);
        }

        if p.clear_domain {
            node.domain = None;
        } else if let Some(d) = p.domain {
            node.domain = Some(d);
        }

        node.tags.retain(|t| !p.remove_tags.contains(t));
        for tag in p.add_tags {
            if !node.tags.contains(&tag) {
                node.tags.push(tag);
            }
        }

        match p.body {
            Some(b) => engine
                .update_node_with_body(&p.path, &node, &format!("\n{b}\n"))
                .map_err(to_mcp_err)?,
            None => engine.update_node(&p.path, &node).map_err(to_mcp_err)?,
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "Updated {}",
            p.path
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for IrisMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "Tools over an Iris vault: search_notes, get_note, create_note, update_note. \
                 Paths are relative to the vault root this server was started against."
                    .to_string(),
            )
    }
}

/// Serve `vault_root` over stdio until the connected client disconnects.
pub async fn serve(vault_root: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let service = IrisMcpServer::new(vault_root).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
