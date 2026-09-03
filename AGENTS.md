# AGENTS.md

Instructions for any AI coding agent working in this repo (Claude Code, opencode, Codex, etc). This is the shared source — tool-specific files should import or defer to this one rather than duplicating it.

## What this repo is

Iris: local-first, git-backed PKM. Rust core (`iris-core`) — typed nodes, PARA org, optional AI distillation layer. See `docs/files/OVERVIEW.md` and `ARCHITECTURE.md` for the full design.

## Repo split — read before touching git

Two separate git histories share this working tree:

- **`.git`** — public repo (`iris-core`, the Rust crate + CI config). This is what `git` commands hit by default.
- **`.git-private`** — private planning repo. Covers `docs/` (ARCHITECTURE, ROADMAP, DECISION_LOG, PROGRESS, AI_CONTEXT, etc). `docs/` is gitignored in `.git`, so it never leaks into the public repo.

Commands: `git --git-dir=.git-private --work-tree=. <cmd>` for the private side. When a change touches both code and docs, that's normally **two commits**, one per repo. Never `git add -f` docs into `.git` (the ignore is deliberate).

## Source of truth for build order

`docs/files/ROADMAP.md` governs *what gets built when* — phase-based, not date-based, each phase has an exit criterion. `IRIS_PHASED_PRODUCT_PLAN.md` is a thematic map with different phase numbers; if they disagree, ROADMAP.md wins (see its own note on this).

`docs/files/DECISION_LOG.md` holds numbered ADRs. Before revisiting a settled decision, check whether it's already an ADR — reopen only with a genuinely new argument, and note the reconsideration in `PROGRESS.md` either way (see e.g. ADR-031's reaffirmation entry).

## Working conventions

- **Before committing Rust changes:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p iris-core` must all be clean. CI (`.github/workflows/ci.yml`) enforces the same on push/PR.
- **Non-trivial logic gets a test** — a branch, a query, a parser, anything touching the write path. Trivial one-liners don't need one.
- **Cache is derived, never source of truth** (ADR-002) — the only way rows get into `.iris/cache.sqlite` is `Cache::rebuild()` re-scanning the vault. Never write to it directly.
- **Node mutation write order** (ADR-021, see `engine.rs` module docs): validate → write canonical file → rebuild cache → git commit. A canonical file write is never rolled back because a later step failed.
- **After a real chunk of work,** append an entry to `docs/PROGRESS.md` (append-only, headed `## YYYY-MM-DD HH:MM <tz>`) — what changed and why, not just what. This is the project history; don't skip it because the code diff already explains "what".
- Use `.codegraph/` (via the `codegraph` CLI or MCP tool, if the harness has it) instead of grep/find for locating or understanding code, when the index exists.

## Docs you probably need before a nontrivial change

- `docs/files/ARCHITECTURE.md` — system design, numbered sections and guardrails (e.g. "attach to node/relation substrate, don't introduce a parallel system").
- `docs/files/SCHEMA_SPEC.md` — node/frontmatter schema (currently v1, unfrozen but safe to build against).
- `docs/AI_CONTEXT.md` — current-state snapshot (what's done, what's blocked, what's next). Keep it in sync when you finish a chunk of Phase work — it's the fast-orientation doc for the next agent/session.
