# Iris — Architecture Document

**Status:** Draft v1 — reflects decisions made through prior design sessions; unresolved items are flagged, not guessed at.

---

## 1. System Overview

```
                     ┌─────────────────────────────────────┐
                     │           UI Shells (thin)           │
                     │  Tauri (desktop) · SwiftUI (iOS/iPad) │
                     │      Kotlin Compose (Android)         │
                     └───────────────┬───────────────────────┘
                                     │  UniFFI / cxx bindings
                     ┌───────────────▼───────────────────────┐
                     │              Rust Core                │
                     │  vault engine · sync/CRDT · search     │
                     │  plugin runtime · distillation logic   │
                     └───┬─────────┬─────────┬────────────┬──┘
                         │         │         │            │
                 ┌───────▼──┐ ┌────▼───┐ ┌───▼──────┐ ┌───▼────┐
                 │ Git vault│ │ SQLite │ │  Search  │ │   AI   │
                 │ (source  │ │ (cache,│ │  index   │ │abstrac-│
                 │ of truth)│ │rebuild-│ │(lexical/ │ │  tion  │
                 │          │ │ able)  │ │ vector/  │ │ layer  │
                 │          │ │        │ │ graph)   │ │(BYO)   │
                 └──────────┘ └────────┘ └──────────┘ └────────┘
```

## 2. Layered Architecture

- **Rust core** — the single correctness-critical library: vault read/write, CRDT merge logic, search indexing, plugin sandboxing, distillation-queue logic. Written once, consumed everywhere via bindings.
- **Storage layer** — git-backed markdown vault (canonical source of truth) + SQLite (derived, fully rebuildable cache — never a source of truth).
- **Sync layer** — two-tier, phased (see §6).
- **Search layer** — four-layer target: lexical, semantic/vector, structural/graph, temporal. Lexical ships first; the rest are later-phase.
- **AI abstraction layer** — provider-agnostic; BYO API key (Anthropic, Google, OpenAI, Deepseek, Qwen, etc.) or MCP server connection for local models. Entirely optional at runtime.
- **Plugin runtime** — WASM-sandboxed, because plugins touch a personal, often-sensitive vault.
- **UI shells** — thin, native-feeling per platform, all consuming the same Rust core.

## 3. Object Hierarchy

This section makes explicit a structure that was previously only implicit across the Data Model and PARA discussions. It answers a deceptively simple question — "what contains what in Iris?" — whose answer is important precisely because Iris's answer differs from the folder-based mental model most note apps train users into.

