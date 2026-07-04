# Iris — Phased Product Plan and AI Collaborator Reference

**Document purpose:** This is the shared reference document for building Iris with help from ChatGPT, Claude, Gemini, and any future AI collaborators. It explains the product intent, non-negotiable principles, architecture assumptions, development phases, and how each collaborator should reason about the project.

**Important framing:** Iris is not being treated as a tiny MVP experiment. The goal is to build a genuine, long-haul, high-quality product. The product should still be built in phases, but each phase is a coherent layer of the final system — not a disposable prototype.

> **⚠️ On phase numbers — read before acting on any "Phase N" reference in this file.** The phases in *this* document are a **thematic maturation map** — they group work by capability layer to make the full system legible and to reason about dependencies. They are **not the build order**, and their numbers do **not** indicate the sequence in which things get implemented. **`ROADMAP.md` is authoritative for build order and phase numbering.** Its phases are numbered differently (an MVP-first 0–8 scheme) from this document's 0–12 thematic scheme. If an AI collaborator or developer is told to "build Phase N," that means the phase number in `ROADMAP.md`, not this file. A cross-walk table at the bottom of `ROADMAP.md` maps the two schemes onto each other; consult it whenever a phase number needs translating. When this document says something like "arrives in Phase 9," treat that as "arrives in the CRDT-grade-sync capability layer," and look up where that falls in the actual build sequence via the cross-walk.

---

## 1. What Iris Is

Iris is a personal second brain and knowledge operating system.

It combines ideas from:

- Obsidian-style local markdown knowledge bases
- Notion-style structured pages and databases
- PARA/CODE personal knowledge management
- Jira/Confluence-style hierarchy for projects, tasks, and knowledge
- AI-assisted distillation and synthesis
- Local-first software design
- Cross-platform native product design

The closest shorthand is:

> Iris is a Notion + Obsidian hybrid with Jira/Confluence-style structure, local-first ownership, and an optional AI agent — unified through a typed node model.

However, Iris is not just a clone of those tools. The central product primitive is the **node**.

A node can represent:

- a note
- a task
- an event
- a project
- an area
- a resource
- (archival is a lifecycle state on any node, not a separate type — ADR-016)
- a daily note
- a trading journal entry
- a music idea
- a reading note
- a research paper
- a future domain-specific object

All of these share a common storage, relation, search, and distillation system.

---

## 2. What Iris Is Not

Iris is explicitly **not**:

- A quick MVP with intentionally reduced ambition
- A weekend SaaS experiment
- A generic AI notes app
- A cloud-first app where user data is trapped in a backend
- A productivity app that optimizes for shallow task capture only
- A tool related to any employer or workplace
- A product whose AI layer is required for basic functionality
- A vector graphics or prototyping design tool — "design your vault" means rich embedded content, custom node schemas, and a freeform pre-structural canvas, not shape editing or asset creation. Iris borrows organizational patterns from design tools (live-linked templates, freeform canvases), not their editing capabilities.
- Gamified — no XP, levels, achievements, or skill trees, by deliberate rejection (ADR-009 in `DECISION_LOG.md`), not by oversight.

Iris must remain useful, understandable, and owned by the user even if:

- the UI disappears
- the AI layer is turned off
- the sync server is unavailable
- the project is open-sourced or forked
- the original developer stops maintaining it

The vault must outlive the app.

---

## 3. Product North Star

The product north star is:

> Capture information, organize it by actionability, resurface it when it becomes relevant, distill it into usable knowledge, and help express that knowledge into real output.

The core workflow is:

```text
Capture → Organize → Activate Project → Resurface Relevant Notes → Distill → Express
```

The most important differentiator is not merely AI chat over notes. The most important differentiator is the **project-activation-triggered distillation queue**.

Most PKM tools allow capture. Many allow organization. Few force or support the distillation step. Iris should make distillation visible, trackable, contextual, and useful.

---

## 4. Philosophical Foundation

### 4.1 CODE Framework

Iris is grounded in Tiago Forte's CODE framework:

1. **Capture** — Save useful information with low friction.
2. **Organize** — Organize by actionability, not topic.
3. **Distill** — Progressively compress raw notes into useful knowledge.
4. **Express** — Turn knowledge into output: decisions, tasks, essays, plans, code, papers, reports, projects, or actions.

Iris should not merely store information. It should help move information toward expression.

### 4.2 PARA

Iris uses PARA as a primary organizational model:

- **Projects** — Active work with a defined outcome.
- **Areas** — Ongoing responsibilities without a fixed end date.
- **Resources** — Reference material organized by topic or interest.
- **Archive** — Inactive material from any previous category.

The key principle is actionability.

A note belongs where it is useful, not where it is taxonomically pure.

### 4.3 Progressive Summarization

Iris tracks the maturity of knowledge using progressive summarization.

A note can move through these states:

```text
raw → bolded → highlighted → summarized
```

Each state means:

- **raw** — Captured but not processed.
- **bolded** — Important lines have been identified.
- **highlighted** — The best of the important lines have been selected.
- **summarized** — The note has a concise human-readable summary.

AI may assist this process, but the user remains in control.

---

## 5. The Core Differentiator: Project-Activation Distillation

The key product insight is:

> Notes should be resurfaced when they become relevant to active work, not according to an arbitrary calendar.

A global review queue becomes guilt. A contextual review queue becomes useful.

When a project moves to `active`, Iris should surface related notes that are still raw or partially distilled.

Example:

```text
Project: Build Iris Sync Layer
Status changed: inactive → active
Iris surfaces:
- 12 raw notes related to CRDTs
- 4 bolded notes about sync conflict UX
- 3 resources about Automerge
- 2 previous architecture decisions
```

