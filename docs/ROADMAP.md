# Iris — Roadmap

**Format note:** this roadmap is phase-based, not date-based. As a solo build, real calendar timing depends on your available hours/week, which isn't pinned down yet. Each phase has an exit criterion instead of a deadline — once you know your weekly time budget, dates can be layered on top of these phases directly.

> **Which document governs build order?** *This* file (`ROADMAP.md`) is the **authoritative source for build sequence and phase numbering.** When any other document refers to a "Phase N" in the sense of *when something gets built*, it means the phase number defined *here*. `IRIS_PHASED_PRODUCT_PLAN.md` uses a different, finer-grained 13-phase breakdown that is a **thematic maturation map, not a build order** — it groups work by capability layer for discussion, and its phase numbers deliberately do **not** imply build sequence. If the two ever disagree on *what gets built when*, this file wins. A cross-walk table at the bottom of this document maps the two numbering schemes onto each other.

---

## Phase 0 — Foundations
*Nothing user-facing yet; make everything downstream possible.*

- Finalize file/frontmatter format spec (per-node-type YAML schema).
- Rust core skeleton: vault read/write, git integration.
- SQLite cache schema + rebuild-from-vault logic.
- Node type schema definitions (note, task, event, project, area, resource, archive, domain-specific types), designed for later extensibility (custom types, controlled vocabularies for domains/priority/workflow states — full plugin-driven custom types land in Phase 7, but the schema shouldn't need reshaping to support them then).
- Testing foundations from day one, since correctness is the first principle (see `ARCHITECTURE.md` §16): golden-file round-trip tests for the frontmatter parser (parse → serialize → assert byte-identical, enforcing "files are canonical"), the full-cycle cache-rebuild integrity test (wipe cache → rebuild from vault → assert identical state), and the vault integrity checker (walks nodes for broken relations, unresolved anchors, invalid frontmatter) — which doubles as a user-facing health tool later.
- Malformed-frontmatter quarantine behavior (a file that fails to parse is excluded but never crashes a rebuild and is never silently dropped) — cheap to build here, expensive to retrofit.
- Single-active-vault model (ADR-014): the core knows how to open/close a vault and rebuild its cache; multi-vault-on-disk switching is just "open a different folder," not simultaneous vaults.

**Exit criteria:** a typed node can be created/read/updated/deleted via a CLI or test harness, backed by git commits, with the SQLite cache correctly reflecting vault state after a rebuild; the CRDT-convergence property test harness exists in skeleton form even before the CRDT layer itself (Phase 6), so merge work is test-driven from the start.

## Phase 1 — MVP Desktop (Capture + Organize)
*Usable single-player desktop app. No AI, no sync.*

- Tauri desktop shell wired to the Rust core.
- Quick capture (text).
- PARA-based organization (Projects/Areas/Resources/Archive).
- Basic views: list/table, simple kanban.
- Task views as filtered lenses: Inbox, Today, Upcoming, Someday/Maybe, Logbook (see `ARCHITECTURE.md` §12) — cheap to build once task nodes exist, and immediately makes the app daily-driveable for todo use.
- Spaces — saved UI/context configurations (pinned nodes, active filter, default view, theme); low-complexity relative to its daily-use payoff since it's UI state layered on data that already exists.
- Manual reminders (`reminder` node type) with OS-native local notification delivery — user-authored only, never auto-generated (ADR-015); pairs naturally with the task views since this is where due-date nudging would otherwise be expected.
- Soft-delete/Trash with a recovery window, plus in-session undo/redo — the data-safety baseline users expect before trusting an app with real content.
- Anchored comments (text-fragment-fallback version only — the CRDT-backed stable-position version upgrades in Phase 6).
- Named checkpoints and branching, surfaced in plain language over the git integration already being built.
- Dev Mode toggle (raw frontmatter/relations view on any node) — cheap, and useful for your own debugging from day one; also the surface for viewing quarantined malformed files.
- Restore-from-backup as a first-run path (clone remote → rebuild cache → run integrity checker).
- **Vault import at first-run — onboarding-critical, not deferred (ADR-025):** plain-Markdown-folder and Obsidian importers first (they map most directly onto Iris's own format, lowest mapping risk), preserving links → relations, attachments, tags, timestamps, and hierarchy. (Notion/ENEX/Roam importers follow in later phases; the pipeline and the two safest sources land here so real vaults can move in from day one.)
- Tier-1 node templates (ADR-026) — copy-on-use starting points (daily note, meeting note, book review); mechanically simple (`is_template` flag + copy), so they land early with the editor. (Tier-2 Components/Instances and Tier-3 starter systems come with custom types in the plugin phase.)
- Accessibility baseline established now rather than retrofitted: keyboard navigation and screen-reader labels for the views that exist at this phase, and the colorblind-safe palette decision locked before color-coded views (heatmap, graph) arrive.
- Zero-telemetry posture (ADR-013) — trivially "true by doing nothing," but worth an explicit check that no dependency phones home.
- **Basic search — foundational navigation, not an advanced feature (review §8):** title search, full-text body search, and filtering by type/project/domain/tag, plus command-palette navigation. A knowledge app isn't daily-driveable without find, so this belongs in the foundation. (The advanced *four-layer fusion* search — embeddings, RRF, structural/temporal ranking — stays in Phase 4; only basic lexical find/filter moves here.)
- Git commit workflow surfaced in the UI (not just under the hood).
- Desktop widget (today's tasks + quick capture) as an early validator that the SQLite cache is fast enough to power a glance-surface — see Engineering Principles.

**Exit criteria:** daily-driveable for capture + organize — replacing at least one existing tool for that slice of your workflow. Basic search returns results across titles, bodies, and metadata filters.

## Phase 2 — Distillation Queue
*Ship the core differentiator — manual first, AI later.*

- Distillation-level metadata on nodes (`raw → bolded → highlighted → summarized`).
- Project-activation-triggered queue surfacing, per the ADR-018 state machine and trigger rules (activation fires on entering `active`; newly-linked raw notes join the queue incrementally while active).
- Manual bolding/highlighting/summarizing UI, queue logic, progress tracking, project-context review — the full manual distillation experience standing on its own.
- **Guided project activation (ADR-023) — the differentiator made tangible:** activating a project assembles a focused working environment (linked notes, unresolved decisions, dependent/blocked tasks, related resources, calendar constraints, a recommended starting set), all derived from the graph and fully usable with zero AI. This is the make-or-break "Core Product Test" surface, so it ships with the distillation core, not later. (AI-suggested *ordering* of the starting set arrives with the AI layer in Phase 4.)
- **AI-assisted bolding is deliberately *not* in this phase (review §9).** The manual distillation UX must be good on its own merits, not propped up by an LLM disguising weak interaction design — and no provider-specific shortcut should be introduced just to make AI appear earlier. LLM-assisted first-pass bolding arrives in Phase 4, *after* the real provider abstraction exists.

**Exit criteria:** moving a project to `active` surfaces its raw notes for progressive processing; distillation level is visibly tracked per note; the manual distill flow is genuinely usable with zero AI configured.

## Phase 3 — Planning Layer (Sprints, Timeline, Calendar)
*Turn the task substrate into a real single-user planning system.*

- Pomodoro/focus-session timer tied to a node; completed-count and actual-vs-estimated time logged per node.
- Sprint planning: capacity computed from calendar availability (not manually entered), task commitment against that capacity, burndown tracking, velocity history in pomodoros.
- Calendar view (day/week/month) — drag to schedule/reschedule, time-blocking linked to a node/project.
- Bidirectional Google/Apple Calendar sync as a dedicated sync adapter (separate from vault CRDT/git sync).
- Timeline/Gantt view for projects/epics with dependency arrows and critical-path highlighting.
- Task dependencies (canonical `blocks` / `depends-on` relations, inverse `blocked-by` derived per ADR-017); blocked-task visual state.
- Recurrence: fixed, flexible, and custom-RRULE models (canonical storage representation is an open thread — see `DECISION_LOG.md`).

**Exit criteria:** a sprint can be planned against real calendar capacity, tracked to completion with a burndown, and reviewed with an honest velocity number — without any AI involved.

## Phase 4 — AI Layer + Advanced Search
*BYO AI made real; the semantic/fusion layer on top of the basic search already shipped in Phase 1.*

- AI provider abstraction (API key config for Anthropic/OpenAI/Google/etc., MCP client support for local models).
- Reasoning model and embedding model configured as independent settings.
- **LLM-assisted first-pass bolding for the distillation queue** — deliberately deferred here from Phase 2 (review §9), now that the real provider abstraction exists; the manual flow already stands on its own, this accelerates it.
- Vector/semantic search (embeddings — model choice still open) — the semantic layer added *on top of* the basic lexical find/filter that shipped in Phase 1.
- **Vault-wide OCR indexing (ADR-022)** — OCR every image/PDF/scan and fold the extracted text into the lexical and semantic layers, so search reaches inside attachments (the main retrieval gap vs. Evernote). On-device engine by default; optional cloud OCR provider via the same BYO abstraction. In-image match highlighting at retrieval. Pairs with search here because it's a retrieval feature and shares the background-indexing/battery-gating machinery with embeddings.
- Reciprocal Rank Fusion to merge lexical + semantic results into one ranked list (extends naturally to the structural/temporal layers when they arrive in Phase 6; temporal enters as a recency boost, not a co-equal list — see `ARCHITECTURE.md` §15).
- **Transparent retrieval (ADR-024)** — surface *why* each result ranked (matched terms, project/link relationship, distillation level, recency, OCR/attachment source), on demand; turns the fused ranker from a black box into something inspectable.
- AI-suggested ordering of the guided-activation "recommended starting set" (the activation environment itself shipped manual-first in Phase 2).
- Additional importers (Notion export, Evernote ENEX, Roam JSON) building on the Phase 1 import pipeline — ENEX pairs naturally with OCR here (ADR-025).
- AI-assisted sprint/scheduling suggestions layered onto Phase 3's planning views.

**Exit criteria:** app remains fully functional with zero AI configured; when AI is on, distillation LLM-assist, planning suggestions, and semantic search all work; basic search (from Phase 1) is unaffected by whether AI is configured.

## Phase 5 — Sync + Mobile/Tablet + Browser Extension
*Multi-device, plus the capture surface that lives outside the app entirely. Decomposed into sub-gates (review §10) — this preserves full scope while making each an independently-shippable engineering gate rather than one monolithic phase.*

**Prerequisite before any sync code:** write the **sync threat model** (its own short document — review §12). It must state explicitly whether the relay persists/reads vault contents, whether protection is TLS-only or end-to-end, how device keys are provisioned and revoked, and whether attachment blobs are protected in relay/storage. Rejecting per-file encryption (ADR-011) covers data at rest locally but says nothing about the sync trust boundary, so this gap must be closed before building sync.

**Phase 5A — Sync Foundation**
- Two-tier sync, tier 1: naive WebSocket sync with conflict-copy resolution — the losing edit on a detected conflict is preserved as a separate, tagged node rather than silently discarded (ADR-012).
- Device identity; desktop-to-desktop sync first (validates sync without also debugging a mobile shell).
- **Attachment synchronization** as its own adapter (blobs travel with the vault per ADR-020, but via a separate transfer path from the git/text sync — see `ARCHITECTURE.md` §5).
- Sync status/observability UI; recovery behavior.

**Phase 5B — iOS / iPadOS**
- SwiftUI shell via Rust core (UniFFI) bindings.
- Capture, reading, tasks, queue review, tablet-oriented distillation; handwriting/ink where appropriate (raw strokes canonical + OCR transcript).
- Document scanner (edge detection, perspective correction, multi-page) — the mobile-native capture surface for receipts/whiteboards/pages; scans land as nodes and are OCR-indexed via the Phase 4 OCR machinery.
- Mobile widgets (quick capture, today's tasks), respecting platform widget constraints (`ARCHITECTURE.md` §9).

**Phase 5C — Android / Android tablet**
- Compose shell via Rust core (UniFFI) bindings; equivalent native surface.
- Mobile capture, review and task workflows, offline behavior.

**Phase 5D — Browser Extension**
- Web clipper, highlight-to-node, quick-capture popover — thin client over the same sync API.

Rich inline content rendering (images, embedded PDF viewer, inline audio player, syntax-highlighted code, LaTeX, video) is built alongside 5B/5C, since capture-and-read-heavy platforms benefit from it most.

**Exit criteria (whole phase):** the same vault is usable across desktop + at least one mobile platform + the browser extension, with node changes *and attachments* propagating correctly, and the sync threat model documented before any relay was built.

## Phase 6 — CRDT-Grade Sync + Advanced Views/Search
*Real-time multi-device correctness; richer surfaces.*

- CRDT merge layer (replacing/augmenting naive sync) — driven by the `proptest` convergence harness scaffolded back in Phase 0, so merge correctness is test-first rather than tested-after (see `ARCHITECTURE.md` §16).
- Anchored comments upgrade to CRDT-backed stable-position anchoring (Phase 1 shipped the text-fragment-fallback version only).
- Graph/constellation view, mind-map view, semantic-zoom graph.
- Gallery/moodboard view; Canvas mode (freeform pre-structural surface with a graduate-to-node action).
- Structural (graph-topology) and temporal search layers — folded into the existing RRF ranking from Phase 4 (temporal as a recency boost).
- Activity heatmap + analytics dashboard (knowledge velocity, domain balance, distillation depth, retrieval rate — see `OVERVIEW.md` §Meta; deliberately no gamification layer per `DECISION_LOG.md` ADR-009).
- Colorblind-safe palette applied in earnest now that the color-coded surfaces (domain-colored heatmap, graph clusters) exist, plus keyboard navigation and reduced-motion support extended to these new graph/canvas/heatmap views (accessibility baseline was set in Phase 1).

**Exit criteria:** concurrent edits on two devices merge without data loss or manual conflict resolution, verified by the property-based convergence suite; the analytics dashboard reflects real vault health, not vanity counts.

## Phase 7 — Plugin API + Polish
*Extensibility and open-source readiness.*

- WASM plugin runtime + sandboxed plugin API v1.
- Custom node types via the plugin API, plus Tier-2 Components/Instances (edit a Component, propagate to Instances) and Tier-3 starter systems (whole configured workflows — node types + relations + views + statuses + activation behavior, installed as a unit) — the full realization of "design your own vault schema." The three template tiers are defined in `DECISION_LOG.md` ADR-026 (Tier 1 shipped back in the Desktop phase).
- Documentation as a first-class citizen (user docs + plugin dev docs).
- GPL v3 licensing pass; onboarding flow for non-author users.

**Exit criteria:** ready for a public/open-source release candidate.

## Phase 8 — Future / Not Yet Scheduled

- Watch app.
- Community plugin distribution.
- Expanded AI-agent capabilities beyond distillation assist.

---

### A note on scope discipline

The open threads section in `ARCHITECTURE.md` and `DECISION_LOG.md` exists specifically as a forcing function against over-planning. Phase 0 and Phase 1 are the two phases that unblock everything else — if this roadmap starts feeling like a document to keep refining rather than a thing to execute against, that's the signal to go build Phase 0 instead.

One category is intentionally absent from every phase above: gamification (XP, levels, achievements, streaks-with-rewards). It was explored in depth and deliberately rejected — see `DECISION_LOG.md` ADR-009. If it resurfaces as a tempting addition in a later phase, that ADR is the answer, not a reason to re-litigate it.

---

### Cross-walk: ROADMAP phases ↔ `IRIS_PHASED_PRODUCT_PLAN.md` phases

The two documents slice the same work differently. This roadmap (authoritative for build order) is MVP-first and collapses several capability layers into single phases; the phased product plan is a finer-grained thematic map. This table reconciles them so a reference to either can be translated. Where one ROADMAP phase spans multiple plan phases (or vice versa), that's expected — they're different granularities, not a contradiction.

| ROADMAP phase (build order) | Corresponding `IRIS_PHASED_PRODUCT_PLAN.md` phase(s) (thematic) |
|---|---|
| **0 — Foundations** | 0 (Product Constitution) + 1 (Core Engine) |
| **1 — MVP Desktop (Capture + Organize)** | 2 (Desktop Knowledge Workbench) |
| **2 — Distillation Queue** | 3 (Distillation System) |
| **3 — Planning Layer (Sprints/Timeline/Calendar)** | part of 6 (Advanced Knowledge Surfaces — the sprints/timeline/calendar portion) |
| **4 — AI Layer + Search v1** | 4 (Search and Retrieval) + 5 (Optional AI Layer) |
| **5 — Sync + Mobile/Tablet + Browser Extension** | 7 (Sync Foundation) + 8 (Mobile and Tablet) |
| **6 — CRDT-Grade Sync + Advanced Views/Search** | 9 (CRDT-Grade Sync) + remainder of 6 (graphs/dashboards/heatmap) |
| **7 — Plugin API + Polish** | 11 (Plugin System) + 12 (Public Release Readiness) |
| **8 — Future / Not Yet Scheduled** | — (watch app, community plugin distribution, expanded AI) |
| *(no ROADMAP phase — retired)* | 10 (retired: Security and Private Vaults — see ADR-011) |

**Note on ordering divergence:** the most visible difference is planning/sprints — this roadmap builds it early (Phase 3, right after distillation) because it's high daily-use value and needs no AI, whereas the thematic plan files it under "Advanced Knowledge Surfaces" (its Phase 6) alongside graphs and dashboards. Build order follows this roadmap: planning comes before AI and sync.
