# Iris — Product Requirements Document

**Status:** Draft v1
**Owner:** Swarup (solo, long-haul build)

---

## 1. Vision

Iris is a local-first, git-backed personal knowledge management app that treats **distillation** — turning captured notes into genuinely internalized, actionable knowledge — as a first-class engineering problem, not an afterthought. It combines the free-form linking of Obsidian, the structured views of Notion, the hierarchical task model of Jira/Confluence, and a project-activation-triggered distillation workflow that none of the incumbents have.

## 2. Problem Statement

Existing PKM tools are excellent at **capture** and adequate at **organize**, but they collapse at **distill** — the stage in Tiago Forte's CODE framework where raw notes become usable knowledge. The typical failure mode: notes accumulate into a "graveyard" that's never revisited, because the only resurfacing mechanisms tools offer are time-based (spaced repetition, daily review) rather than tied to when the knowledge actually becomes relevant — i.e., when a project goes active. Distillation is also inherently multi-visit and layered (progressive summarization), and no mainstream tool models that as structured, trackable state.

## 3. Target User

**Primary:** the builder — a power user who wants a correctness-first, no-compromise second brain across desktop, phone, and tablet, with full data ownership (plain text, versioned, exportable, no lock-in).

**Secondary (later):** other PKM practitioners familiar with PARA/CODE who hit the same distillation wall in Obsidian/Notion/Logseq, reached via an eventual open-source (GPL v3-leaning) release.

## 4. Goals

- Make distillation a structured, trackable, low-friction workflow — the core product differentiator.
- One typed **node** primitive unifying notes, tasks, events, projects, and domain-specific entries (trading journal, reading list, music ideas, daily notes), connected by typed relations.
- True data ownership: plain markdown, git-versioned, human-readable, no proprietary lock-in.
- BYO AI: a provider-agnostic AI layer (API key or MCP) that's a genuine optional multiplier — the app is fully functional with zero AI configured.
- Multi-platform from a shared core: desktop (Win/Mac/Linux), mobile (iOS/Android), tablet (iPad/Android tablet); watch is future, not now.
- Correctness-first engineering: features are built to the standard they deserve or deferred entirely — never cheapened to hit a date.

## 5. Non-Goals (v1)

- Multi-user / team collaboration workspace.
- Watch app.
- Full real-time CRDT sync on day one (ships naive websocket sync with conflict-copy resolution first — see `DECISION_LOG.md` ADR-012; CRDT-grade merge is a later phase).
- Plugin marketplace, monetization, or any startup/business framing — this is a personal project, open-source is the only "distribution" model under consideration.
- **Gamification / engagement mechanics** (XP, levels, achievements, streak-based rewards, skill trees) — deliberately rejected, not deferred. Iris should be motivating because it makes thinking genuinely better, not because it manufactures points. See `DECISION_LOG.md` ADR-009 for the full rationale; honest, non-gamified signals (activity heatmap, plain streaks, the Iris Score reflecting real vault health) serve the same motivational need without the failure modes of a parallel scoring system.
- **Vector graphics / prototyping design tooling** — "design your vault" (§7 Organize/Vault Design) means rich embedded content, custom node schemas, and a freeform pre-structural canvas; it does not mean shape editing, asset creation, or any Figma-style canvas *editing* surface. Iris borrows organizational patterns from design tools, not their editing capabilities.

## 6. Core Concepts