This is where Iris becomes meaningfully different from Obsidian, Notion, Logseq, and generic AI note tools.

---

## 6. Product Principles

These principles should guide every feature decision.

### 6.1 The Vault Must Outlive the App

The canonical source of truth is a plain markdown vault with YAML frontmatter stored in git.

The user should never be locked into Iris.

### 6.2 Local-First by Default

Iris should work offline and locally. Cloud services may enhance the product, but the product should not depend on a cloud backend for its core function.

### 6.3 AI Is Optional, Not Foundational

Iris must be fully usable without AI configured.

AI should accelerate:

- distillation
- summarization
- synthesis
- search
- project briefing
- relationship explanation
- outline generation

AI should not silently mutate the vault.

### 6.4 Human-in-the-Loop Distillation

AI suggestions are drafts. The user approves, edits, rejects, or applies them.

No invisible rewriting. No unreviewed mutation.

### 6.5 Correctness Before Convenience

Iris is a second brain. Losing or corrupting notes is unacceptable.

Correctness matters more than speed of shipping.

### 6.6 One Product, Many Surfaces

Desktop, mobile, tablet, and future watch surfaces should share a common core but have platform-appropriate workflows.

Desktop is for deep work.

Phone is for capture, search, review, and lightweight distillation.

Tablet is for reading, review, annotation, and medium-depth work.

### 6.7 Features Must Strengthen the System

A new feature should strengthen at least one of these:

- capture
- organization
- distillation
- retrieval
- expression
- trust
- portability
- cross-device continuity
- extensibility

Features that only add novelty should be deferred.

### 6.8 No Gamification

XP, levels, achievements, skill trees, and streak-based reward mechanics were evaluated in depth — including a redesigned "outcome-based" version intended to resist being gamed — and rejected.

Gamification solves a motivation problem. Iris should not have one by design. The just-in-time distillation trigger, and honest signals like the activity heatmap and plain streak counters, are meant to make using Iris its own reward.

A points system is also, by definition, meaningless outside the app itself — directly opposed to Principle 6.1, that the vault must outlive the app.

This is a standing constraint, not a one-time decision. See `DECISION_LOG.md` ADR-009. AI collaborators should not re-propose gamification mechanics even when framed as "outcome-based" or "hard to game" — the objection is to the category, not to a specific implementation's gameability.

### 6.9 Lightweight by Construction, Not by Omission

"Lightweight" means feature cost is proportional to current usage, not to whether a feature exists. This is decomposed into four independently-solved dimensions, each with its own concrete pattern:

- **Startup weight** — progressive/lazy loading; the current view loads first, everything else hydrates in the background.
- **Memory weight** — never materialize the full graph or embedding index in memory; query the derived cache on demand.
- **Battery weight** — adaptive sync (aggressive foreground, backoff backgrounded); scheduled AI/embedding work gated on charging + WiFi.
- **Storage weight** — scheduled git gc; attachments never committed as binary blobs to git history.

This does not mean cutting scope. It means every new subsystem — the constellation view, the AI layer, the analytics dashboard, the plugin runtime — must have a credible answer for how it costs a user nothing when unused. See `DECISION_LOG.md` ADR-010.

### 6.10 Every Feature Attaches to a Substrate — No Parallel Systems

Iris's breadth is defensible only because nearly every feature attaches to one of four coherent substrates rather than spawning its own isolated system: the **knowledge substrate** (nodes, relations, vault, search, views, graph), the **action substrate** (tasks, projects, calendar, sprints, reminders, focus sessions, dependencies), **distillation & expression** (progressive summarization, the queue, AI assist, synthesis, export), and **access surfaces** (desktop, mobile, tablet, widgets, extension, integrations).

The guardrail is therefore *not* "reduce features" — it's: **every feature must attach to an existing substrate rather than create a parallel one.** Concretely, the trading journal is node types + relations + metadata + views, *not* an isolated mini-application with its own data model. Any proposal that would introduce a second, parallel way of storing or relating content should be redesigned to sit on the node/relation substrate instead, or rejected. This is what keeps a large feature set from fragmenting into several half-integrated apps wearing one name.

### 6.11 Planning Stays Subordinate to Knowledge

The action substrate (sprints, velocity, burndown, Gantt, recurrence, dependencies, calendars, focus sessions, reminders, time-blocking) is genuinely powerful — powerful enough that, left unchecked, it could turn Iris into "a personal Jira with notes attached." That inversion must be resisted. The planning layer exists to answer *"what am I doing with my knowledge?"* — it is in service of the knowledge graph, not the other way round. When a design choice pits planning richness against knowledge-graph integrity or the distillation core, knowledge wins. Iris is a second brain that can also plan work; it is not a project-management tool that also stores notes.

---

## 7. Architecture Assumptions

### 7.1 Canonical Storage

The source of truth is:

```text
Markdown files + YAML frontmatter + git
```

SQLite is not the source of truth. SQLite is a derived cache.

### 7.2 Derived Cache

SQLite exists for:

- fast queries
- relation indexing
- full-text search
- view rendering
- filtering
- metadata queries
- graph traversal support

The cache must be rebuildable from the markdown vault.

### 7.3 Core Architecture

The preferred architecture is:

```text
Rust Core
├── vault engine
├── markdown/frontmatter parser
├── schema validation
├── relation engine
├── git integration
├── SQLite indexing/cache
├── search engine
├── distillation logic
├── sync logic
├── CRDT layer later
├── plugin runtime later
└── AI provider abstraction later

UI Shells
├── Desktop: Tauri
├── iOS/iPadOS: SwiftUI
└── Android/Tablet: Kotlin Compose
```

The Rust core should contain correctness-critical business logic. UI shells should be thin and platform-native.

