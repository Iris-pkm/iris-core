# Iris

A local-first, git-backed ideation station — a single place to accumulate, structure, write, distill, search, and build out whatever's in your head, with an optional BYO-AI layer on top. Everything lives as plain markdown files with YAML frontmatter in a git repo you own; Iris is a set of clients over that vault, not a database that owns it.

**Status:** early, solo build. The Rust core and macOS app are the most complete pieces; Windows/Linux shells don't exist yet.

## What's in this repo

- **`iris-core`** — the Rust engine: typed nodes, PARA organization, task views, distillation queue, guided project activation, search, import/export, a WASM plugin runtime, and the UniFFI surface every native client binds to. No UI of any kind lives here.
- **`iris-cli`** — a scriptable command-line client (`iris` binary) over `iris-core`: `init`/`create`/`read`/`search`/`update`/`edit`/`done`/`rm`/`restore`, plus `iris mcp-server`, which exposes the vault to any MCP-compatible AI agent (search/read/create/update tools) over stdio.
- **`apps/macos`** — the native SwiftUI shell (no webview), consuming `iris-core` through the real UniFFI `FfiEngine` surface. See [`apps/macos/README.md`](apps/macos/README.md) for building/running it.

## Building the core + CLI

```bash
cargo build --workspace
./target/debug/iris -C /path/to/a/vault init
./target/debug/iris -C /path/to/a/vault create notes/idea.md --type note --body "hello"
./target/debug/iris -C /path/to/a/vault search
```

`cargo test -p iris-core` runs the engine's own test suite (round-trip parsing, cache rebuilds, node CRUD, plugin sandbox, etc.).

## License

GPL v3 — see [`LICENSE`](LICENSE).
