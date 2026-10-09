# Iris

A local-first, git-backed ideation station — a single place to accumulate, structure, write, distill, search, and build out whatever's in your head, with an optional BYO-AI layer on top. Everything lives as plain markdown files with YAML frontmatter in a git repo you own; Iris is a set of clients over that vault, not a database that owns it.

**Status:** early, solo build. The Rust core and macOS app are the most complete pieces; Windows/Linux shells don't exist yet.

## What's in this repo

- **`iris-core`** — the Rust engine: typed nodes, PARA organization, task views, distillation queue, guided project activation, search, import/export, a WASM plugin runtime, and the UniFFI surface every native client binds to. No UI of any kind lives here.
- **`iris-cli`** — a scriptable command-line client (`iris` binary) over `iris-core`: `init`/`create`/`read`/`search`/`update`/`edit`/`done`/`rm`/`restore`. `update` can add/remove canonical relations by target ID or path. `iris mcp-server` exposes the vault to MCP-compatible AI agents (search/read/create/update/get_links/whoami tools) over stdio; `--read-only` refuses writes, note content is returned inside untrusted-content markers, and write tools take an `idempotency_key` (retries must use the same arguments).
- **`apps/macos`** — the native SwiftUI shell (no webview), consuming `iris-core` through the real UniFFI `FfiEngine` surface. See [`apps/macos/README.md`](apps/macos/README.md) for building/running it.

## Contributing (humans and AI agents)

Read `AGENTS.md` first (in the private planning repo; see `docs/WINDOWS_SETUP.md` for the three-repo layout). After cloning, run `bash scripts/install-hooks.sh` once to enable the pre-push check that requires docs to ship with code.

## Building the core + CLI

```bash
cargo build --workspace
./target/debug/iris -C /path/to/a/vault init
./target/debug/iris -C /path/to/a/vault create notes/idea.md --type note --body "hello"
./target/debug/iris -C /path/to/a/vault create notes/other.md --type note --body "related"
./target/debug/iris -C /path/to/a/vault search
./target/debug/iris -C /path/to/a/vault update notes/idea.md --add-relation references:notes/other.md
```

`cargo test -p iris-core` runs the engine's own test suite (round-trip parsing, cache rebuilds, node CRUD, plugin sandbox, etc.).

## License

GPL v3 — see [`LICENSE`](LICENSE).
