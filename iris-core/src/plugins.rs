//! Plugin runtime v1 (ADR-034, ARCHITECTURE.md §10).
//!
//! A plugin is a directory — `manifest.yaml` (author-provided, declares the
//! plugin's identity and the `NodeType`s it may read/create) plus
//! `plugin.wasm` (or hand-authored WebAssembly Text, which `wasmtime`
//! accepts directly) — installed into `.iris/plugins/<id>/` inside the
//! vault. `.iris/` is git-tracked by default except `cache.sqlite`
//! (`Engine::init`'s own `.gitignore`), so installed plugins travel with
//! the vault like any other committed file.
//!
//! Install-state (`enabled`) lives in a separate `state.yaml` next to the
//! manifest — the author's shipped `manifest.yaml` is never mutated by
//! Iris; `state.yaml` is Iris's own file.
//!
//! **Permission model v1:** a manifest declares `permissions.node_types`
//! (enforced — every host function that touches the vault checks the
//! target node's type against this list before doing anything) and
//! `permissions.network_hosts` (parsed and stored for the UI's
//! permission-disclosure line, but **not enforced or usable** — no host
//! function exists to make a network call yet; real sandboxed network
//! access is a separate subsystem, deliberately out of v1's scope).
//!
//! **Invocation model v1:** manual only. `run_plugin` executes a plugin's
//! exported `plugin_run` once in a fresh `wasmtime::Store`. No automatic
//! lifecycle hooks (create/update/delete triggers) — that's real,
//! separate future work, not faked here.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wasmtime::{Caller, Engine as WasmEngine, Instance, Linker, Memory, Module, Store};

use crate::engine::Engine;
use crate::error::{IrisError, IrisResult};
use crate::types::{Node, NodeType};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub node_types: Vec<String>,
    #[serde(default)]
    pub network_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub enabled: bool,
}

impl Default for PluginState {
    fn default() -> Self {
        PluginState { enabled: true }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub dir: PathBuf,
}

fn plugins_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(".iris").join("plugins")
}

fn read_manifest(dir: &Path) -> IrisResult<PluginManifest> {
    let text = fs::read_to_string(dir.join("manifest.yaml"))?;
    serde_yaml::from_str(&text)
        .map_err(|e| IrisError::Validation(format!("invalid plugin manifest: {e}")))
}