A natural first guess at the hierarchy is something like `Project → Vault → Node`, but that has the direction inverted (a Project can't contain the Vault — the Vault is the whole system) and, more importantly, it assumes *containment* is the organizing principle when in Iris the organizing principle is *typed relations in a graph*. The corrected structure:

```
Vault  (exactly one active at a time; not a node — it is the boundary of the whole system)
  └── Node  (the universal primitive; almost everything is one)
        ├── Project / Area / Resource            (the P/A/R of PARA — node *types*, related by edges, not containers)
        │     └── Epic → Story → Subtask          (nodes, connected by typed parent/child relations)
        │           └── Checklist item             (NON-NODE exception #1 — lightweight sub-content of a Task)
        ├── Canvas  (a node type with its own identity; optionally relates to a Project)
        │     └── Ungraduated scratch content       (NON-NODE exception #2 — freeform, lives inside the canvas node)
        ├── Space   (a saved lens/context — NOT a container; sits beside this hierarchy, not inside it)
        └── Note, Task, Event, Reminder, Annotation, Ink-note, Daily Note,
            Trading Journal Entry, Music Idea, Reading Item, user-defined custom types …
            (all nodes; all freely relatable to any Project / Area / Resource simultaneously)

  Note: the "A" of PARA — Archive — is NOT a node type. Archival is a lifecycle STATE
  (`lifecycle: archived`) that any node can carry without changing its type (ADR-016);
  "Archive" is a system view filtering for that state, not a container.
```

**The three things worth internalizing about this structure:**

1. **The Vault is the top, and there is exactly one active at a time.** It's the git repository — the entire second brain and the boundary around it. It is deliberately *not* itself a node; it's the container/namespace within which all nodes exist. (Multi-vault behavior — how more than one vault folder can exist on disk while only one is ever active — is covered in §6.)

2. **PARA containers are node types, not a structural containment layer.** This is the single biggest departure from a filesystem mental model. A Project does not "contain" its Tasks the way a directory contains files. A Task node relates to its Project node through a typed relation (`parent_project`), which is the *same mechanism* that links any two nodes for any reason. The practical payoff is significant: because membership is a relation rather than physical containment, a single Task can belong to a Project *and* simultaneously relate to a Resource that informed it, a Reading note it came from, and a Trading Journal Entry it affects — none of which a strict one-parent folder tree can represent. The Epic → Story → Subtask nesting inside a Project works identically: it's nodes joined by typed parent/child relations, not physical nesting.

3. **The tree is a view; the graph is the reality.** PARA and the Epic/Story/Subtask breakdown both *look* like trees because a tree is the most legible default lens for them, and Iris presents them that way. But nothing is stored as a tree — the underlying structure is always a graph of typed relations, and the same node can occupy a "position" in several trees at once depending on which lens you're viewing it through. The tree is a projection chosen at render time; the graph is what's on disk.

**The two deliberate non-node exceptions**, both of which exist for the same reason — some content is genuinely sub-content of one parent and should not have independent existence in the graph:

- **Checklist items** (inside a Task) — a checklist item has no frontmatter and no independent relations; it's lightweight sub-content of its Task. It gets promoted to a full Subtask node *only* when the user explicitly asks, at which point it becomes a first-class graph citizen like anything else.
- **Ungraduated scratch content** (inside a Canvas) — sticky notes, loose images, and dragged-in references dropped onto a canvas live as freeform, unstructured data inside that Canvas node's own body, not as independent nodes. This is exactly what keeps pre-structural canvas thinking out of search, the distillation queue, and analytics until it's ready. **Graduating** a scratch item promotes it into a real, independent node (with its own frontmatter and relations) and leaves behind a `graduated-from` relation pointing back to the canvas it originated on — the same relation mechanism used everywhere else.

**On Canvas specifically:** Canvas is the one place the otherwise-clean "every visual surface is just a stateless lens over selected nodes" rule genuinely breaks, so it's worth stating the resolution precisely. Table, Graph, Board, Calendar, and Gallery are *rendering modes* — you don't create "a Table," you apply the table lens to whatever's selected. A Canvas is different: you create a *specific, named, persistent* one ("Iris Q3 brainstorm"), it has its own identity and lifespan, and it can relate to a Project like any other node. Therefore Canvas is both a **node type** (the instantiated object, sitting at the same level as Project or Space) *and*, when you're looking at one, a **view** (the freeform spatial rendering of that node's contents). Both facts are true simultaneously; earlier drafts listed only the "view" half, which undersold it.

**On Space specifically:** a Space is easy to mis-slot into this hierarchy as a container tier, so to be explicit — it isn't one. A Space is a saved lens (pinned nodes, an active filter, a default view, a theme) that can cut across multiple Projects and Domains at once. It sits *beside* the hierarchy, not at any level within it. Switching Spaces is an instant, lightweight context change within the one open Vault; it moves your point of view, it does not move or contain any data. (Contrast with switching Vaults, §6, which is heavyweight and closes one graph to open another.)

## 4. Data Model

**Node** — the universal primitive.

| Field | Description |
|---|---|
| `id` | Stable unique identifier |
| `type` | note / task / event / project / area / resource / space / annotation / reminder / ink-note / daily-note / domain-specific (trading journal entry, music idea, reading item) / user-defined custom types. **`archive` is not a type** — archival is lifecycle state (ADR-016). |
| `frontmatter` | Typed YAML metadata, schema varies by node type |
| `body` | Markdown content — may embed images, audio players, PDFs, code blocks, LaTeX, and video inline via standard markdown/embed syntax, so rich content stays portable rather than proprietary |
| `relations` | Typed edges to other nodes (references, blocks, part-of, related-to, annotates, instance-of, flow-next, etc.) |
| `distillation_level` | `raw \| bolded \| highlighted \| summarized` — only meaningful for note-like content |
| `timestamps` | created, modified |
| `deleted_at` | soft-delete marker; if set, the node is in Trash (excluded from normal views, still queryable in Trash and always recoverable from git) — see Data Integrity & Recovery below |
| `lifecycle` | `active` (default) or `archived` — archival state, orthogonal to `type` (ADR-016); an archived node keeps its original type. Paired with `archived_at`. Distinct from `deleted_at` (Trash): archived = preserved-but-inactive, trashed = recoverable-pending-removal |

Relations are typed and directional where relevant, forming the graph that powers graph/constellation views and structural search.

**New lightweight node types (beyond the original set):**
- **`space`** — a saved UI/context configuration (pinned node IDs, active domain/tag filter, default view + its state, visible panels, theme accent). Explicitly *not* a PARA container — it can span multiple projects or cut across domains. Modeled as a node so it gets version history, sync, and export for free rather than needing a separate config system.
- **`annotation`** — an anchored comment: a note pinned to a specific range of text inside a target node's body, related via a typed `annotates` edge, with `resolved: bool` and optional threaded replies (each reply itself a lightweight `annotation` targeting the parent annotation). Carries an anchor with two resolution strategies: a CRDT position (stable under concurrent edits once the CRDT layer exists — see §6) and a text-fragment fallback (context-matching, for edits made outside Iris where the CRDT position can't apply). If neither resolves, the annotation detaches to an explicit "orphaned" state rather than silently vanishing or reattaching to the wrong text.
- **`ink-note`** — a handwritten node type for stylus input (iPad/Android tablet). Stores raw stroke data as the canonical content *and* an OCR'd text transcript alongside it — two representations of the same node, not a lossy one-way conversion, so it's both visually personal and fully searchable.
- **`reminder`** — a user-authored, time-based nudge. Deliberately **manual-only**: the `text` is always written by the user and never auto-generated from a task or event title, and a reminder never fires unless the user explicitly created it. Fields: `text` (user-authored), `fire_at` (a specific datetime, or a recurrence rule reusing the same fixed/flexible/RRULE model as recurring tasks — so "remind me every Monday at 9am" works identically), `status` (`pending | fired | dismissed | snoozed`), and an optional `target` typed relation to any node. A reminder can stand entirely alone ("call the broker at 9am" — no target) or point at a node ("remind me about *this* note at 3pm"). Rationale for manual-only: auto-firing a notification whenever a due date approaches is the same "the system decides when to interrupt you" pattern that manufactured-urgency gamification was rejected for (ADR-009). Task due dates, event start times, sprint-ending, and streak-at-risk therefore remain *in-app indicators only* (Today view, burndown, streak counter) and never push an OS notification on their own — if the user wants to be nudged about a due task, they create a reminder for it themselves (optionally one tap from the task, pre-filling the task's title as an editable starting point, but never automatically). See the Notifications entry in `OVERVIEW.md` and `PRD.md` for the full user-facing behavior.

**The template model — three distinct tiers (ADR-026):**
"Template" in Iris means one of three genuinely different things. They share a word but not their mechanics, and keeping them distinct is deliberate — conflating them causes the "why did editing this template change my old notes?" class of surprise.

- **Tier 1 — Node templates (copy-on-use).** A reusable starting point for a *single* node: a daily-note layout, a meeting-note skeleton, a book-review structure. A node flagged `is_template: true` in frontmatter (excluded from normal views/search) that is **copied** when used. The resulting node is fully independent — later edits to the template do *not* touch notes already created from it, and no `instance-of` link is made. This is the simplest tier and the earliest to build (Desktop phase). It's what most apps mean by "template."

- **Tier 2 — Components / Instances (live-linked).** A template node acting as a **Component**; every node created from it is an **Instance** carrying an `instance-of` relation back to the Component. The Instance inherits the Component's schema but can override individual fields locally. Editing the Component (e.g. adding a field) **propagates** to every Instance that hasn't overridden that specific field — the same live-linked relationship Figma has between components and instances. This is the actual mechanism behind schema evolution: it turns "I need to add a field to every trading journal entry" from a manual migration into a single template edit. The distinction from Tier 1 is exactly that the link *persists* and changes *propagate*. Delivered with custom node types in the Plugin System phase (Phase 11 thematic / Phase 7 build-order). Override semantics — how a per-instance override survives a later Component edit (last-write-wins per field, or an explicit per-field "locked" flag) — remain an open sub-question (Open Architectural Threads).

- **Tier 3 — Starter systems (configured workflows).** The adoption-critical sense (`IRIS_MISSING_PRODUCT_CAPABILITIES.md` §11): not a page but a *whole working setup* for a domain — software project, research paper, trading journal, reading pipeline, job search, fitness plan, and so on. A starter system may bundle node types, relations, views, statuses/workflow states, activation behavior, dashboards, default queries, capture destinations, and review workflows, installed as a unit so a non-builder adopts a sophisticated workflow without designing it from scratch. Tier 3 is built *on top of* Tiers 1–2 plus custom node types and views, which is why it lands latest (Plugin phase and beyond). Crucially, a starter system is defined in the same declarative form users and plugins can author — not privileged app internals — so first-party starter systems and community ones share one mechanism.

All three tiers attach to the node/relation substrate rather than introducing a parallel system (guardrail 6.10): a template *is* a node (Tiers 1–2), and a starter system is a bundle of nodes, relations, and view configs (Tier 3). None requires AI — the mechanics are copy, inherit-with-override, and bundle-install respectively; AI could later *suggest* a starter system, but no tier depends on it.

**Custom node types:** beyond the built-in types, the plugin API (the Plugin Runtime, §10 below; delivered in the Plugin System phase — Phase 11 in `IRIS_PHASED_PRODUCT_PLAN.md`'s thematic scheme, Phase 7 in `ROADMAP.md`'s build order; API surface not yet finalized — see Open Architectural Threads) lets a user define their own node type with its own frontmatter schema. Custom types are what Tier 2 Components parameterize and what Tier 3 starter systems bundle, which is why all three of these capabilities arrive together in the Plugin phase and why the Phase 0 schema is designed not to need reshaping to support them (ADR — see SCHEMA_SPEC type taxonomy).

**Controlled vocabularies:**
Domains, priority levels, and workflow states are defined once (as values in a small controlled-vocabulary table, not as free-typed strings scattered across frontmatter) and referenced by every node that uses them. Renaming a domain updates every node that references it in one operation, preventing the common failure mode where `trading`, `Trading`, and `#trading` silently fragment into three untethered tags over time. Free-form tags remain available alongside controlled vocabularies for genuinely open-ended labeling — the two aren't mutually exclusive.

**Domain vs. tags vs. Areas — three distinct concepts that are easy to conflate.** Because `domain` is load-bearing (it drives heatmap coloring, graph-cluster coloring, and the domain-balance analytic), it needs to be pinned down rather than left implicit:
- **`domain`** is a single controlled-vocabulary value per node (e.g. `trading`, `music`, `iris-dev`) — the node's *primary sphere of life*, chosen from a defined, renameable list. It is deliberately singular (one domain per node) precisely so color-coding is unambiguous: a node maps to exactly one color. This is the field the heatmap and graph clusters key off.
- **Tags** are zero-or-many free-form labels per node, for open-ended cross-cutting categorization (`#q3`, `#follow-up`, `#idea`). They are not controlled and not colored.
- **Areas** are PARA *node types* (ongoing responsibilities), i.e. actual nodes in the graph that other nodes relate to — not a property on a node. An Area and a domain may sound similar ("Music" the Area vs. `music` the domain) but they operate at different layers: the Area is a container node you can open, attach notes to, and see a dashboard for; the domain is a colored classification stamped on each node. A node can carry the `music` domain while relating to several different Area and Project nodes.
- **Open question (flagged, not yet closed):** whether `domain` should ever support multiple values per node for genuinely cross-domain nodes (a trading-psychology note that's equally `trading` and `psychology`). Current decision is singular-for-now to keep coloring unambiguous; if multi-domain proves necessary, the coloring rule needs a tie-breaker (primary domain, or a blended/striped cell). Tracked in `DECISION_LOG.md`.

## 5. Storage & File Format

- Each node is a markdown file with YAML frontmatter, stored in a git repository. This is the canonical, human-readable, portable source of truth.
- **The canonical vault is the markdown/git repository *plus* the content-addressed blob store, together (ADR-020)** — neither alone is complete. A plain `git clone` recovers text and metadata but not attachments; a complete backup/restore must include both.
- Attachments use content-addressed storage (hash-based filenames), avoiding duplication and enabling integrity checks.
- SQLite holds a derived, queryable cache of the vault state — indexes, relation tables, distillation status — and must be fully rebuildable from the vault alone at any time.
- Exports are described by a versioned JSON Schema manifest, so any export is self-describing and future-proof against schema drift.

**Lossless editing (ADR-019).** When Iris modifies a node it edits only the frontmatter fields that actually changed, leaving key order, comments, whitespace, unknown fields, and the entire markdown body byte-for-byte untouched — it does *not* re-serialize the whole file from an in-memory model. This preserves hand-edited formatting and comments, keeps git diffs minimal, and composes with the preserve-unknown-fields rule. Architecturally it implies a span-preserving / concrete-syntax-tree editor rather than a load-then-dump serializer — a parser-shaping decision, which is why it's fixed before implementation.

**Write-path transaction ordering (ADR-021).** A node edit touches three systems — the canonical file, the SQLite cache, and git history — and the canonical file always wins. Fixed order: validate → atomically write canonical file(s) → update/rebuild affected cache → git commit. If the cache update fails, mark the cache dirty and rebuild; if the git commit fails, keep the canonical data and surface an "unsaved history" warning. **A canonical file mutation is never rolled back because SQLite or git failed** — the cache is rebuildable and commits can be retried, but the user's content is never discarded for a secondary system's failure. Multi-file operations must not leave partial canonical mutations.

### Attachment model

Attachments (image, audio for music-idea capture, PDF, video) are core to several use cases, so the mechanism is specified here rather than left implicit. Canonicality is settled (ADR-020); the *transfer* mechanism is the remaining open detail.

- **Reference mechanism:** a node's markdown body references an attachment through standard markdown syntax (`![alt](path)` for images, a link for other types); on write, Iris resolves that reference to a **content hash** and records the mapping. The canonical form stored is a stable logical reference plus the hash — so renaming or moving things never breaks the link, consistent with why relations use ULIDs rather than filenames.
- **Physical location:** attachment blobs live in a dedicated content-addressed store *inside the vault directory* (e.g. an `attachments/` subtree keyed by hash), so a clone/backup of the vault carries its attachments with it and the vault stays self-contained. The vault manifest maps required blobs by hash.
- **Relationship to git:** per the storage-weight principle (ADR-010), large binary blobs are **not** committed into the main git history as normal versioned files (that would bloat history irreparably). The blob lives in the content-addressed store, tracked by the manifest. **Open (transfer mechanism only):** git-LFS vs. a custom content-addressed store synced out-of-band is a Phase 5 (sync) decision — but that's *how blobs move*, not *whether they're canonical*; canonicality is settled (ADR-020).
- **Sync:** because blobs are deliberately outside normal git history, they need a sync path of their own. This is called out explicitly because "the vault syncs" does **not** automatically mean "attachments sync" — the two are separate transfer problems, and attachment sync must be designed as its own adapter (much like external calendar sync is a separate adapter from vault sync).
- **Integrity:** because storage is content-addressed, verifying an attachment is just re-hashing the blob and comparing — one of the checks the vault integrity checker (§16) performs; a mismatch or *missing* blob (referenced by a node but absent from the store) surfaces in the "Needs Attention" list rather than failing silently.

**Named checkpoints and branching:** since the vault is already a real git repository, "save a named checkpoint here" and "try a different reorganization without committing to it" map directly onto git tags and git branches — no separate versioning system needed. The UI surfaces these in plain language (checkpoint name, branch name) rather than requiring the user to think in git directly, but the underlying primitive is real git the whole way down, so anything done through the UI remains inspectable/recoverable with any standard git tool if Iris itself is ever unavailable.

### Data Integrity & Recovery

Because files are canonical and hand-editable, and because losing or corrupting a note is treated as an unacceptable failure (not a minor bug), several protections are first-class rather than afterthoughts:

- **Malformed frontmatter → quarantine, never crash, never silently drop.** A file that fails to parse (most likely hand-broken YAML, since the vault is deliberately editable outside Iris) is *excluded* from the cache and graph but *not* discarded and *not* allowed to abort a whole-vault rebuild. It's listed in a "Needs Attention" view with the actual parse error shown, and Dev Mode exposes the raw file content so it can be fixed in-app or in an external editor. The guiding rule: one broken file can degrade exactly itself, never the rest of the vault.
- **Soft-delete / Trash.** Deleting a node sets its `deleted_at` marker rather than immediately erasing it. Deleted nodes drop out of all normal views but remain in a dedicated Trash view, queryable and one-click recoverable, for a retention window (default ~30 days). After the window they're removed from the working vault — but because every prior state was committed to git, they remain recoverable from history indefinitely; Trash is simply the *convenient* recovery surface for the common "oops, I deleted that" case, with git as the permanent backstop. This deliberately covers the most frequent data-loss panic (an accidental delete) far more directly than an undo stack alone could.
- **Undo/redo (two tiers, deliberately distinct).** An in-session undo stack (Cmd/Ctrl+Z, in-memory, cleared on restart) handles immediate "undo that last edit" mistakes with standard editor behavior. Git-history restore handles the durable, further-back, survives-a-restart case. These are two different mechanisms for two different needs, not one feature stretched across both — conflating them would make the common quick-undo case feel heavier than it should and the deep-restore case feel less reliable than it is.
- **Restore from backup (an explicit first-run path).** Alongside "create a new vault" and "open an existing vault," first-run offers "restore from backup": clone/pull from a git remote, rebuild the SQLite cache from the restored files, and run the vault integrity checker (see Testing & Correctness, §16) *before* declaring the restore complete. There is deliberately no periodic automatic backup-verification process — that would spend battery and complexity on a rare event; instead, verification happens at restore time, which is precisely when it actually matters.

**Open thread:** final frontmatter/schema spec per node type is not yet locked — this is Phase 0 work (see `ROADMAP.md`).

## 6. Sync Strategy (Phased)

- **Phase A (ships first):** naive WebSocket-based sync, plus periodic git commits for durable version history. Good enough for single-user, low-concurrency multi-device use.
- **Conflict resolution (Phase A):** when the same node is edited independently on two devices before syncing, the edit that syncs first becomes the canonical node — but the losing edit is never silently discarded. It's preserved as a separate node, tagged `conflicted` and linked to the original via a typed relation, surfaced via a small badge/list until manually reconciled (copy content across, discard, or merge). Same underlying problem as a GitHub merge conflict — two edits diverging from a common ancestor — resolved by keeping two clean versions side by side rather than splicing inline conflict markers into the file. See ADR-012 in `DECISION_LOG.md`.
- **Phase B (later):** CRDT-grade merge (Automerge-rs or equivalent) layered on top, enabling real-time, conflict-free concurrent editing across devices. Interacts with the git layer as a periodic snapshot/commit mechanism rather than replacing it. Reduces how often conflicts occur in the first place; the conflict-copy mechanism above remains the fallback for anything it can't auto-merge (structural/relation-level conflicts, not just text).
- **Dependency worth flagging explicitly:** the CRDT layer isn't only a sync mechanism — it's also the stable-position-tracking primitive that `annotation` nodes (§4) need to keep an anchor attached to the right text under concurrent edits. Anchored comments can ship earlier with the text-fragment-fallback strategy alone, but their robustness upgrades meaningfully once Phase B lands, rather than needing a second, separately-engineered position-tracking system.

### Multi-vault behavior — one active vault, not many simultaneous ones

Iris's entire premise is that everything connects: one graph, one search index, one AI context, cross-domain links surfacing non-obvious relationships. Splitting a user's knowledge across multiple simultaneously-active vaults would directly undermine that — you'd have two separate second brains that can't see or link to each other, which is the specific failure the whole design exists to avoid. So the rule is deliberate: **Iris operates on exactly one active vault at a time.**

That said, the app is not hardcoded to be aware of only a single vault folder that can ever exist on disk. Two distinct concepts are kept separate:

- **The active vault** — whatever is currently open. The graph, the search index (all four layers), the AI's context window, and the analytics dashboard all operate exclusively over this one vault. There is never any cross-vault traversal, cross-vault search, or cross-vault AI reasoning. While Iris is running, from the application's perspective, the active vault is effectively the only vault that exists.
- **Switching vaults** — a deliberate, heavyweight operation that fully closes the current vault and points Iris at a different folder: a different git repository, a different SQLite cache rebuilt from scratch, a different everything. This is explicitly *not* a lightweight context flip.

The contrast that matters most, because the two are easy to conflate: **a Space switch changes your point of view instantly within the one open vault; a vault switch closes one graph entirely and opens a different one.** Spaces are for "I'm now focused on trading" — cheap, frequent, no data reload, covered in the object hierarchy (§3). Vault switching is for "I'm now working inside a completely separate second brain" — rare, deliberate, and expensive (a full cache rebuild).

Legitimate reasons a second vault folder might exist at all, despite single-active-vault being the norm: a **sandbox** for testing a new plugin or a risky schema change without endangering the real vault, or a **parallel vault during a structural migration**. These are the exception, not the daily workflow.

This also disambiguates what "Vault picker" means wherever it appears in the roadmap docs: it is *not* "choose which of my many vaults to work in today." It means "point Iris at a vault folder" — used at first-run setup, when restoring from a backup (§7-adjacent, see restore/recovery in the roadmap), or for the rare deliberate switch to a sandbox/migration vault. A lightweight "recent vaults" list (the same pattern VS Code and Obsidian use for workspace folders) covers this without implying routine multi-vault use.

## 7. Security

- **No per-file encryption.** Considered (ADR-007) and superseded (ADR-011, see `DECISION_LOG.md`) — the functional cost (search exclusion, meaningless git diffs, AI needing per-action unlock, key-management friction across devices) outweighed the threat it addressed, given the realistic threat model is a lost/stolen device.
- **Baseline protection is OS-level full-disk encryption** (FileVault/BitLocker/LUKS) — outside Iris's own scope, same posture most local-first note tools take.
- Local-first by default: core functionality has no server dependency. Sync is additive, not required.

## 8. AI Layer

- Abstraction is provider-agnostic by construction: works via user-supplied API key or an MCP server connection (enabling local models).
- Every core workflow (capture, organize, PARA, distillation queue mechanics, task/sprint/calendar planning) must work with **zero AI configured**. AI is a multiplier layered on top, never load-bearing.
- Primary AI use cases: LLM-assisted first-pass bolding in the distillation queue; sprint/scheduling suggestions grounded in calendar capacity; longer-term, general agent-style actions over the vault.

**Two integration surfaces, not one:**
- **API key** — user supplies a key for a hosted provider (Anthropic, Google, OpenAI, Deepseek, Qwen, etc.); Iris calls that provider's API directly from the client. The key is stored locally only (OS keychain/credential store) — never on an Iris server, since there is no Iris server.
- **MCP server connection** — user points Iris at any MCP-compatible endpoint, including a fully local model (e.g. via Ollama). This is what makes a genuine air-gapped configuration possible: zero data leaves the machine if the user chooses this path.

**Two model roles, configured independently:**
- **Reasoning/completion model** — used for distillation drafting, the writing assistant, contradiction detection, digest generation, sprint/scheduling suggestions. Heavier, typically hosted.
- **Embedding model** — used only for semantic search and similarity/auto-linking. Cheaper, and realistic to run fully locally (e.g. a small on-device embedding model) even when the reasoning model is a hosted API. These are separate settings — a user may run embeddings locally while using a hosted reasoning model, or vice versa.
- Practical effect: three of the four search layers (lexical, structural, temporal — see PRD/OVERVIEW) require no model at all; only the semantic layer needs the embedding model specifically, not a full LLM.

**Provider abstraction shape:**
```
IrisAIProvider (interface)
    ├── AnthropicProvider / GoogleProvider / OpenAIProvider / ... (API key)
    ├── OllamaProvider / CustomMCPProvider (MCP, local or remote)
    └── NullProvider (no AI configured — every feature degrades gracefully)
```
Every AI-touching feature calls through `IrisAIProvider`, never a specific provider directly, so switching or removing a provider is a settings change, not a code change.

**Non-negotiable rule:** AI outputs must be reviewable before being written into the vault. No silent mutation — the user approves, edits, or rejects every AI-proposed change.

## 9. Platform Strategy

**Target platforms:** Desktop (Windows/macOS/Linux) · Mobile (iOS, Android) · Tablet (iPad, Android tablet) · Browser extension (capture surface, not a full client) · Widgets (desktop + mobile, read-mostly surfaces) · Watch (explicitly future, not in scope now).

**Decision:** shared Rust core + native/thin UI shells per platform (see ADR-004 in `DECISION_LOG.md`).

- Desktop: Tauri.
- iOS/iPadOS: SwiftUI, consuming the Rust core via UniFFI bindings.
- Android/Android tablet: Kotlin Compose, same binding approach.
- Browser extension: thin client, talks to the same sync API every other device talks to — not special-cased. Capture-only surface (clip, highlight, quick-capture popover, tab-to-node); it is not expected to render full views (graph/table/board).

This was chosen over (a) a single shared UI codebase (React Native/Flutter/KMP-Compose-Multiplatform) — rejected for feeling less native — and (b) fully independent native implementations including business logic per platform — rejected as unmaintainable for a solo long-haul build. The correctness-critical logic lives once, in Rust; UI is thin and idiomatic per platform.

### Widgets — a distinct, more constrained surface

Widgets are read-mostly and platform-sandboxed, which makes them structurally different from the full app shells above:

- **iOS (WidgetKit) / Android (Glance):** no arbitrary interactivity, no live network calls on render — the OS renders periodic static snapshots. Data must be pre-fetched and cached by the main app; the widget reads from that cache only. Interactive elements (a task checkbox, a quick-capture input) are supported only within each platform's specific interactive-widget APIs and are the exception, not the default.
- **Desktop widgets:** less constrained — no OS sandbox equivalent, can talk to the local Iris process directly, can refresh more frequently.
- **Why this is a validator, not just a feature:** a widget can only be fast and battery-safe if it reads from an already-warm local cache rather than triggering a fetch. If the local SQLite cache can answer "today's tasks" or "a recently active node" fast enough to power a widget, it's fast enough to power the rest of the app — see Engineering Principles §14 and ADR-010's memory/battery weight dimensions.

## 10. Plugin Runtime

WASM-based sandboxing (e.g. via `wasmtime`), given that third-party plugins will have access to a personal, potentially sensitive vault. Plugin API surface is not yet finalized — flagged as an open thread.

## 11. Distillation Queue Subsystem

This is the product's core differentiator and deserves architectural detail beyond the PRD:

- **Trigger:** a project transitioning *into* `active` status enqueues its associated raw notes for distillation. This is deliberately *not* time-based — relevance is tied to when the knowledge is actually needed, not an arbitrary schedule. Per ADR-018, the trigger semantics are precise: (1) activation fires on any transition into `active` from a non-active state; (2) it does *not* re-fire on merely opening/viewing an already-active project; (3) while the project stays `active`, newly-linked raw notes are added to the queue incrementally — so queue membership is continuously maintained, not a one-time snapshot at activation. The full project state machine (`someday → planned → active → paused → active`, `active → completed`, etc.) lives in ADR-018 and `SCHEMA_SPEC.md`.
- **State model:** each note carries a `distillation_level` (`raw → bolded → highlighted → summarized`), enabling progressive, multi-visit processing rather than one-shot summarization — directly modeling Forte's progressive summarization method.
- **LLM assist:** when a note enters the queue, an optional LLM pass proposes first-draft bolding to reduce friction at the raw-note stage; the user reviews/accepts rather than the system silently rewriting content.
- **Queue UI:** surfaces notes needing attention per active project, not globally — keeping the queue scoped and non-overwhelming.

**Open thread:** exact data model for queue state (is it derived purely from `distillation_level` + project status, or does it need its own persisted queue table in SQLite?) — worth a dedicated architecture session.

### 11.5 Guided Project Activation

Activation is Iris's core differentiator, and it's more than a status change (ADR-023). When a project enters `active`, Iris assembles a **focused working environment** — a single activation view answering "here is everything you need to start working on this." This is the tangible form of the whole thesis (resurface the right knowledge when work becomes active), and the adoption "Core Product Test" turns on it: *when a user activates a project, does Iris bring together the knowledge, actions, context, and next steps better than their existing tools?*

**What activation surfaces**, all derived from data already in the graph — this is a query/presentation layer, not new stored state (guardrail 6.10):
- **Linked raw/undistilled notes** — the existing distillation queue for the project (from `distillation_level` + relations).
- **Unresolved decisions** — decision-type or open-question nodes linked to the project.
- **Dependent and blocked tasks** — from the task hierarchy and `blocks`/`depends-on` relations (ADR-017).
- **Related resources** — linked resource nodes.
- **Upcoming calendar constraints** — event nodes touching the project or its window.
- **Recently-added related material** — recently linked/created nodes in the project's neighborhood.
- **Recommended starting set** — an ordered set of suggested next actions.

**AI boundary (same manual-first discipline as distillation).** The environment is fully useful with **zero AI configured**: it surfaces and groups the linked material and lets the user choose where to start. With AI, the *recommended starting set* additionally becomes an AI-suggested ordering ("review schema invariants → resolve the YAML-preservation library → create parser test fixtures"). AI enriches the recommendation; it never gates the environment. This keeps the core differentiator working without AI, per ADR-005.

**Composition with other subsystems:** ADR-018's incremental-queue rule means notes linked *while* the project stays active keep flowing into this environment (not a one-time snapshot). Each surfaced item can carry its "why it's here" rationale, sharing the transparent-retrieval machinery (§15, ADR-024). Activation is therefore a read-model built by querying the substrate on the activation event and kept live while the project is active.

**Open thread:** the precise "recommended starting set" heuristic without AI (e.g. unblocked + highest-priority + on the critical path) needs definition; and whether the activation read-model is recomputed on demand or cached and incrementally updated is a performance decision tied to the distillation-queue-state open thread above.

## 12. Task & Planning Subsystem

Tasks, sprints, timeline, and calendar are not separate features — they're one planning stack reading and writing the same node substrate, at three different time horizons:

```
Sprints   → what am I committing to this week/fortnight (short horizon, capacity-bounded)
Timeline  → how do projects/epics fit together over months (medium horizon, dependency-aware)
Calendar  → what is actually happening on a specific day (ground-level, time-of-day precision)
```

- **Task views as saved queries, not separate data:** Inbox, Today, Upcoming, Someday/Maybe, and Logbook are filtered/sorted views over the same task nodes — Inbox is `project = null`, Today is `scheduled_date = today OR (due_date <= today AND status != done)`, Someday/Maybe is `scheduled_date = null AND due_date = null`, Logbook is `status = done`, ordered by completion time. None of these need their own storage.
- **`scheduled_date` vs. `due_date` are distinct fields**, deliberately — "when I plan to work on it" and "when it's actually due" are different questions and collapsing them (as most todo apps do) loses information.
- **Recurrence** must support three distinct models, not one: fixed (next instance N days after the *original* due date, regardless of completion time — e.g. rent), flexible (next instance N days after *actual completion* — e.g. a recurring review), and custom RRULE for irregular patterns. This is an open thread (see `DECISION_LOG.md`) because the underlying task schema needs one canonical recurrence representation even though three input models are user-facing.
- **Dependencies** are typed relations, reusing the same relation engine as everything else — no separate "dependency graph" data structure. Per ADR-017, only the canonical direction `blocks` (and `depends-on`) is stored; the inverse `blocked-by` is derived from the inverse registry, never written to a file. A task's "blocked" visual state is computed by checking whether anything that `blocks` it is still incomplete.
- **Sprint capacity** is computed, not manually entered: available hours = calendar working-hours window, minus existing events, minus already-placed time blocks, for the sprint's date range. This computed number is what sprint planning checks committed task estimates against. Exact derivation formula is an open thread (see `DECISION_LOG.md`).
- **Timeline/Gantt view** renders project and epic nodes as bars using their date-range properties, with dependency edges drawn as arrows and a computed **critical path** (the dependency chain determining the earliest possible completion date) highlighted — this is a read/derived view, not additional stored state beyond the relations and dates already on the nodes.
- **Calendar sync (Google/Apple) is bidirectional**, not import-only: external events become event nodes; edits in Iris propagate out, edits externally propagate in. This means the calendar layer has an external-sync surface in addition to the local CRDT/git sync every other node type uses — worth treating as its own sync adapter rather than assuming the vault's normal sync path covers it.
- **Unit of effort is the pomodoro, not an abstract story point.** Estimates, actuals, velocity, and burndown are all denominated in pomodoros (or focus-session minutes) because they're grounded in real logged time rather than a relative complexity score that needs a team to calibrate — consistent with Iris being single-user.

## 13. Canvas & Rich Views Subsystem

Covers the more visual/spatial views layered on top of the same node substrate — none of these require new stored data beyond what's already in the node model (§4); they're rendering and interaction modes.

- **Canvas mode** — a freeform, infinite 2D surface for pre-structural thinking (FigJam-style), distinct from both the constellation graph (which visualizes *existing* typed relations) and any single node's content. Items dropped on the canvas — images, sticky-note text fragments, references to existing nodes — are disposable scratch content by default, not obligated to become real nodes. The one deliberate mechanic is **graduation**: a canvas item can be explicitly promoted into a real, typed node (with the canvas position optionally retained as a property), at which point it enters the graph properly. Until graduated, canvas content should not appear in search, the distillation queue, or analytics — it's genuinely pre-structural, not a disguised inbox.
- **Gallery/moodboard view** — a new entry in the view layer (alongside table/graph/board/calendar/document/matrix): an image-forward grid of cards for nodes carrying visual attachments (music references, trading chart screenshots, a moodboard node type). Reads the same node/attachment data as every other view; no separate storage.
- **Rich inline content rendering** — images, an embedded PDF viewer, an inline audio player (for music captures and voice notes — scrub, not just download), syntax-highlighted code blocks, LaTeX/math rendering, and video embeds, all rendered inline within a node's body. The authoring format stays standard markdown/embed syntax throughout (inline image syntax, fenced code blocks) specifically so the "design" lives in how Iris *renders* the file, not in a proprietary body format — portability and git-diffability are unaffected by how rich the rendered view looks.
- **Flow connections** — a typed `flow-next` relation for defining explicit reading/assembly order between distilled notes, distinct from a general semantic relation. Used by the Express/publishing surface when composing several notes into an essay or decision document where sequence matters, not just relatedness.

## 14. Engineering Principles

- **Correctness-first:** no cheapened implementations; defer a feature entirely rather than ship it wrong.
- **Zero-cost-when-unused:** feature cost is proportional to current usage, not existence. Decomposed into four independently-solved dimensions (see ADR-010 in `DECISION_LOG.md`):
  - **Startup weight** — progressive/lazy loading; the current view's data loads first, everything else (full graph index, embedding index, analytics) hydrates in the background; quick capture must be interactive almost immediately regardless of what else is loading.
  - **Memory weight** — never materialize the full graph or full embedding index in memory; query the SQLite cache on demand for whatever neighborhood the current view needs. The constellation view and temporal replay load only their active subset and release it on view change.
  - **Battery weight** — adaptive sync (aggressive foreground, exponential backoff backgrounded, sync-on-wake rather than continuous polling); CRDT reconciliation pauses when backgrounded; scheduled AI/embedding work (nightly digest, batch embedding of new nodes) gated on charging + WiFi, not run inline on every capture.
  - **Storage weight** — scheduled git gc/repack; attachments never committed as binary blobs to git history (content-addressed store instead); configurable history retention depth.
- **Feature modules as separate, lazily-loaded bundles** — the 3D constellation view, the analytics dashboard, the AI layer, and the plugin runtime should be independently loadable rather than bundled into the core app, so a user who never opens them pays no startup/memory cost for their existence.

## 15. Search Ranking

The four search layers (lexical, semantic/vector, structural/graph, temporal — see §2 and the Search section of `OVERVIEW.md`) each return their own ranked list of results, and those lists are scored on fundamentally incompatible scales: lexical produces something like an unbounded BM25 score, semantic produces a cosine similarity in the 0–1 range, structural produces a hop-count or centrality measure. These numbers cannot be meaningfully averaged — a semantic similarity of 0.87 is not "worth" the same as a lexical score of 12.3, because they aren't measuring the same quantity.

**Reciprocal Rank Fusion (RRF) is the combination method.** Rather than trying to reconcile incompatible scores, RRF discards the raw scores and uses only each document's *rank position* within each list. For every document, Iris sums `1 / (k + rank)` across every list the document appears in (with `k` a standard dampening constant, typically 60), then sorts by that combined value. The effect is that a result which ranks *consistently well across multiple signals* beats one that is the single best hit on just one axis — which is exactly the relevance property wanted for a second brain, where a note that's relevant lexically *and* semantically *and* structurally is almost always what the user actually wanted over a lucky one-dimensional top hit.

Worked example for the query "market psychology": a note ranking #1 lexical / #2 semantic / #1 structural scores ≈ 1/61 + 1/62 + 1/61 ≈ 0.049; a note that is the #1 semantic match but appears in no other list scores ≈ 1/61 ≈ 0.016. The consistently-relevant note wins, despite not being the single best match on any one axis, because relevance corroborated across independent signals is stronger evidence than a single strong signal.

**Temporal is applied as a recency boost, not as a fifth fused list.** If recency were fused in as an equal fifth ranked list, a mediocre note edited yesterday could outrank a highly relevant note from eight months ago purely for being recent — which is wrong, because "just written" is not the same as "relevant." Instead, the temporal signal multiplies the *already-fused* RRF score by a small decaying recency factor (near 1 for something edited today, decaying toward 0 for something a year old), nudging ordering among results of otherwise-similar relevance without letting recency override genuine relevance.

**Index freshness — when each layer updates, and the honest asymmetry between them.** The four layers do not all refresh on the same cadence, and this matters because it determines whether a just-edited note is immediately findable:
- **Lexical and structural** indexes are cheap to update, so they refresh on save (or on a short debounce after an edit) — a note is findable by keyword and by relationship essentially as soon as it's written.
- **The vector/semantic index is the exception.** Generating an embedding has real compute/API cost, and the battery-weight principle (ADR-010) says batch embedding work should be gated on charging/WiFi rather than run inline on every keystroke. The honest consequence: **a very recently edited note may not yet be semantically indexed**, so it can be missing from *semantic* results for a short window even though it already appears in lexical/structural results. This is an acceptable, deliberate tradeoff — but it must be surfaced, not hidden: the UI should indicate when semantic indexing is pending (e.g. a subtle "N notes pending semantic indexing" indicator) so the absence is understood rather than mistaken for a bug. When AI/embeddings are not configured at all, the semantic layer is simply absent and the other three carry search entirely.
- **Open thread:** exact embedding-refresh trigger policy (on-save-when-charging, periodic sweep, explicit "index now" action, or a hybrid) is flagged in Open Architectural Threads.

**OCR'd attachment text feeds the index (ADR-022).** Vault-wide OCR extracts text from every image/PDF/scan/ink-note and folds it into the search layers — lexical directly, semantic via embeddings if configured. This extracted text is *derived, rebuildable cache* (like every other index), never a mutation of the canonical attachment or node. OCR is compute-heavy, so it follows the same freshness asymmetry as embeddings: it runs batched in the background (gated on charging/WiFi where sensible) rather than inline on capture, so a just-scanned document may be briefly findable by filename/metadata before its OCR text is indexed — surfaced the same way pending semantic indexing is, not hidden. OCR runs on-device by default; an optional cloud OCR provider (same BYO abstraction as the AI layer) can be configured for higher accuracy when the user opts to send image content out.

**Transparent retrieval — results explain themselves (ADR-024).** Because RRF fuses *ranked signals*, the signals that made a result rank are available at ranking time — so Iris can show *why* a result surfaced rather than presenting an opaque order. Each result can carry a compact, on-demand rationale drawn from its contributing signals: matched terms, title/metadata match, project/area/resource relationship, backlink relevance, recent usage, distillation level, active-project relevance, OCR source, attachment source, semantic similarity, temporal relevance. Example: *"Linked to Project Iris · contains 'lossless YAML' · referenced by 2 active tasks · distillation: highlighted."* This turns retrieval from "trust the model" into "see the reasoning" — consistent with Iris's broader preference for understandable signals over opaque scores (cf. the diagnostic-panel-first framing of vault health), and useful in its own right for refining a search. Compact by default, expandable on demand; never forced verbosity.

## 16. Testing & Correctness

"Correctness-first" is the first engineering principle, so the testing approach is architecture, not an afterthought. Beyond conventional unit tests throughout the Rust core, three pillars target the specific properties that are hardest to retrofit and most catastrophic to get wrong:

- **Property-based testing (`proptest`) for CRDT merge convergence.** A conventional test checks one fixed input against one expected output; property-based testing instead generates hundreds of randomized inputs and asserts a general invariant holds across all of them. For the CRDT layer the invariant that matters is **convergence**: regardless of the order edits arrive in, or how many simulated devices participate, every device must end at an identical final state. The test harness spins up several simulated devices, generates random interleaved edit sequences (insert text, delete, add a relation, change a tag), merges them in randomized orders, and asserts identical results everywhere. When a failing case is found — and it will be — `proptest` automatically **shrinks** it to the minimal reproducing case (e.g. "device A adds a link while device B deletes the target node, in that order") rather than leaving a wall of random operations to sift through. This is the single most important test in the system, because CRDT merge bugs are both subtle and data-destroying.
- **Golden-file round-trip tests for the frontmatter parser.** Parse a real markdown file into memory, serialize it back out, and assert the result is byte-identical to the original. This directly enforces the "files are canonical" guarantee: if parsing ever silently reorders a field, reformats a value, or drops content, the test fails immediately rather than the vault quietly drifting over months of use. Per ADR-019 (lossless editing), the stronger version of this test also asserts losslessness *on edited files*: change one frontmatter field and confirm that key order, comments, whitespace, unknown fields, and the entire body are preserved byte-for-byte apart from the single intended change.
- **Full-cycle cache-rebuild integrity test.** The whole architecture rests on "SQLite is a derived, rebuildable cache." This test proves rather than assumes it: populate a cache normally, wipe it entirely, rebuild purely by re-parsing the vault, then diff the two states and assert they're identical. This is precisely the test that catches an insidious class of bug like "a relation gets recorded during live editing but not during a cold rebuild from files."

**Vault integrity checker (test harness *and* user-facing tool).** A routine that walks every node checking for structural problems — relations pointing at a node ID that no longer exists, an annotation whose anchor resolves to nothing, frontmatter failing schema validation, orphaned attachments. It's built once and used in two roles: as a test-suite assertion (a known-good fixture vault must produce zero issues) and as a real "check my vault's health" action for the user (and, as noted in §5, the same surface that reports quarantined malformed files and runs as the final step of a backup restore).

## 17. Privacy & Accessibility

**Telemetry: none, by default and by principle.** Iris sends zero telemetry, has no phone-home of any kind, and collects no usage analytics on any server — consistent with local-first, no-lock-in, and the eventual open-source posture (it's exactly the property auditors of an open-source repo look for). If crash reporting is ever added, it is strictly opt-in and scrubbed to stack traces only — never vault content, never file paths, never node text. This is significant enough to be recorded as its own decision — see ADR-013 in `DECISION_LOG.md`.

**Accessibility: a standing requirement across every view, not a late polish pass.** The concrete commitments, several of which are load-bearing given how much of Iris's design leans on color and spatial layout:
- Full keyboard navigation across *every* view — including the hard cases of the graph and canvas surfaces, not just table/list views.
- Screen-reader labels on all custom-drawn UI components (the graph, the heatmap, the canvas), which don't get accessibility for free the way native controls do.
- A **colorblind-safe default palette** for everything that encodes meaning in color — most importantly the domain-colored heatmap and the domain-colored graph clusters — with the palette reassignable by the user. This one genuinely matters: a meaningful share of the app's information design is color-coding, and color-coding that a colorblind user can't distinguish is information lost, not cosmetics.
- OS font-scaling respected rather than overridden.
- A reduced-motion setting that disables the temporal graph replay animation and canvas transitions, honoring the OS-level reduced-motion preference where available.

## 18. Tech Stack Summary

| Layer | Technology |
|---|---|
| Core engine | Rust |
| Desktop shell | Tauri |
| iOS/iPad shell | SwiftUI |
| Android/tablet shell | Kotlin Compose |
| Browser extension | Thin client over the sync API (framework TBD) |
| Core↔UI bindings | UniFFI / cxx |
| Storage | Git + Markdown + YAML frontmatter; SQLite (derived cache) |
| Sync (phase 1) | WebSocket + conflict-copy resolution (see ADR-012) |
| Sync (phase 2) | CRDT (Automerge-rs or equivalent — unresolved) |
| External calendar sync | Google Calendar API / CalDAV (Apple) — bidirectional adapter, separate from vault sync |
| Search | Lexical (e.g. tantivy) → vector embeddings → graph index → temporal index |
| Search ranking | Reciprocal Rank Fusion across the four layers; temporal as a recency-boost multiplier (see §15) |
| OCR | Vault-wide; on-device engine by default, optional cloud provider via BYO abstraction; extracted text is derived index data (ADR-022) |
| Reminders | User-authored `reminder` nodes; OS-native local scheduling (no server, no polling) |
| Notifications | OS-native local notifications, fired *only* by user-created reminders — no auto-generated nudges |
| Testing | `proptest` for CRDT convergence; golden-file round-trip for the parser; full-cycle cache-rebuild integrity; vault integrity checker (see §16) |
| Telemetry | None — zero phone-home; opt-in scrubbed crash reports only if ever added (see ADR-013) |
| Security baseline | OS-level full-disk encryption (FileVault/BitLocker/LUKS) — no app-level encryption; see ADR-011 |
| Plugin sandbox | WASM (e.g. wasmtime) |
| AI integration | BYO API key / MCP client |

## 19. Open Architectural Threads

- CRDT library selection (Automerge-rs vs. alternatives).
- Embedding model choice for semantic search.
- Plugin API exact surface and permission model.
- Final file/frontmatter format spec (Phase 0 blocker).
- Distillation queue's precise data model (derived vs. persisted queue state).
- Sprint capacity derivation formula (exact calendar-hours-minus-events-minus-blocks calculation).
- Canonical recurrence rule representation (single stored model underlying fixed/flexible/RRULE input modes).
- Widget data-freshness strategy per platform (WidgetKit/Glance refresh cadence vs. battery-weight constraint in ADR-010).
- Component/Instance override semantics (exactly how a per-instance field override survives a later Component schema edit — last-write-wins on the field, or an explicit "locked" flag per overridden field). Related open item: the Tier-3 starter-system definition-file format (ADR-026), tied to the plugin API surface.
- Anchored-comment anchor resolution order (CRDT position vs. text-fragment fallback vs. orphaned state).
- Canvas-mode graduation UX (what "promote this canvas item to a real node" actually looks like, and whether ungraduated canvas content is ever included in vault export).
- Attachment storage/sync mechanism (git-LFS vs. a custom content-addressed store synced out-of-band) — a Phase 5 (sync-order) decision; blobs are deliberately outside normal git history (§5) so they need their own sync adapter.
- Whether `domain` stays single-valued per node (current decision, for unambiguous color-coding) or gains multi-value support with a coloring tie-breaker (§4).
- Semantic-index refresh trigger policy (on-save-when-charging vs. periodic sweep vs. explicit "index now" vs. hybrid) and how pending-index state is surfaced in the UI (§15).
- Async across the UniFFI boundary (network sync and AI calls are inherently async; UniFFI's async support is a known sharp edge and both sync and AI depend on it) — worth an early spike, since it underpins two whole subsystems.
- Cross-compilation / build pipeline for the five target triples (desktop ×3, iOS, Android), XCFramework generation, and Android NDK setup — real setup work not yet assigned to a phase; belongs in Phase 0/1 tooling.
- Visual/logo direction (non-blocking, cosmetic).