- **Vault** — the top-level container: one git repository holding the entire second brain. Exactly one is active at a time (see `DECISION_LOG.md` ADR-014). Not itself a node.
- **Node** — the universal primitive. Every piece of content (note, task, event, project, area, resource, domain-specific entry; archival is lifecycle state, not a type — ADR-016) is a typed node with frontmatter metadata and a markdown body, connected to other nodes via typed relations.
- **Object hierarchy** — Vault contains Nodes; PARA containers and the Epic/Story/Subtask breakdown are node *types* connected by relations, not physical folders (so one node can occupy several "trees" at once). Two deliberate non-node exceptions: checklist items (inside a Task) and ungraduated scratch content (inside a Canvas). A Space is a cross-cutting lens, not a container. What you see is often a tree; what's stored is always a graph. Full treatment in `ARCHITECTURE.md` §3.
- **PARA** — Projects / Areas / Resources / Archive. Organization by actionability, not topic.
- **CODE** — Capture, Organize, Distill, Express. Iris's differentiation is concentrated at Distill.
- **Distillation queue** — a project-activation-triggered (not calendar-triggered) surface that resurfaces a project's raw notes for progressive processing, with per-note `distillation_level` metadata: `raw → bolded → highlighted → summarized`.

## 7. Feature Requirements

Organized by CODE stage, prioritized MoSCoW for the v1 MVP.

### Capture
- **Must:** quick text capture, node creation with type selection.
- **Should:** capture from multiple modalities (voice, web clip, image) — later phase.
- **Should:** browser extension for in-context web clipping/highlighting and quick capture, talking to the same sync API as every other client — not a separate silo.
- **Could:** handwritten/ink capture on stylus-enabled tablets, storing raw strokes plus an OCR transcript as dual representations of one node.
- **Should:** document scanner (edge detection, perspective correction, multi-page) for receipts/whiteboards/pages — capture lands as a node, not a filed image.
- **Should:** vault-wide OCR indexing — OCR every image/PDF/scan/ink-note and fold the text into search; on-device by default, optional cloud OCR provider via the BYO abstraction (see `DECISION_LOG.md` ADR-022). Closes the main capture/retrieval gap vs. Evernote.
- **Won't (v1):** mobile capture (mobile ships in a later phase).

### Organize
- **Must:** PARA structure (Projects/Areas/Resources/Archive), typed relations between nodes, basic list/table view.
- **Must:** task views as filtered lenses over task nodes — Inbox (unprocessed capture), Today (scheduled/overdue), Upcoming (rolling lookahead), Someday/Maybe (uncommitted), Logbook (completed history).
- **Must:** task dependencies (canonical `blocks`/`depends-on`; `blocked-by` derived per ADR-017), with blocked tasks visually distinct and excluded from Today by default.
- **Should:** kanban/board view, calendar/timeline view.
- **Should:** Eisenhower-matrix priority (urgent × important) as a 2×2 triage view, not just a linear priority field.
- **Should:** recurrence supporting fixed, flexible, and custom-RRULE models (see `ARCHITECTURE.md` §12).
- **Should:** natural-language task capture (parse title/date/priority/tag from one typed line, entirely local — no API call required).
- **Should:** anchored comments — text-range-pinned annotations with threaded replies and a resolve state, distinct from whole-node relations.
- **Should:** Spaces — saved UI/context configurations (pinned nodes, active filter, default view, theme) that reconfigure the whole interface in one switch.
- **Should:** vault import at first-run (Obsidian/plain-Markdown first, then Notion/ENEX/Roam) — onboarding-critical, preserves links/attachments/tags/timestamps/hierarchy (see `DECISION_LOG.md` ADR-025).
- **Should:** Tier-1 node templates (copy-on-use starting points for a single node — daily note, meeting note, etc.); early and mechanically simple (see `DECISION_LOG.md` ADR-026).
- **Could:** graph/constellation view, mind-map view, gallery/moodboard view, canvas mode (freeform pre-structural surface with a "graduate to real node" action).
- **Could:** saved searches as live "smart list" nodes (e.g. "high priority, no due date," "quick wins ≤1 pomodoro").
- **Could:** custom node types, Tier-2 Components/Instances (live-linked templates), and Tier-3 starter systems (whole configured workflows) via the plugin API; controlled vocabularies for domains/priority/workflow states. The three template tiers are defined in ADR-026.