### 7.4 AI Architecture

The AI layer should be provider-agnostic.

Supported provider direction:

- OpenAI
- Anthropic
- Google
- DeepSeek
- Qwen
- local models
- MCP-compatible providers

The user supplies their own API key or local model endpoint.

### 7.5 Sync Direction

Sync should evolve in two major stages:

1. Naive sync: WebSocket or sync bridge with conflict-copy resolution (losing edits preserved, never silently discarded — see `DECISION_LOG.md` ADR-012) plus conflict logs.
2. CRDT-grade sync: concurrent editing with robust merge behavior.

Naive sync is acceptable as an earlier stage, but CRDT-grade sync is part of the long-term product, not an optional fantasy.

### 7.6 Security Direction

No per-file or app-level encryption. Superseded (ADR-007 → ADR-011 in `DECISION_LOG.md`) after review found the functional cost — search exclusion, meaningless git diffs, AI needing per-action unlock, multi-device key-management friction — outweighed the threat it addressed. OS-level full-disk encryption (FileVault/BitLocker/LUKS) is the accepted baseline, consistent with how Obsidian's core app and most local-first note tools handle this.

### 7.7 Object Hierarchy

The containment/relationship structure of everything in Iris, made explicit here because it differs fundamentally from the folder-based model most note tools use. Full treatment in `ARCHITECTURE.md` §3; the essentials:

- **Vault** is the top level — one git repository, the entire second brain, exactly one active at a time (see 7.10 and ADR-014). Not itself a node; it's the boundary within which nodes exist.
- **Node** is the universal primitive directly beneath the Vault. Almost everything is one.
- **PARA containers (Project/Area/Resource/Archive) are node types connected by typed relations, not physical folders.** A Task relates to its Project the same way any two nodes link — which is precisely what allows a single Task to belong to a Project *and* simultaneously relate to a Resource, a Reading note, and a Trading entry. A filesystem's one-parent containment cannot express this; a graph of typed relations can.
- **Within a Project**, the Epic → Story → Subtask breakdown is likewise nodes joined by parent/child relations, not physical nesting.
- **Two deliberate non-node exceptions**, both being genuine sub-content of a single parent: **checklist items** (inside a Task; promotable to a real Subtask on demand) and **ungraduated canvas scratch content** (inside a Canvas; promoted to real nodes via "graduation," which leaves a `graduated-from` relation). These two exceptions are the *only* things in Iris that aren't first-class nodes.
- **Canvas is both a node type and a view.** Unlike Table/Graph/Board/Calendar/Gallery — which are stateless lenses you apply to a selection — a Canvas is a specific, named, persistent object with its own identity, so it's a node type; when you're looking at one, it's also the freeform spatial rendering of that node's contents.
- **Space is not a container in this hierarchy at all** — it's a saved lens (pinned nodes, filter, default view, theme) that cuts across multiple Projects and Domains. A Space switch is an instant, lightweight context change within the one open Vault; contrast with a Vault switch (7.10), which closes one graph and opens another.
- **The governing mental model:** what the user sees is frequently a tree (PARA, task breakdown), but what is stored is always a graph of typed relations. The tree is the default lens; the graph is the substrate.

### 7.8 Data Safety and Recovery

Because files are canonical and hand-editable, and losing/corrupting a note is treated as unacceptable rather than a minor bug, several protections are first-class (full detail in `ARCHITECTURE.md` §5):

- **Malformed frontmatter → quarantine, never crash, never silently drop.** A file that fails to parse is excluded from cache/graph but listed in a "Needs Attention" surface with the actual error (raw content viewable in Dev Mode). One broken file can degrade only itself, never a whole-vault rebuild.
- **Soft-delete / Trash.** Deletion sets a `deleted_at` marker; the node leaves normal views but stays recoverable in Trash for a retention window (tentatively ~30 days), after which it's git-history-only. This directly addresses the most common data-loss event — an accidental delete — better than an undo stack alone.
- **Undo/redo in two tiers:** an in-session undo stack (ephemeral, for immediate edit mistakes) and git-history restore (durable, further back). Two mechanisms for two timescales, deliberately not conflated.
- **Restore from backup** is an explicit first-run path: clone/pull from a git remote, rebuild the cache, run the vault integrity checker before declaring completion. No periodic auto-verification (unnecessary battery/complexity for a rare event); verification happens at restore time, when it matters.

### 7.9 Testing and Correctness

"Correctness before convenience" (Principle 6.5) is backed by a concrete strategy, not left as an aspiration (full detail in `ARCHITECTURE.md` §16):

- **Property-based testing (`proptest`) for CRDT merge convergence** — generate randomized interleaved edit sequences across simulated devices, assert all devices converge to identical state regardless of order; failures auto-shrink to the minimal reproducing case. This is the single most important test in the system because CRDT merge bugs are both subtle and data-destroying. The harness is scaffolded in Phase 0/1 so the eventual CRDT work (Phase 9) is test-first.
- **Golden-file round-trip tests** for the frontmatter parser (parse → serialize → assert byte-identical) — the mechanical enforcement of "files are canonical."
- **Full-cycle cache-rebuild integrity test** (wipe cache → rebuild from vault → assert identical) — proves rather than assumes the cache is purely derived.
- **Vault integrity checker** — walks all nodes for dangling relations, unresolved anchors, invalid frontmatter, orphaned attachments; serves as both a test assertion (a known-good fixture yields zero issues) and a user-facing health tool.

### 7.10 Single Active Vault