fn read_state(dir: &Path) -> PluginState {
    // Missing state.yaml (e.g. a hand-placed bundle, not installed through
    // `install_plugin`) defaults to enabled — matches install's own default,
    // rather than treating an absent file as a validation error.
    fs::read_to_string(dir.join("state.yaml"))
        .ok()
        .and_then(|text| serde_yaml::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_state(dir: &Path, state: &PluginState) -> IrisResult<()> {
    let text = serde_yaml::to_string(state)
        .map_err(|e| IrisError::Validation(format!("failed to serialize plugin state: {e}")))?;
    fs::write(dir.join("state.yaml"), text)?;
    Ok(())
}

/// Every plugin installed in this vault, `.iris/plugins/*/`. An absent
/// `plugins/` directory (no plugin ever installed) is not an error — an
/// empty list.
pub fn list_plugins(vault_root: &Path) -> IrisResult<Vec<InstalledPlugin>> {
    let dir = plugins_dir(vault_root);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let plugin_dir = entry.path();
        // A malformed plugin directory (bad/missing manifest) is skipped,
        // not fatal to listing every other installed plugin — same
        // quarantine-one-broken-file philosophy `cache.rs::rebuild` already
        // applies to malformed vault nodes.
        let Ok(manifest) = read_manifest(&plugin_dir) else {
            continue;
        };
        out.push(InstalledPlugin {
            state: read_state(&plugin_dir),
            manifest,
            dir: plugin_dir,
        });
    }
    out.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(out)
}

/// Install a plugin bundle (a directory containing `manifest.yaml` +
/// `plugin.wasm`) from `bundle_dir` into this vault's `.iris/plugins/<id>/`.
/// The source bundle is left untouched — this copies, not moves, matching
/// `import.rs`'s own read-only-source convention.
pub fn install_plugin(vault_root: &Path, bundle_dir: &Path) -> IrisResult<InstalledPlugin> {
    let manifest = read_manifest(bundle_dir)?;
    let wasm_src = bundle_dir.join("plugin.wasm");
    if !wasm_src.exists() {
        return Err(IrisError::Validation(
            "plugin bundle is missing plugin.wasm".to_string(),
        ));
    }

    let dest = plugins_dir(vault_root).join(&manifest.id);
    fs::create_dir_all(&dest)?;
    fs::copy(bundle_dir.join("manifest.yaml"), dest.join("manifest.yaml"))?;
    fs::copy(&wasm_src, dest.join("plugin.wasm"))?;
    let state = PluginState::default();
    write_state(&dest, &state)?;

    Ok(InstalledPlugin {
        manifest,
        state,
        dir: dest,
    })
}

pub fn set_plugin_enabled(vault_root: &Path, id: &str, enabled: bool) -> IrisResult<()> {
    let dir = plugins_dir(vault_root).join(id);
    if !dir.exists() {
        return Err(IrisError::Validation(format!(
            "plugin {id} is not installed"
        )));
    }
    write_state(&dir, &PluginState { enabled })
}

/// Everything a running plugin's host functions need — collected here so
/// the `Linker::func_wrap` closures only capture one value each, not a
/// scattered handful. `RefCell` because `wasmtime` hands out `&Caller`
/// (shared) to host functions even though they need to mutate this state
/// (append log lines, write nodes through `engine`); nothing here is ever
/// accessed concurrently — one `Store`, one thread, run to completion.
// `wasmtime::Store<T>` requires `T: 'static`, which a `&'a mut Engine`
// field can't satisfy. `run_plugin` is fully synchronous and single-
// threaded, and the `Store`/`Instance`/every closure that touches
// `engine_ptr` are created and dropped entirely within `run_plugin`'s own
// stack frame — `engine_ptr` never outlives the `&mut Engine` it's derived
// from, and nothing else can touch `engine` while a plugin is running (no
// reentrancy, no other call in flight). A raw pointer sidesteps the
// lifetime `Store` can't express here without weakening any real
// guarantee.
struct PluginHost {
    engine_ptr: *mut Engine,
    plugin_id: String,
    permissions: PluginPermissions,
    logs: RefCell<Vec<String>>,
}

impl PluginHost {
    fn permitted(&self, node_type: &str) -> bool {
        self.permissions.node_types.iter().any(|t| t == node_type)
    }
}

fn guest_memory(caller: &mut Caller<'_, PluginHost>) -> Option<Memory> {
    caller.get_export("memory")?.into_memory()
}

fn read_guest_string(caller: &mut Caller<'_, PluginHost>, ptr: i32, len: i32) -> Option<String> {
    let memory = guest_memory(caller)?;
    let mut buf = vec![0u8; len as usize];
    memory.read(&caller, ptr as usize, &mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// A plugin's `host_create_node` wire format — deliberately minimal (three
/// fields the guest controls). `id`/`created`/`modified` are always set by
/// the host, never the guest, so a plugin can't forge a node's identity or
/// timestamps; the host also always tags the node `plugin:<plugin id>` for
/// provenance, which the guest has no way to omit or fake either.
#[derive(Debug, Deserialize)]
struct CreateNodeRequest {
    rel_path: String,
    node_type: String,
    #[serde(default)]
    body: String,
}

/// Load and run `plugin_id`'s `plugin_run` once, returning every line it
/// logged via `host_log`. A disabled plugin refuses to run rather than
/// silently no-op-ing.
pub fn run_plugin(engine: &mut Engine, plugin_id: &str) -> IrisResult<Vec<String>> {
    let vault_root = engine.vault_root().to_path_buf();
    let dir = plugins_dir(&vault_root).join(plugin_id);
    let manifest = read_manifest(&dir)?;
    let state = read_state(&dir);
    if !state.enabled {
        return Err(IrisError::Validation(format!(
            "plugin {plugin_id} is disabled"
        )));
    }
    let wasm_bytes = fs::read(dir.join("plugin.wasm"))?;

    let wasm_engine = WasmEngine::default();
    // `Module::new` auto-detects WebAssembly Text vs. the binary format —
    // the shipped example plugin is hand-authored `.wat`, since this
    // environment has no wasm32 Rust toolchain to compile a "real"
    // language down to WASM; real .wasm binaries from any source work
    // identically here.
    let module = Module::new(&wasm_engine, &wasm_bytes)
        .map_err(|e| IrisError::Validation(format!("failed to load plugin module: {e}")))?;

    let host = PluginHost {
        engine_ptr: engine as *mut Engine,
        plugin_id: manifest.id.clone(),
        permissions: manifest.permissions.clone(),
        logs: RefCell::new(Vec::new()),
    };
    let mut store = Store::new(&wasm_engine, host);
    let mut linker: Linker<PluginHost> = Linker::new(&wasm_engine);

    linker
        .func_wrap(
            "iris",
            "host_log",
            |mut caller: Caller<'_, PluginHost>, ptr: i32, len: i32| {
                if let Some(text) = read_guest_string(&mut caller, ptr, len) {
                    caller.data().logs.borrow_mut().push(text);
                }
            },
        )
        .map_err(|e| IrisError::Validation(format!("failed to register host_log: {e}")))?;

    linker
        .func_wrap(
            "iris",
            "host_create_node",
            |mut caller: Caller<'_, PluginHost>, ptr: i32, len: i32| -> i32 {
                let Some(raw) = read_guest_string(&mut caller, ptr, len) else {
                    return -1; // couldn't read the guest's request at all
                };
                let Ok(req) = serde_yaml::from_str::<CreateNodeRequest>(&raw) else {
                    return -2; // malformed request
                };
                if !caller.data().permitted(&req.node_type) {
                    return -3; // node type not in this plugin's declared permissions
                }

                let node_type = if let Some(builtin) = builtin_node_type(&req.node_type) {
                    builtin
                } else {
                    NodeType::Custom(req.node_type.clone())
                };
                let now = chrono::Utc::now();
                let node = Node {
                    id: crate::types::new_node_id(),
                    node_type,
                    created: now,
                    modified: now,
                    schema_version: crate::types::CURRENT_SCHEMA_VERSION,
                    lifecycle: None,
                    archived_at: None,
                    domain: None,
                    tags: vec![format!("plugin:{}", caller.data().plugin_id)],
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

                // SAFETY: see the doc comment on `PluginHost` — this
                // pointer is derived from `run_plugin`'s own `&mut Engine`
                // parameter and never outlives it.
                let engine_ref: &mut Engine = unsafe { &mut *caller.data().engine_ptr };
                // `Engine::create_node`'s `body` must start with `\n` — it's
                // appended directly after the closing `---` with no
                // newline inserted for you (`engine.rs::render`). A
                // plugin's guest-supplied body is plain text with no such
                // convention, so the host adds it here rather than making
                // every plugin author know this parser detail.
                let body = format!("\n{}\n", req.body);
                match engine_ref.create_node(&req.rel_path, &node, &body) {
                    Ok(()) => 0,
                    Err(_) => -4,
                }
            },
        )
        .map_err(|e| IrisError::Validation(format!("failed to register host_create_node: {e}")))?;

    let instance: Instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| IrisError::Validation(format!("failed to instantiate plugin: {e}")))?;
    let run = instance
        .get_typed_func::<(), i32>(&mut store, "plugin_run")
        .map_err(|e| IrisError::Validation(format!("plugin does not export plugin_run: {e}")))?;
    run.call(&mut store, ())
        .map_err(|e| IrisError::Validation(format!("plugin execution failed: {e}")))?;

    let logs = store.data().logs.borrow().clone();
    Ok(logs)
}

/// The plugin wire format spells node types the same way the schema's own
/// YAML frontmatter does (kebab-case, per `NodeType`'s `Serialize`) — this
/// just maps the handful of built-ins back to the typed enum so a plugin
/// creating an ordinary `note`/`task`/etc. gets the real variant, not
/// `Custom("note")`. Anything else falls through to `Custom`, exactly like
/// a hand-written vault file with an unrecognized `type:` would.
fn builtin_node_type(s: &str) -> Option<NodeType> {
    match s {
        "note" => Some(NodeType::Note),
        "task" => Some(NodeType::Task),
        "event" => Some(NodeType::Event),
        "project" => Some(NodeType::Project),
        "area" => Some(NodeType::Area),
        "resource" => Some(NodeType::Resource),
        "space" => Some(NodeType::Space),
        "annotation" => Some(NodeType::Annotation),
        "ink-note" => Some(NodeType::InkNote),
        "reminder" => Some(NodeType::Reminder),
        "daily-note" => Some(NodeType::DailyNote),
        "trading-journal-entry" => Some(NodeType::TradingJournalEntry),
        "music-idea" => Some(NodeType::MusicIdea),
        "reading-item" => Some(NodeType::ReadingItem),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-plugins-test-{label}-{nanos}"));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_bundle(dir: &Path, manifest_yaml: &str, wat: &str) {
        fs::write(dir.join("manifest.yaml"), manifest_yaml).unwrap();
        // A real .wasm binary is just what `wat::parse_str` produces — the
        // bundle format doesn't care whether the *source* was text or
        // binary, only that `plugin.wasm` decodes as one or the other.
        let wasm = wat::parse_str(wat).unwrap();
        fs::write(dir.join("plugin.wasm"), wasm).unwrap();
    }

    const HELLO_MANIFEST: &str = "\
id: hello-world
name: Hello World
version: 0.1.0
author: Iris
description: Proves the sandbox actually executes.
permissions:
  node_types: [note]
";

    const HELLO_WAT: &str = r#"
        (module
          (import "iris" "host_log" (func $log (param i32 i32)))
          (import "iris" "host_create_node" (func $create (param i32 i32) (result i32)))
          (memory (export "memory") 1)
          (data (i32.const 0) "hello from the sandbox")
          (data (i32.const 100) "rel_path: notes/plugin-demo.md\nnode_type: note\nbody: made by a real plugin\n")
          (func (export "plugin_run") (result i32)
            (call $log (i32.const 0) (i32.const 22))
            (call $create (i32.const 100) (i32.const 75))
            drop
            (i32.const 0))
        )
    "#;

    #[test]
    fn install_list_and_toggle_round_trip() {
        let vault_dir = TempDir::new("install");
        let mut engine = Engine::init(vault_dir.path()).unwrap();
        let bundle_dir = TempDir::new("bundle");
        write_bundle(bundle_dir.path(), HELLO_MANIFEST, HELLO_WAT);

        let installed = install_plugin(engine.vault_root(), bundle_dir.path()).unwrap();
        assert_eq!(installed.manifest.id, "hello-world");
        assert!(installed.state.enabled);

        let listed = list_plugins(engine.vault_root()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].manifest.name, "Hello World");

        set_plugin_enabled(engine.vault_root(), "hello-world", false).unwrap();
        let listed = list_plugins(engine.vault_root()).unwrap();
        assert!(!listed[0].state.enabled);

        // Disabled plugins refuse to run rather than silently no-op-ing.
        assert!(run_plugin(&mut engine, "hello-world").is_err());
    }

    #[test]
    fn hello_world_plugin_actually_creates_its_declared_node() {
        let vault_dir = TempDir::new("run");
        let mut engine = Engine::init(vault_dir.path()).unwrap();
        let bundle_dir = TempDir::new("bundle");
        write_bundle(bundle_dir.path(), HELLO_MANIFEST, HELLO_WAT);
        install_plugin(engine.vault_root(), bundle_dir.path()).unwrap();

        let logs = run_plugin(&mut engine, "hello-world").unwrap();
        assert_eq!(logs, vec!["hello from the sandbox".to_string()]);

        // Real proof: the node the plugin asked for actually exists in the
        // real vault, through the real Engine::create_node write path.
        let created = engine.read_node("notes/plugin-demo.md").unwrap();
        assert_eq!(created.node.node_type, NodeType::Note);
        assert!(created.body.contains("made by a real plugin"));
        assert!(created.node.tags.iter().any(|t| t.starts_with("plugin:")));
    }

    #[test]
    fn undeclared_node_type_is_rejected_not_created() {
        let vault_dir = TempDir::new("permission");
        let mut engine = Engine::init(vault_dir.path()).unwrap();
        let bundle_dir = TempDir::new("bundle");
        // Same plugin, but its manifest never declares `task` — the .wat
        // below tries to create one anyway.
        let manifest = "\
id: overreaching
name: Overreaching Plugin
version: 0.1.0
author: Iris
description: Declares note, tries to create task.
permissions:
  node_types: [note]
";
        let wat = r#"
            (module
              (import "iris" "host_log" (func $log (param i32 i32)))
              (import "iris" "host_create_node" (func $create (param i32 i32) (result i32)))
              (memory (export "memory") 1)
              (data (i32.const 0) "rel_path: tasks/sneaky.md\nnode_type: task\nbody: should not exist\n")
              (func (export "plugin_run") (result i32)
                (call $create (i32.const 0) (i32.const 65))
                drop
                (i32.const 0))
            )
        "#;
        write_bundle(bundle_dir.path(), manifest, wat);
        install_plugin(engine.vault_root(), bundle_dir.path()).unwrap();

        run_plugin(&mut engine, "overreaching").unwrap();

        // The sandbox rejected the call before it ever touched the vault —
        // proves enforcement, not just a manifest declaration nobody checks.
        assert!(engine.read_node("tasks/sneaky.md").is_err());
    }

    /// Not a test of `plugins.rs`'s own logic (that's every test above) —
    /// this proves the actual shipped `examples/plugins/hello-world/`
    /// bundle (the one a real user installs via the real "Install…" flow)
    /// still works, compiled fresh from its checked-in `.wat` source.
    /// Also regenerates the committed `plugin.wasm` binary from that
    /// source when run with `--ignored` — `.wat` is what's hand-edited;
    /// `.wasm` is the derived install artifact, kept in sync deliberately
    /// rather than trusted to stay in sync by hand.
    #[test]
    fn shipped_hello_world_example_installs_and_runs() {
        let example_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/plugins/hello-world");
        let wat_source = fs::read_to_string(example_dir.join("plugin.wat")).unwrap();
        let wasm_bytes = wat::parse_str(&wat_source).unwrap();

        let vault_dir = TempDir::new("shipped-example");
        let mut engine = Engine::init(vault_dir.path()).unwrap();
        let bundle_dir = TempDir::new("shipped-example-bundle");
        fs::copy(
            example_dir.join("manifest.yaml"),
            bundle_dir.path().join("manifest.yaml"),
        )
        .unwrap();
        fs::write(bundle_dir.path().join("plugin.wasm"), &wasm_bytes).unwrap();

        install_plugin(engine.vault_root(), bundle_dir.path()).unwrap();
        let logs = run_plugin(&mut engine, "hello-world").unwrap();
        assert_eq!(
            logs,
            vec!["Hello from the Iris plugin sandbox!".to_string()]
        );

        let created = engine.read_node("notes/hello-world-plugin.md").unwrap();
        assert_eq!(created.node.node_type, NodeType::Note);
        assert!(created
            .node
            .tags
            .contains(&"plugin:hello-world".to_string()));

        // Keep the committed binary artifact byte-for-byte in sync with the
        // hand-edited `.wat` source, rather than trusting a human to
        // remember to regenerate it after every edit.
        fs::write(example_dir.join("plugin.wasm"), &wasm_bytes).unwrap();
    }
}