### Plan (Sprints, Timeline, Calendar)
- **Must:** calendar view (day/week/month) showing event nodes and scheduled/due tasks, with drag-to-schedule and drag-to-reschedule.
- **Should:** bidirectional Google/Apple Calendar sync — external events become event nodes and stay in sync in both directions.
- **Should:** sprint planning — time-boxed task commitments per project, with capacity computed from calendar availability rather than entered manually, and a completion-rate/velocity history in pomodoros.
- **Should:** burndown tracking within an active sprint (tasks remaining vs. days remaining).
- **Could:** timeline/Gantt view of projects and epics with dependency arrows and a highlighted critical path.
- **Could:** time-blocking directly on the calendar, linked to a specific node/project as protected focus time.
- **Won't (v1):** team sprints, shared capacity across multiple people — this is a single-user capacity model only.

### Distill
- **Must:** distillation queue triggered by project activation; per-note distillation-level tracking; manual bolding/highlighting/summarizing UI.
- **Must:** guided project activation — activating a project assembles a focused working environment (linked notes, unresolved decisions, dependent/blocked tasks, related resources, calendar constraints, recommended next actions), fully usable with zero AI (see `DECISION_LOG.md` ADR-023, `ARCHITECTURE.md` §11.5). This is the core differentiator made tangible.
- **Should:** LLM-assisted first-pass bolding, and AI-suggested ordering of the activation "recommended starting set" (both require the AI abstraction layer; both optional).
- **Won't (v1):** fully automated summarization without user review — distillation stays human-in-the-loop by design.

### Express
- **Could:** writing/publishing surface that composes distilled notes into shareable output.
- **Won't (v1):** publishing integrations (e.g., direct-to-blog).

### Search
- **Should:** lexical (full-text) search.
- **Could:** semantic (vector), structural (graph-topology), temporal layers — four-layer search is a later-phase goal, not v1.
- **Should (once >1 layer exists):** combine layers via Reciprocal Rank Fusion into a single ranked list, with temporal applied as a recency boost rather than a co-equal ranked layer (see `ARCHITECTURE.md` §15).
- **Should:** search reaches inside attachments — OCR'd text from images/PDFs/scans is searchable (ADR-022), with in-image highlighting of matches at retrieval time.
- **Should:** transparent retrieval — each result can show *why* it surfaced (matched terms, project/link relationship, distillation level, recency, etc.) rather than an opaque ranked list (see `DECISION_LOG.md` ADR-024).