Iris operates on exactly one **active** vault at a time (ADR-014). The graph, all four search layers, the AI context, and analytics operate only over that one vault — never any cross-vault traversal, search, or reasoning, because Iris's "everything connects" premise is defeated the moment knowledge is split across simultaneously-active vaults. The app is not, however, limited to knowing one vault folder ever: **switching** vaults is a supported but deliberate, heavyweight operation (close current, point at a different folder, rebuild cache from scratch). The distinction that matters: a **Space** switch changes point of view instantly within the open vault; a **Vault** switch closes one graph and opens another. Legitimate second-vault uses are a sandbox (testing a plugin or risky schema change) and a parallel vault during migration — the exception, not the daily workflow. "Vault picker" therefore means "point Iris at a vault folder" (first-run, restore, or the rare deliberate switch), a lightweight recent-vaults list in the VS Code / Obsidian style.

---

## 8. Product Phase Map

The phases below are not MVP phases. They are product maturation layers.

Each phase should unlock a serious new mode of use.

| Phase | Name | What It Unlocks |
|---:|---|---|
| 0 | Product Constitution | Stable product contract and invariants |
| 1 | Core Engine | Trusted local-first storage and node operations |
| 2 | Desktop Knowledge Workbench | Serious daily capture, organization, and full task management (Inbox/Today/Upcoming/Someday/Logbook, dependencies, recurrence) |
| 3 | Distillation System | The defining Iris workflow |
| 4 | Search and Retrieval | Deep navigability across the vault |
| 5 | Optional AI Layer | Accelerated distillation and synthesis |
| 6 | Advanced Knowledge Surfaces | Graphs, sprints, timeline/Gantt, dashboards, analytics heatmap, expression tools |
| 7 | Sync Foundation | Multi-device continuity, bidirectional calendar sync |
| 8 | Mobile and Tablet | Ubiquitous capture, review, light work, browser extension, widgets |
| 9 | CRDT-Grade Sync | Correct concurrent editing |
| 10 | *Retired* (was: Security and Private Vaults) | See `DECISION_LOG.md` ADR-011 — per-file encryption was superseded, not deferred |
| 11 | Plugin System | Safe extensibility |
| 12 | Public Release Readiness | Open-source/product-grade polish |

---

# 9. Detailed Phases

## Phase 0 — Product Constitution

**Goal:** Define the product contract before implementation spreads across systems.

This phase prevents drift. It answers what Iris is, what Iris refuses to become, and what invariants every later subsystem must respect.

### Build / decide

