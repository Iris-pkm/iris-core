;; The reference example plugin for Iris's plugin runtime v1 (ADR-034).
;;
;; Hand-authored WebAssembly Text rather than compiled from a higher-level
;; language — this dev environment has no wasm32 Rust toolchain installed
;; (only aarch64-apple-darwin), and this plugin's whole job is to prove the
;; sandbox executes real WASM through the real host API, not to demonstrate
;; a particular guest language. `plugin.wasm` (the binary this compiles to)
;; is what actually gets installed; this file is the readable source.
;;
;; Declares `node_types: [note]` in its manifest.yaml. plugin_run:
;;   1. Logs a greeting via host_log (proves the sandbox runs at all).
;;   2. Creates one real note in the vault via host_create_node (proves the
;;      permission-checked host API actually reaches the real Engine).
(module
  (import "iris" "host_log" (func $host_log (param i32 i32)))
  (import "iris" "host_create_node" (func $host_create_node (param i32 i32) (result i32)))

  (memory (export "memory") 1)

  (data (i32.const 0) "Hello from the Iris plugin sandbox!")
  (data (i32.const 128) "rel_path: notes/hello-world-plugin.md\nnode_type: note\nbody: This note was created by the hello-world example plugin, running in a real wasmtime sandbox against the real permission-checked host API.\n")

  (func (export "plugin_run") (result i32)
    (call $host_log (i32.const 0) (i32.const 35))
    (call $host_create_node (i32.const 128) (i32.const 198))
    drop
    (i32.const 0))
)