### Platform / Infra
- **Must:** git-backed markdown vault as source of truth; SQLite as a rebuildable derived cache; desktop app (Tauri).
- **Must:** single active vault at a time (multi-vault-on-disk with deliberate switching allowed, but never simultaneous — see `DECISION_LOG.md` ADR-014).
- **Must:** data-safety baseline — soft-delete/Trash with a recovery window, in-session undo, malformed-frontmatter quarantine (never crash or silently drop a file), and git history as the permanent backstop (see `ARCHITECTURE.md` §5).
- **Must:** zero telemetry / no phone-home (see `DECISION_LOG.md` ADR-013).
- **Should:** BYO AI abstraction (API key + MCP), with reasoning and embedding models configured independently.
- **Should:** Pomodoro/focus-session timer tied to a node, with completed-count and actual-vs-estimated time logged against that node.
- **Should:** manual, user-authored reminders (`reminder` node type) delivered via OS-native local notifications — never auto-generated; task/event/sprint/streak states stay as in-app indicators only (see `DECISION_LOG.md` ADR-015).
- **Should:** accessibility as a standing requirement — full keyboard navigation (including graph/canvas), screen-reader labels on custom components, colorblind-safe reassignable palette for color-coded views, OS font-scaling, reduced-motion setting.
- **Should:** restore-from-backup as a first-run path (clone remote → rebuild cache → integrity check).
- **Could:** mobile/tablet shells, sync, plugin API.
- **Could:** desktop/mobile widgets (quick capture, today's tasks, recent nodes) — read-mostly, powered by the local cache rather than a live fetch, per platform widget constraints (see `ARCHITECTURE.md` §9).
- **Won't (v1):** watch app, plugin marketplace, per-file encryption (superseded — see `DECISION_LOG.md` ADR-011; OS-level disk encryption is the accepted baseline).

## 7.5 Adoption & Quality Bars

Some capabilities are already listed above as features, but for *adoption-critical* surfaces "we have one" is not the requirement — "it is good enough that someone leaves their current tool for it" is. This section records those quality bars explicitly (surfaced by `IRIS_MISSING_PRODUCT_CAPABILITIES.md`), because a sophisticated architecture cannot compensate for a mediocre daily surface.

- **Capture must be effortless and near-instant.** Quick capture is interactive in well under a second (already an ADR-010 startup-weight requirement) and must never make the user choose type/project/tags *before* the thought is saved — classify-after-capture, never gate-before-capture. Friction at capture is the fastest way to lose a second-brain user.
- **The editor is a craft surface, not a checkbox.** A sophisticated data model cannot rescue a mediocre writing experience — for a notes tool, the editor *is* the product to the user. The bar: fluid Markdown editing, reliable keyboard behavior, no lag on large notes, rich inline content that renders cleanly. This is called out because "an editor exists" is the easiest requirement to under-build.
- **Today / daily surface must answer "what should I do now?" at a glance** — scheduled + overdue tasks, active-project activation entry points, today's events, and reminders, without configuration.
- **Low-maintenance defaults.** A new user must get value before designing anything — Iris ships with sensible default PARA views, node types, and a starter structure so nobody has to "design a personal operating system before their first useful note." Sophistication is available, not mandatory.
- **Progressive structure.** Structure is added *as needed*, not upfront: a plain note can gain a type, relations, a project, distillation, and tasks incrementally (the checklist→subtask graduation model is one instance). The user is never forced to over-organize early.
- **Onboarding for a stranger, not just the author.** A fresh vault is welcoming (starter templates, example graph) and the three-or-four first-run paths (create / open / **import** / restore) are obvious. Import specifically is onboarding-critical (ADR-025), not a late utility.
- **Migration fidelity gates who can adopt at all (ADR-025).** Importing an existing Obsidian/Notion/Evernote/Markdown vault must preserve links, attachments, tags, timestamps, hierarchy, and task states as fully as the source allows.

These bars are requirements on *how well* the corresponding features are built, tracked here so they aren't quietly satisfied by a minimal implementation.

## 8. Success Metrics

Since this is a solo-usage product before any public release, success is measured by personal-workflow signal, not growth metrics:

- % of captured notes that reach `summarized` distillation level within an active project's lifecycle.
- Reduction in "note graveyard" — nodes untouched 90+ days while their parent project is active.
- Whether Iris is genuinely daily-driven, replacing existing tools slice by slice (capture → organize → distill) rather than running in parallel with them.

## 9. Guiding Constraints

- **Correctness-first:** no shortcuts. A feature is built right or deferred — never shipped cheapened.
- **Zero-cost-when-unused:** feature cost should scale with actual usage (lazy loading, separate bundles, gated background work), not mere existence.
- **Local-first, no lock-in:** the vault must remain fully usable and human-readable without Iris running.
- **AI is optional, never load-bearing:** every core workflow must work with zero AI configured.
- **Lightweight by construction, not by omission:** "lightweight" is decomposed into four independently-solved dimensions — startup, memory, battery, and storage weight (see `DECISION_LOG.md` ADR-010) — so that adding features never means the app gets heavier for someone who doesn't use them.
- **Correctness is tested, not asserted:** the correctness-first principle is backed by a concrete testing strategy — property-based testing for CRDT convergence, golden-file round-trip tests protecting "files are canonical," and a full-cycle cache-rebuild integrity test proving the cache is truly derived (see `ARCHITECTURE.md` §16). The vault integrity checker doubles as a user-facing health tool.

## 10. Open Questions

Tracked in `DECISION_LOG.md` (unresolved section) and `ROADMAP.md` (phase exit criteria): CRDT library choice, embedding model for semantic search, exact plugin API surface, final file/frontmatter format spec, licensing finalization (GPL v3 leaning).