- Product principles
- Node model
- Node type taxonomy
- File and frontmatter format
- Vault folder structure
- PARA/CODE interpretation
- Distillation state machine
- Relation model
- Git behavior
- SQLite cache behavior
- Schema versioning
- Migration strategy
- Naming conventions
- Architecture boundaries
- Controlled-vocabulary model for domains/priority/workflow states (defined once, referenced everywhere, distinct from free-form tags)
- Extensibility boundary for custom node types + the three-tier template model (Tier 1 node templates, Tier 2 Components/Instances, Tier 3 starter systems — ADR-026); full Tier 2/3 implementation is the Plugin phase, but the schema should not need reshaping to support it then (Tier 1's `is_template` flag and the `instance-of` relation are already in `SCHEMA_SPEC.md`)
- Object hierarchy (§7.7): confirm PARA and Epic/Story/Subtask are relation-connected node types not folders, and that checklist items + ungraduated canvas content are the only two non-node exceptions
- Data-safety model (§7.8): soft-delete/`deleted_at` semantics, malformed-frontmatter quarantine behavior, the two-tier undo model — decided here so the schema and rebuild logic account for them from the start
- Single-active-vault model (§7.10, ADR-014)
- Zero-telemetry posture (ADR-013) and manual-only reminder model (ADR-015) recorded as binding constraints

### First-party node types

- note
- task
- event
- project
- area
- resource
- (no `archive` type — archival is the `lifecycle` field, ADR-016)
- daily note
- space (saved UI/context configuration)
- annotation (anchored comment, text-range-pinned)
- reminder (user-authored, time-based, manual-only — never auto-generated; ADR-015)
- ink-note (handwritten strokes + OCR transcript; tablet, arrives Phase 8)
- canvas (named freeform surface; is both a node type and a view — arrives Phase 6)
- domain-specific node types later

### Exit criteria

The team and AI collaborators can answer:

- What is a node?
- What is the canonical source of truth?
- What belongs in markdown/frontmatter?
- What belongs only in SQLite?
- How does a project become active?
- How does distillation state move forward?
- What makes Iris different from Obsidian, Notion, Logseq, and generic AI note apps?
- What features would violate Iris's philosophy?

### Primary output

A stable internal product contract.

---

## Phase 1 — Core Engine

**Goal:** Make Iris real as a local-first engine before polishing UI.

This is the product spine.

### Build

- Rust workspace setup
- Vault creation/opening
- Markdown file read/write
- YAML frontmatter parser/writer
- Node CRUD
- Node ID strategy
- Node schema validation
- Malformed-frontmatter quarantine (fail-soft: exclude the file, surface the error, never crash a rebuild or drop the file)
- Git init/commit/status integration
- SQLite cache schema
- Cache rebuild from vault
- Relation indexing
- Basic CLI or test harness
- Migration/versioning system
- Unit and integration tests
- Golden-file round-trip parser tests (parse → serialize → byte-identical) — enforces "files are canonical"
- Full-cycle cache-rebuild integrity test (wipe → rebuild → assert identical state)
- Vault integrity checker (dangling relations, invalid frontmatter, orphaned attachments) — reused later as a user-facing health tool and as the final step of backup restore
- `proptest` CRDT-convergence harness in skeleton form — even though the CRDT layer itself is Phase 9, standing the harness up now makes that later work test-first (see `ARCHITECTURE.md` §16)

### Design rule

The core should be usable through tests or a CLI before the full desktop UI exists.

### Exit criteria

A typed node can be created, read, updated, deleted, related, indexed, and committed through the Rust core.

The vault remains readable without Iris.

SQLite can be deleted and rebuilt from the vault — and this is proven by the integrity test, not just assumed.

A file with broken frontmatter is quarantined and surfaced, and does not prevent the rest of the vault from loading.

---

## Phase 2 — Desktop Knowledge Workbench

**Goal:** Create the first serious daily-use desktop product surface.

This is where Iris becomes usable as a knowledge workbench.

### Build

- Tauri desktop shell
- Vault picker (per §7.10: "point Iris at a vault folder" — first-run, restore, or the rare deliberate switch; a recent-vaults list, not routine multi-vault switching)
- Vault creation flow
- Restore-from-backup flow (clone/pull remote → rebuild cache → run integrity checker before completing)
- Vault import at first-run (ADR-025) — plain-Markdown-folder and Obsidian importers first (lowest mapping risk, map directly onto Iris's own format), preserving links → relations, attachments, tags, timestamps, hierarchy; onboarding-critical, not a late utility (Notion/ENEX/Roam follow in the search/AI phase)
- Sidebar navigation
- Node editor
- Markdown editor
- PARA views
- Project view
- Area view
- Resource view
- Archive view
- Task views: Inbox, Today, Upcoming, Someday/Maybe, Logbook — filtered lenses over one task-node table, not separate storage
- Task dependencies (canonical `blocks`/`depends-on` stored; `blocked-by` derived per ADR-017)
- Recurrence (fixed / flexible / custom RRULE)
- Natural-language task capture (local parsing — date/priority/tag from one typed line)
- Pomodoro/focus-session timer tied to a node, with completed-count and actual-vs-estimated logging
- Manual reminders (`reminder` nodes) + OS-native local notification delivery — user-authored only, never auto-generated (ADR-015); due dates/events/sprint/streak remain in-app indicators that never self-notify
- Soft-delete/Trash with recovery window + in-session undo/redo (git history as the deeper backstop) — the data-safety baseline (§7.8)
- Basic event/date metadata
- List/table views
- Simple kanban view
- Manual linking between nodes
- Backlink display v1
- Anchored comments (text-fragment-fallback version — CRDT-backed stable anchoring upgrades in Phase 9)
- Spaces (saved UI/context configuration: pinned nodes, active filter, default view, theme)
- Named checkpoints and branching, surfaced in plain language over the git integration
- Dev Mode toggle (raw frontmatter/relations view on any node; also the surface for inspecting quarantined malformed files)
- Accessibility baseline: keyboard navigation + screen-reader labels for the views that exist at this phase; colorblind-safe palette decided now, before color-coded views (heatmap/graph) arrive in Phase 6
- Tier-1 node templates (ADR-026) — `is_template`-flagged nodes offered as copy-on-use starting points (daily note, meeting note, book review); the simple copy-on-use tier, landing with the editor (Tier 2/3 come with custom types in the plugin phase)
- Git history/status surfaced in UI
- Command palette
- Settings screen (includes reminder quiet-hours, Trash retention, and the confirmation that nothing phones home per ADR-013)

### Defer from this phase

- AI
- sync
- mobile
- plugins
- graph view
- CRDTs

These are not rejected. They are deferred to protect the foundation. (Encryption is not on this list because it isn't deferred — it's cut entirely; see ADR-011.)

### Exit criteria

Iris can be used as the primary desktop app for:

- capturing notes
- organizing projects
- managing resources
- editing markdown
- tracking basic tasks
- browsing the vault
- viewing git-backed history/status

---

## Phase 3 — Distillation System

**Goal:** Ship the defining Iris workflow.

This phase is where Iris stops being a good local-first PKM and becomes Iris.

### Build

- Project status model:
  - inactive
  - active
  - paused
  - completed
  - archived
- Project activation trigger
- **Guided project activation environment (ADR-023)** — on activation, assemble a focused working view: linked undistilled notes, unresolved decisions, dependent/blocked tasks, related resources, calendar constraints, recently-added material, and a recommended starting set; derived from the graph, fully usable with zero AI (AI adds suggested ordering later)
- Distillation queue
- Per-note distillation level:
  - raw
  - bolded
  - highlighted
  - summarized
- Queue scoped by active project
- Manual bolding UI
- Manual highlighting UI
- Summary field/top-of-note summary UI
- Distillation progress indicators
- Review mode
- "Raw notes for active projects" view
- "Notes blocking this project" view
- Summary node generation
- Distillation history/audit metadata

### Design rule

The queue should be project-contextual, not global.

Bad:

```text
You have 317 notes to review.
```

Good:

```text
You activated Project X. These 17 related raw notes are now relevant.
```

### Exit criteria

When a project becomes active, Iris resurfaces relevant raw or partially distilled notes and helps the user process them into usable knowledge.

---

## Phase 4 — Search and Retrieval

**Goal:** Make the vault deeply navigable.

Search becomes significantly more valuable after typed nodes, organization, and distillation metadata exist.

### Build

- Full-text lexical search
- Search filters:
  - node type
  - project
  - area
  - resource
  - date
  - tag
  - distillation level
  - project status
- Saved searches
- Search result ranking via Reciprocal Rank Fusion — combines each layer's ranked list by rank position rather than incompatible raw scores, so consistently-relevant results beat single-axis lucky hits (see `ARCHITECTURE.md` §15)
- Transparent retrieval (ADR-024) — each result can show *why* it surfaced (matched terms, project/link relationship, distillation level, recency, OCR/attachment source) rather than an opaque ranked list
- Additional importers (Notion export, Evernote ENEX, Roam JSON) building on the Phase-1 import pipeline — ENEX pairs naturally with OCR (ADR-025)
- Backlinks
- Forward links
- Relation browser
- Timeline search
- Recently touched notes
- Stale notes
- Explicit relation-based "find related nodes"
- Search over summaries separately from full raw notes

### Later in this phase

- Embedding index
- Semantic search
- Vault-wide OCR indexing (ADR-022) — OCR images/PDFs/scans/ink-notes into the lexical + semantic index; on-device default, optional cloud provider; in-image match highlighting; derived/rebuildable, background-batched
- Hybrid lexical + semantic search, fused via RRF (temporal enters as a recency boost on the fused ranking, not a co-equal ranked list — full structural/temporal layers land in Phase 6)
- Project-contextual search

### Exit criteria

The user can retrieve knowledge by:

- text
- metadata
- relation
- project context
- time
- distillation state
- node type

---

## Phase 5 — Optional AI Layer

**Goal:** Make AI a multiplier without making Iris dependent on AI.

AI should enter after the manual distillation workflow exists. Otherwise Iris risks becoming a weaker generic summarization app.

### Build

- AI provider abstraction
- BYO API key settings
- Local model/MCP support direction
- Prompt template system
- AI-assisted first-pass bolding
- Suggested highlights
- Draft summaries
- Ask-over-selected-notes
- Ask-over-active-project
- Explain relation between notes
- Generate project brief from summarized notes
- Convert distilled notes into outlines
- Generate expression drafts from selected nodes
- AI safety/review UI for proposed changes

### Non-negotiable rule

AI outputs must be reviewable before being written into the vault.

No silent mutation.

### Exit criteria

Iris remains fully useful with AI off, but becomes meaningfully faster with AI on.

The best AI use case is:

> Help me process raw material into structured understanding.

---

## Phase 6 — Advanced Knowledge Surfaces

**Goal:** Make Iris visually and structurally powerful.

These views should be built after the underlying model is stable.

### Build

- Graph/constellation view
- Calendar view (day/week/month, drag-to-schedule/reschedule, time-blocking)
- Timeline/Gantt view — project/epic bars, dependency arrows, critical-path highlighting, grounded in real calendar-derived capacity rather than drawn estimates alone
- Sprint planning — capacity computed from calendar availability, task commitment against capacity, burndown chart, velocity tracked in pomodoros
- Sprint review/retrospective flow, with the retrospective captured as a real linked node
- Mind-map view
- Gallery/moodboard view — image-forward grid for nodes with visual attachments
- Canvas mode — freeform pre-structural surface with a graduate-to-real-node action; content stays out of search/distillation/analytics until graduated
- Rich inline content rendering — images, embedded PDF viewer, inline audio player, syntax-highlighted code, LaTeX, video — all in standard markdown/embed syntax underneath
- Project dashboard
- Area dashboard
- Resource dashboard
- Distillation dashboard
- Knowledge maturity map
- Stale active-project notes view
- Activity heatmap (domain-colored contribution grid; click a cell to open that day's note) and knowledge analytics dashboard (velocity, domain balance, connectivity score, idea half-life, cross-domain link ratio). **Explicitly excludes any XP/achievement/points mechanic — see Product Principle 6.8 below and `DECISION_LOG.md` ADR-009.**
- Colorblind-safe palette applied in earnest now that color-coded surfaces (domain-colored heatmap, graph clusters) exist, plus keyboard-nav and reduced-motion support extended to the new graph/canvas/heatmap views (baseline was set in Phase 2; §7.9-adjacent accessibility commitments in `ARCHITECTURE.md` §17)
- Express view for producing:
  - essays
  - project reports
  - papers
  - blog drafts
  - plans
  - implementation specs
  - decision documents

### Design rule

Graph view is not decoration. It should answer real questions:

- Which notes support this project?
- Which resources feed this area?
- Which ideas are central?
- Which nodes are isolated?
- Which active projects depend on undistilled notes?

### Exit criteria

Iris provides multiple powerful lenses over the same vault.

---

## Phase 7 — Sync Foundation

**Goal:** Make Iris multi-device with a pragmatic first sync layer.

### Build

- Sync service or local sync bridge
- Device identity
- Conflict-copy resolution: on a detected conflict, the node that syncs first stays canonical; the losing edit is preserved as a separate `conflicted`-tagged node linked to the original, never silently discarded (see `DECISION_LOG.md` ADR-012)
- Conflict badge/list UI — surfaces unreconciled conflict copies until the user resolves them (copy content across, discard, or merge)
- Conflict logs
- Sync status UI
- Manual conflict recovery
- Git commit safety net
- Desktop-to-desktop sync first
- Sync settings
- Bidirectional Google/Apple Calendar sync adapter (a separate external-sync surface from vault CRDT/git sync — external events become event nodes, edits propagate both directions)

### Why desktop-to-desktop first

It validates sync without also debugging:

- mobile app lifecycle
- iOS sandboxing
- Android storage
- background execution
- mobile bindings
- mobile UI constraints

### Exit criteria

Two desktop devices can safely use the same Iris vault with predictable propagation.

---

## Phase 8 — Mobile and Tablet

**Goal:** Bring Iris to capture, review, and light distillation across devices.

Mobile should not initially replicate the full desktop workbench.

### Phone priorities

- Quick capture
- View active projects
- View distillation queue
- Light highlighting
- Task checkoff
- Daily note
- Search
- Read-only or light edit mode
- Home-screen widgets (today's tasks, quick capture, active Pomodoro) — read-mostly, powered by the local cache, respecting per-platform widget API constraints (WidgetKit/Glance)

### Tablet priorities

- Richer editing
- Split-pane reading and distillation
- Project dashboard
- Stylus-friendly handwritten ink capture — raw strokes as canonical content, OCR transcript alongside for search (a real node type, not a lossy one-way conversion)
- Larger review workflows

### Build

- SwiftUI iOS/iPadOS shell
- Kotlin Compose Android/tablet shell
- UniFFI bindings
- Mobile vault access
- Sync integration
- Mobile-safe conflict handling
- Offline mode
- Ink-note node type with dual stroke/OCR-text representation (tablet-only)
- Browser extension (web clipper, highlight-to-node, quick-capture popover) — a thin client over the same sync API as every other device, not a special-cased integration

### Exit criteria

Iris is usable across desktop, phone, and tablet with platform-appropriate workflows.

---

## Phase 9 — CRDT-Grade Sync

**Goal:** Make multi-device correctness robust.

### Build

- CRDT layer
- Concurrent edit merge
- Better conflict UI (extends the Phase 7 conflict-copy badge/list to CRDT-level structural conflicts, not just text)
- Merge testing
- Offline-first editing
- Git snapshots from CRDT state
- Recovery tools
- Stress tests for concurrent edits — the `proptest` convergence harness scaffolded in Phase 1 becomes the primary correctness gate here, generating randomized concurrent edit sequences and asserting all devices converge (auto-shrinking any failure to its minimal case — see `ARCHITECTURE.md` §16)
- Anchored-comment anchoring upgraded to CRDT stable-position tracking (Phase 2 shipped the text-fragment-fallback version only — this is the point where that annotation feature becomes fully robust under concurrent edits)

### Exit criteria

The same note can be edited on two devices and merged without data loss — proven by the property-based convergence suite, not just spot-checked.

---

## Phase 10 — Retired (was: Security and Private Vaults)

**This phase is intentionally retired, not silently removed.** It originally covered per-node `age` encryption for sensitive content. A follow-up review (see `DECISION_LOG.md` ADR-011, superseding ADR-007) concluded the functional cost — search exclusion, meaningless git diffs, AI needing per-action unlock, cross-device key-management friction, and a genuinely unforgiving failure mode (lose the passphrase, lose the notes permanently) — outweighed the threat it addressed, given the realistic threat model (a lost/stolen device) is already covered by OS-level full-disk encryption. Precedent from Obsidian (no native per-note encryption in the core app) and Notion (no client-side encryption at all) supported the same conclusion.

**Backup/export behavior**, which was the one non-encryption-specific item in this phase's original build list, is covered under Meta/System's "Backup to anywhere" (see `OVERVIEW.md`) and doesn't need its own phase.

The phase number is left retired rather than renumbering Phases 11–12 down, to avoid destabilizing cross-references elsewhere in this document and in `ARCHITECTURE.md`/`ROADMAP.md`.

---

## Phase 11 — Plugin System

**Goal:** Make Iris extensible without corrupting the core.

Plugins should come after the core API stabilizes.

### Build

- WASM plugin runtime
- Plugin manifest
- Permission model
- Plugin lifecycle
- Read/write APIs
- UI extension points
- Sandboxing
- Custom node type definition UI (user-defined types with their own frontmatter schema)
- Components/Instances templating (Tier 2, ADR-026) — a Component node whose schema edits propagate to every Instance that hasn't locally overridden the changed field
- Starter systems (Tier 3, ADR-026) — declarative bundles configuring a whole workflow (node types + relations + views + statuses + activation behavior + dashboards + default queries + capture destinations + review workflows) for domains like software projects, research papers, trading journals, reading pipelines; first-party and community systems share one mechanism
- Controlled-vocabulary management UI (rename a domain/priority/workflow-state value once, every referencing node updates)
- First-party example plugins
- Developer documentation

### Good first-party plugin candidates

- Trading journal
- Reading tracker
- Music idea tracker
- Fitness log
- Paper/research manager
- Spaced repetition
- Export formats
- Custom dashboards

### Exit criteria

Plugins can extend Iris safely without unrestricted access to the entire vault.

---

## Phase 12 — Public Release Readiness

**Goal:** Make Iris usable by people other than the original developer.

This is not the beginning. It is the result of product maturity.

### Build

- Installer flows
- Onboarding
- Sample vault
- User documentation
- Architecture documentation
- Plugin developer docs
- Migration guides
- Contribution guide
- License finalization
- Issue templates
- Stability pass
- Design polish
- Accessibility audit (final verification, not first implementation — the baseline was built in Phase 2 and extended to color-coded views in Phase 6; this pass confirms full keyboard nav, screen-reader coverage, the colorblind-safe palette, font-scaling, and reduced-motion across the whole surface)
- Privacy verification — confirm zero telemetry / no phone-home across all dependencies (ADR-013), and that any opt-in crash reporting is content-scrubbed
- Performance profiling
- Public/open-source release checklist

### Exit criteria

A serious PKM user can install Iris, understand the model, create a vault, and use it without personal explanation from the developer.

---

# 10. Phase Ordering Rationale

The recommended order is:

```text
Core → Desktop → Distillation → Search → AI → Advanced Surfaces → Sync → Mobile → CRDT → (Security: retired) → Plugins → Public Release
```

The reasoning:

1. **Core before UI** because correctness-critical storage must be stable.
2. **Desktop before mobile** because deep knowledge work happens on desktop first.
3. **Manual distillation before AI** because AI should accelerate a good workflow, not hide a weak one.
4. **Search after structure** because search is stronger when metadata, relations, and distillation state exist.
5. **Sync before mobile** because mobile without coherent sync creates fragmented vaults.
6. **Naive sync before CRDT** because multi-device usability can arrive before perfect concurrent editing.
7. **Plugins late** because extension APIs should not be designed before the core model stabilizes.

---

# 11. AI Collaborator Instructions

This section is specifically for ChatGPT, Claude, Gemini, and future AI assistants.

## 11.1 Do Not Reframe Iris as a Tiny MVP

The goal is not to reduce Iris to the smallest possible test.

The correct framing is:

> Build the full product through coherent phases.

AI collaborators should help with sequencing, design clarity, architecture, implementation, testing, and documentation — not by repeatedly telling the developer to cut ambition down to a trivial MVP.

Scope discipline is still required, but ambition is not the problem.

## 11.2 Optimize for Product Integrity

When proposing features or implementation plans, preserve these invariants:

- markdown/git vault is canonical
- SQLite is derived
- AI is optional
- user owns the data
- distillation is project-contextual
- Rust core owns business logic
- UI shells are thin/native
- no silent AI mutation
- correctness matters more than speed

## 11.3 Be Honest About Tradeoffs

The developer wants direct, objective guidance.

Do not be a yes-man.

If a design is flawed, say so clearly. If a phase is overloaded, say so. If a technical choice creates long-term pain, explain it.

But do not confuse difficulty with impossibility.

## 11.4 Preserve the Product's Soul

When evaluating a feature, ask:

- Does it improve capture?
- Does it improve organization?
- Does it improve distillation?
- Does it improve retrieval?
- Does it improve expression?
- Does it improve trust?
- Does it improve portability?
- Does it improve cross-device continuity?
- Does it improve extensibility?

If not, it may be scope creep.

## 11.5 Prefer Layered Design

For every subsystem, separate:

- canonical data model
- derived/cache model
- core logic
- UI representation
- sync behavior
- AI behavior
- test strategy

Do not mix UI assumptions into core data structures unless absolutely necessary.

## 11.6 Treat Existing ADRs as Load-Bearing

Current accepted decisions:

- Git-backed markdown vault is source of truth.
- SQLite is rebuildable cache.
- Sync has two tiers: naive first, CRDT later.
- Rust core with native/thin UI shells.
- BYO/provider-agnostic AI, with reasoning and embedding models configured independently.
- Distillation queue is triggered by project activation.
- No per-file/app-level encryption — OS-level disk encryption is the accepted baseline (ADR-011, superseding ADR-007).
- Conflict copies, never silent last-write-wins — the losing edit is always preserved on a sync conflict (ADR-012).
- No telemetry — zero phone-home; opt-in scrubbed crash reports only if ever added (ADR-013).
- One active vault at a time; multi-vault-on-disk with deliberate switching, never simultaneous (ADR-014).
- Reminders are manual-only — user-authored, never auto-generated; state indicators never self-notify (ADR-015).
- GPL v3 is likely but not finalized.
- No gamification layer, ever (ADR-009) — not deferred, rejected.
- Lightweight is a four-dimension construction principle (startup/memory/battery/storage weight), not a scope-cutting exercise (ADR-010).

Do not casually override these. If proposing a change, write it as an explicit ADR update.

## 11.7 Do Not Re-Propose Gamification

Because this comes up naturally when discussing motivation, retention, or "making Iris more engaging": XP, achievements, levels, skill trees, and streak-tied rewards were considered in depth — including versions specifically designed to resist gaming — and rejected as a category (ADR-009), not for lack of a good implementation.

If a motivation/engagement problem genuinely surfaces later, the answer is to make the honest signals (heatmap, streaks, Iris Score/analytics) more visible or better designed — not to add a points layer on top of them.

---

# 12. Open Technical Questions

These need future decisions:

- Exact file/frontmatter schema
- Node ID format
- CRDT library choice
- Embedding model for semantic search
- Plugin API surface
- Plugin permission model
- Distillation queue persistence model
- Mobile vault storage model
- License finalization
- Visual identity/logo direction
- Sprint capacity derivation formula (calendar hours minus events minus time blocks)
- Canonical recurrence rule representation (single stored model underlying fixed/flexible/RRULE) — now also underlies `reminder` nodes, which reuse the same recurrence model (ADR-015)
- Widget refresh cadence per platform vs. battery-weight constraint (ADR-010)
- Reasoning-model vs. embedding-model settings UX (one combined "AI provider" setting or two independent ones)
- Component/Instance (Tier-2, ADR-026) override semantics: how a per-instance field override survives a later Component edit (last-write-wins per field vs. explicit per-field lock); plus the Tier-3 starter-system definition-file format (tied to the plugin API surface)
- Canvas-mode graduation UX, and whether ungraduated canvas content is included in vault export
- Anchored-comment anchor resolution order (CRDT position vs. text-fragment fallback vs. orphaned state)
- Trash retention window default (tentatively ~30 days) and whether it's user-configurable

---

# 13. Immediate Next Actions

Recommended next work items:

1. Write the formal file/frontmatter schema.
2. Define the canonical node model.
3. Define relation types.
4. Define project status transitions.
5. Define distillation state transitions.
6. Create Rust core workspace skeleton.
7. Implement vault create/open.
8. Implement markdown/frontmatter read/write.
9. Implement node CRUD.
10. Implement SQLite cache rebuild.
11. Add git init/status/commit integration.
12. Add test fixtures for sample vaults.

These actions start Phase 0 and Phase 1 without weakening the product vision.

---

# 14. Final Product Thesis

Iris is worth building because it is not just another note-taking app.

Its thesis is:

> A second brain should not merely store captured information. It should know when information becomes relevant, help compress it into understanding, and support turning that understanding into output.

The product should be ambitious, local-first, extensible, AI-enhanced, and serious.

The way to build it is not to shrink the dream.

The way to build it is to layer the dream correctly.
