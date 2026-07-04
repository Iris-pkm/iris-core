# Iris — Node & Frontmatter Schema Specification

**Status:** Draft contract
**Schema version:** 1
**Freeze status:** Not frozen — safe to build against, not yet permanent (see §9 versioning and §11 open questions)

**Decision-status legend** (used throughout this document):
- **Accepted** — settled via an ADR in `DECISION_LOG.md`; changing it requires a superseding ADR.
- **Provisional** — a sensible default is in place so implementation isn't blocked, but it may still change before format freeze.
- **Open** — genuinely undecided; a placeholder is used and flagged.

**What this document is:** the exact, on-disk format of an Iris node. It defines what a node looks like as a real text file — which fields exist, what types they hold, how nodes link to each other, and how the format is allowed to change over time. Everything in Iris reads and writes nodes through this format, so this is the contract the whole system is built against.

**Why it matters more than any other spec:** once real notes exist on disk in this format, changing the format means migrating every existing file. This is the most expensive thing in the entire project to get wrong. It is therefore written deliberately, with every open question flagged rather than silently guessed, and with a versioning mechanism (see §9) so the format *can* evolve safely if it must.

**How to read this if you're not deeply technical:** each section explains the "why" before the "what," and every field is shown in a real example. You don't need to understand YAML or Rust to react to the shape of a node — if a field name feels wrong or a decision feels off, that reaction is exactly the kind of feedback this draft exists to collect before any code is written against it.

---

## 1. The core idea — a node is one text file

Every node in Iris is **one file on disk**. That file has two parts:

1. **Frontmatter** — structured metadata at the very top, written in YAML (a simple, human-readable `key: value` format), fenced between two lines of three dashes (`---`). This holds everything the system needs to *reason about* the node: its ID, type, tags, links to other nodes, timestamps, and so on.
2. **Body** — everything below the closing `---`. This is plain Markdown: the actual human content of the note. It can contain rich content (images, code, math, embedded PDFs/audio) using standard Markdown syntax, so it stays portable and readable in any Markdown editor.

Here is the simplest possible complete node — a basic note:

```markdown
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
domain: trading
tags: [market-psychology, fear]
---

Markets overreact to fear far more than to greed. Worth watching for
capitulation signals rather than euphoria — the downside moves are faster.
```

That's it. The part between the `---` lines is the frontmatter (structured data the app understands); the part below is the body (what you actually wrote). Every node, no matter how complex, follows this same two-part shape.

**Why files-as-the-source-of-truth:** because the file *is* the note — not a database row, not an export — your data is readable and portable forever, with or without Iris. This is the "no lock-in by construction" principle (see `DECISION_LOG.md` ADR-001) made concrete: open the file in any text editor and it makes sense.

**Lossless editing (Accepted — ADR-019).** When Iris modifies a node, it changes *only* the specific frontmatter fields that actually changed, and leaves everything else — key order, your comments, whitespace, any fields Iris doesn't recognize, and the entire Markdown body — byte-for-byte untouched. It does *not* re-write the whole file from an internal model. This means your manual formatting and comments survive Iris touching the file, git diffs stay small and meaningful (only real changes show up), and a file written by a newer or plugin-extended Iris isn't damaged when an older Iris edits it. This has a concrete consequence for how the parser is built (it edits files in place rather than load-then-dump), which is exactly why it's decided here, before any parser code exists.

---

## 2. Where files live — folder layout

The vault is a git repository (a folder whose change-history is tracked by git). Inside it, files are organized in a way that is **for human convenience only** — the app does not rely on folder location to understand a node. A node's identity and relationships live entirely in its frontmatter (see §4 and §5), never in its file path. This is deliberate: it means you can move or rename files freely without breaking anything.

A representative layout:

```
my-vault/
├── .iris/                     # Iris's own working area (see below)
│   ├── manifest.json          # the versioned schema manifest (§9)
│   ├── vocabularies.yaml       # controlled vocabularies: domains, priorities, workflow states (§7)
│   └── cache.sqlite            # the derived cache — rebuildable, NOT source of truth
├── attachments/               # content-addressed blob store (images, audio, PDFs) (§8)
│   └── ab/cdef01234...          # files named by content hash, in hash-prefixed subfolders
├── notes/
│   └── market-psychology.md
├── tasks/
│   └── review-architecture-doc.md
├── projects/
│   └── iris-development.md
└── daily/
    └── 2026-01-15.md
```

Key points:

- **`.iris/` holds machine state, not your notes.** The `cache.sqlite` file is a *derived cache* — it can be deleted at any time and rebuilt by re-reading all the `.md` files (see `ARCHITECTURE.md` §5, ADR-002). It is never the source of truth. It should be listed in `.gitignore` so it isn't committed.
- **Folders are suggestions, not structure.** The app finds and understands nodes by parsing files and reading their frontmatter, not by which folder they sit in. Organizing by folder (`notes/`, `tasks/`) is purely for your comfort when browsing the raw files. *(Open question, §11: whether Iris enforces any folder convention at all, or is fully folder-agnostic. Default for now: folder-agnostic — the app scans the whole vault for `.md` files and trusts frontmatter.)*
- **PARA is not folders.** Remember from `ARCHITECTURE.md` §3: Projects/Areas/Resources/Archive are node *types* connected by relations, not physical folders. The `projects/` folder above is just a convenient place to keep project files; a task "belongs to" a project through a relation in its frontmatter (§5), not by living inside a `projects/` folder.

---

## 3. Field types — the building blocks

Before listing fields, here are the value types this spec uses, so every field below can be described precisely:

| Type | What it is | Example |
|---|---|---|
| **ULID** | A 26-character sortable unique identifier (see §4). | `01JQZ8XYABCDEF0123456789AB` |
| **string** | A line of text. | `Review the architecture doc` |
| **enum** | One value chosen from a fixed, defined set. | `status: active` (from a known list) |
| **datetime** | A precise moment, always in UTC, ISO 8601 format. | `2026-01-15T09:30:00Z` |
| **date** | A calendar day, no time. | `2026-01-15` |
| **list** | Zero or more values. | `tags: [fear, greed]` |
| **bool** | True or false. | `resolved: false` |
| **relation** | A typed link to another node, by that node's ULID (see §5). | see §5 |

**On datetimes and UTC:** every timestamp is stored in UTC (the `Z` at the end means "Zulu time," i.e. UTC). This is deliberate — storing local time would make a note created at "9am" ambiguous once you travel across time zones or sync between devices in different zones. The app converts to your local time for *display*; the file always stores UTC. This prevents an entire category of sync and scheduling bugs.

---

## 4. The shared fields — every node has these

Regardless of type, every node carries the same small set of universal fields. These are the fields the core engine depends on; type-specific fields (§6) are layered on top.

| Field | Type | Required? | Meaning |
|---|---|---|---|
| `id` | ULID | **yes** | The node's permanent, unique identity. Never changes, even if the file is renamed or moved. This is what relations point at. |
| `type` | enum | **yes** | What kind of node this is: `note`, `task`, `event`, `project`, `area`, `resource`, `space`, `annotation`, `ink-note`, `reminder`, `daily-note`, or a domain-specific / user-defined type. Determines which type-specific fields (§6) apply. **Note:** `archive` is *not* a type (Accepted — ADR-016); archival is lifecycle state (below), so an archived project keeps `type: project`. |
| `created` | datetime | **yes** | When the node was first created (UTC). |
| `modified` | datetime | **yes** | When the node was last changed (UTC). Updated on every edit. |
| `schema_version` | integer | **yes** | Which version of this schema the file was written against (see §9). Lets future Iris read older files correctly. |
| `lifecycle` | enum | no | `active` (default, may be omitted) or `archived`. Orthogonal to `type` — any node type can be archived without changing what it *is* (Accepted — ADR-016). "Archive" is a system view filtering for `lifecycle: archived`, not a container. |
| `archived_at` | datetime | no | When the node was archived (UTC). Present iff `lifecycle: archived`. |
| `domain` | enum (single) | no | The node's one primary sphere of life (e.g. `trading`, `music`, `iris-dev`), chosen from the controlled vocabulary (§7). Single-valued on purpose so color-coding is unambiguous (`ARCHITECTURE.md` §4). **Provisional** — single-value; see §11. Omit if the node has no natural domain. |
| `tags` | list of strings | no | Zero or more free-form labels for open-ended, cross-cutting categorization. Unlike `domain`, tags are uncontrolled and unlimited. |
| `relations` | list of relations | no | Typed links to other nodes (§5). Omit or leave empty if the node links to nothing. |
| `deleted_at` | datetime | no | If present, the node is in the Trash as of this time (`ARCHITECTURE.md` §5). Absent means the node is not trashed. Distinct from `archived_at`: **archived** = preserved but inactive; **trashed** = recoverable, pending removal. |
| `is_template` | bool | no | If `true`, this node is a **Tier-1 template** (copy-on-use — see the template-tiers note below): excluded from normal views/search, offered as a starting point when creating a node, and **copied** (not linked) on use. Absent/`false` for ordinary nodes. Distinct from a Tier-2 Component, which is marked by *other* nodes carrying an `instance-of` relation to it, not by this flag. |

**Why `id` is a ULID and not the filename:** filenames change (you rename `untitled.md` to `market-psychology.md`), but a node's identity must not. So every node has a permanent ULID, and *relations point at ULIDs, never filenames* (§5). ULIDs are chosen over the more familiar UUID because they are **sortable by creation time** — two ULIDs can be compared to know which node was made first, which is useful for the temporal features, and they're slightly more compact. Functionally, if you've heard of UUIDs, a ULID does the same job (a globally unique ID) with those two bonuses.

**Why `schema_version` is on every file:** this is the safety valve that lets the format evolve. If the schema ever changes (a field is renamed, a new required field is added), old files still declare which version they were written against, so the app knows how to read and, if needed, upgrade them. Without this, any format change would be a catastrophe; with it, format changes become a routine, safe migration. See §9.

---

## 5. Relations — how nodes link to each other

Relations are the heart of Iris: the typed links that turn a pile of notes into a connected graph. A relation is stored in the source node's frontmatter as a small structure with two parts: **what kind of link it is** (the type) and **which node it points to** (the target's ULID).

```yaml
relations:
  - type: parent_project
    target: 01JQZ8PROJECTID0000000000AB
  - type: related-to
    target: 01JQZ8NOTEID000000000000CD
  - type: blocks
    target: 01JQZ8TASKID000000000000EF
```

Read aloud, that says: "this node's parent project is the node with that first ID; it's related to the second; and it blocks the third."

**Why target-by-ULID, never by filename:** if relations pointed at filenames and you renamed a file, every link to it would break. By pointing at the permanent ULID (§4), links survive any rename or move. This is why the whole system can be folder-agnostic (§2).

**The relation types** fall into two families (from `ARCHITECTURE.md` §3). **Canonical-direction rule (Accepted — ADR-017):** each relationship is stored on exactly *one* side, in one canonical direction. The inverse is never written to a file — it's computed by the engine from the inverse registry (below). This makes contradictory states impossible: there is only ever one stored edge.

*Structural / hierarchy relations* (the Jira-flavored, tree-shaped ones) — canonical directions:

| Canonical type (stored) | Meaning | Derived inverse (never stored) |
|---|---|---|
| `parent_project` | This node belongs to that Project. | `project-contains` |
| `parent` | This node is a child of that node (Epic→Story→Subtask nesting). | `children` |
| `blocks` | This node blocks that node (that one can't proceed until this is done). | `blocked-by` |
| `depends-on` | This node depends on that one. | `depended-on-by` |

*Associative / freeform relations* (the Obsidian-flavored, web-shaped ones) — canonical directions:

| Canonical type (stored) | Meaning | Derived inverse (never stored) |
|---|---|---|
| `related-to` | General association. *Symmetric* — its own inverse. | `related-to` |
| `references` | This node cites or refers to that one. | `referenced-by` |
| `annotates` | This node (an annotation) is anchored to that node (§6). | `annotated-by` |
| `graduated-from` | This node was promoted out of a Canvas; points back to it. | `graduated-into` |
| `flow-next` | Reading/assembly order: that node comes next after this one. | `flow-prev` |
| `instance-of` | This node is an Instance of that Component template. | `has-instance` |

**The inverse registry.** The engine holds a single table mapping each canonical relation type to its inverse label (e.g. `blocks → blocked-by`, `parent → children`, `references → referenced-by`, `related-to → related-to`). This registry is the *only* place inverse relationships are defined; the UI and query engine use it to offer full bidirectional navigation ("what blocks this?", "what's blocked by this?") without any inverse edge ever being stored in a file. **Only canonical edge types are ever written to disk.** If a file is hand-edited to contain a non-canonical direction (e.g. someone writes `blocked-by` directly), the integrity checker (§11 note / `ARCHITECTURE.md` §16) flags it for normalization rather than treating it as a second, independent edge.

**On directionality and backlinks:** because relations are stored once and inverses are derived, backlinks ("what links to me") are *always* computed from the single stored edge set when the cache is built — they can never fall out of sync with forward links, because there is only one copy of the truth.

**This is where the integrity checker earns its keep:** a relation can point at a ULID that doesn't exist (target hard-deleted, or a bad hand-edit). The vault integrity checker (`ARCHITECTURE.md` §16) flags these "dangling relations" rather than letting them cause silent bugs.

---

## 6. Type-specific fields — what each node type adds

Every node has the shared fields (§4). On top of those, each type adds its own fields. Below are the built-in types with their additional fields. Fields not marked required are optional.

### `note`
The default. Adds one field:

| Field | Type | Meaning |
|---|---|---|
| `distillation_level` | enum | One of `raw`, `bolded`, `highlighted`, `summarized` — how far this note has been progressively distilled (`ARCHITECTURE.md` §11). Defaults to `raw`. |

```markdown
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-16T14:00:00Z
schema_version: 1
domain: trading
tags: [market-psychology]
distillation_level: bolded
---

**Markets overreact to fear far more than to greed.** Worth watching for
capitulation signals rather than euphoria.
```

### `task`
The workhorse of the planning system.

| Field | Type | Meaning |
|---|---|---|
| `status` | enum | The workflow state, from the controlled vocabulary (§7) — e.g. `todo`, `doing`, `done`. Per-project custom states are allowed. |
| `priority` | enum | `urgent`, `high`, `normal`, `low` — or the Eisenhower two-axis form (§11 open question). |
| `scheduled_date` | date | When you *plan* to work on it. |
| `due_date` | date | When it's actually *due*. Deliberately distinct from `scheduled_date` (`ARCHITECTURE.md` §12). |
| `estimated_pomodoros` | integer | Your effort estimate, in pomodoros. |
| `actual_pomodoros` | integer | Auto-filled from completed focus sessions. |
| `recurrence` | recurrence object | If the task repeats — see the recurrence note below. |
| `checklist` | list of checklist items | Lightweight sub-items (the non-node exception, `ARCHITECTURE.md` §3). Each is `{ text: string, done: bool }`. |

```markdown
---
id: 01JQZ8TASKID000000000000EF
type: task
created: 2026-01-15T10:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
domain: iris-dev
status: todo
priority: high
scheduled_date: 2026-01-17
due_date: 2026-01-20
estimated_pomodoros: 3
relations:
  - type: parent_project
    target: 01JQZ8PROJECTID0000000000AB
checklist:
  - text: Re-read the sync section
    done: false
  - text: Note any open questions
    done: false
---

Review the architecture doc before the planning session.
```

**On recurrence:** the `recurrence` field holds one of the three models from `ARCHITECTURE.md` §12 — fixed (next instance a set interval after the original due date), flexible (a set interval after *actual completion*), or a custom RRULE (the iCalendar standard for complex patterns like "every second Tuesday"). The *canonical stored representation* of recurrence is an open thread (§11) — the three user-facing modes need one underlying storage shape, not yet finalized. For this draft, recurrence is written as:
```yaml
recurrence:
  mode: fixed        # fixed | flexible | rrule
  interval: P1M       # ISO 8601 duration for fixed/flexible; or...
  rrule: "FREQ=MONTHLY;BYDAY=2TU"   # ...an RRULE string when mode is rrule
```
This same recurrence object is reused by `reminder` nodes (ADR-015).

### `event`

| Field | Type | Meaning |
|---|---|---|
| `start` | datetime | When it starts (UTC). |
| `end` | datetime | When it ends (UTC). |
| `recurrence` | recurrence object | As above, if it repeats. |
| `external_id` | string | If synced from Google/Apple Calendar, the source event's ID, so bidirectional sync can match them (`ARCHITECTURE.md` §12). |

### `project`

| Field | Type | Meaning |
|---|---|---|
| `status` | enum | The project's state in its lifecycle (see state machine below). |
| `start_date` | date | For the timeline/Gantt view. |
| `target_date` | date | Estimated completion, for the timeline. |

**Project state machine (Accepted — ADR-018).** `status` is one of `someday`, `planned`, `active`, `paused`, `completed`, `cancelled`. Legal transitions:
```
someday  → planned → active → paused → active
active   → completed
planned  → cancelled
paused   → cancelled
completed → (archived via lifecycle, ADR-016)
cancelled → (archived via lifecycle, ADR-016)
```
Note "archived" is *not* a `status` value — it's the `lifecycle` field (§4), so a completed-then-archived project is `status: completed, lifecycle: archived`, still `type: project`.

**Distillation trigger rules (the reason this must be precise).** A project entering `active` is what drives the distillation queue (ADR-006), so:
1. Activation fires on any transition *into* `active` from a non-active state.
2. It does **not** re-fire merely because an already-active project is opened or viewed.
3. While a project stays `active`, any newly-linked raw note is added to its distillation queue incrementally — activation is a continuously-maintained membership, not a one-time snapshot taken at the moment of activation. (Without rule 3, notes linked after activation would never distill — a real hole this rule closes.)

### `area`, `resource`
These PARA container types mostly rely on the shared fields plus their `type`. `resource` adds optional `source_url` (string) and `read_status` (enum: `unread`/`reading`/`read`) for the reading pipeline. *(There is no `archive` type — archival is the `lifecycle` field on any node, §4, ADR-016.)*

### `reminder`
Manual-only (ADR-015).

| Field | Type | Meaning |
|---|---|---|
| `text` | string | **User-authored.** Never auto-generated. |
| `fire_at` | datetime | When to fire (UTC), or use `recurrence` for repeating reminders. |
| `recurrence` | recurrence object | Optional, same shape as tasks. |
| `status` | enum | `pending`, `fired`, `dismissed`, `snoozed`. |
| (target) | relation | Optionally a `related-to` relation pointing at the node the reminder is about. May stand alone with no target. |

### `annotation`
An anchored comment (`ARCHITECTURE.md` §4).

| Field | Type | Meaning |
|---|---|---|
| `resolved` | bool | Whether the comment has been marked handled. |
| `anchor` | anchor object | How the comment attaches to a text range — carries both a CRDT position (once that layer exists) and a text-fragment fallback. If neither resolves, the annotation goes to an `orphaned` state. |
| (target) | relation | An `annotates` relation pointing at the node this comment is on. For a threaded reply, an `annotates` relation pointing at the parent annotation instead. |

*(The exact internal shape of the `anchor` object is a §11 open question — it depends on the CRDT library choice, which is itself unresolved. For this draft it's a placeholder holding a text-fragment string plus an optional CRDT position.)*

### `space`
A saved lens/context (`ARCHITECTURE.md` §3), stored as a node so it versions and syncs like everything else.

| Field | Type | Meaning |
|---|---|---|
| `pinned` | list of ULIDs | Nodes/searches pinned in this Space. |
| `active_filter` | string | The domain/tag filter this Space applies. |
| `default_view` | enum | Which view opens by default (table/graph/board/calendar/…). |
| `theme` | string | The accent/theme for this Space. |

### `ink-note`
Handwritten (`ARCHITECTURE.md` §4). The stroke data is a binary attachment (§8) referenced by hash; the OCR transcript is stored in the body so it's searchable. Adds `ink_attachment` (the hash of the stroke-data blob).

### `daily-note`
Adds `date` (the calendar day it represents). Otherwise a note.

### Domain-specific & user-defined types
Types like `trading-journal-entry`, `music-idea`, and `reading-item` add their own fields (e.g. a trading entry adds `symbol`, `thesis`, `entry`, `exit`, `pnl`). User-defined types (via the plugin system, much later) follow the exact same pattern — shared fields plus a type-specific set — which is why the schema must not assume the type list is closed. **The parser must treat an unknown `type` gracefully** (preserve it and its unknown fields rather than rejecting the file), so that a vault written by a future or plugin-extended Iris isn't corrupted by an older one.

### Type taxonomy — three tiers

To keep "which types exist" legible (review §13), node types are classified into three tiers:

- **Reserved first-party types (in this spec, built now or soon):** `note`, `task`, `event`, `project`, `area`, `resource`, `space`, `annotation`, `reminder`, `daily-note`, `ink-note`, and the domain modules (`trading-journal-entry`, `music-idea`, `reading-item`).
- **Future first-party types (named, not yet fully specified):** `canvas` (see the note below — its canonical storage is an **Open** question), and any Component *template* nodes used by the Components/Instances mechanism (`ARCHITECTURE.md` §4).
- **Plugin-defined types (arbitrary, arrive with the plugin system):** user- or plugin-authored types. The graceful-unknown-type rule above exists precisely so these coexist safely with older Iris versions.

**Canvas — an explicit open question (Open).** A Canvas is both a node type and a view (`ARCHITECTURE.md` §3), and ungraduated scratch content lives *inside* it as a non-node exception. But the Canvas itself still needs canonical identity and storage, and the shape of that storage isn't decided: is a Canvas (a) a normal node whose *body* holds a structured (e.g. JSON) canvas document, (b) a separate canonical file format alongside `.md`, (c) a specialized kind of `space`, or (d) plugin-defined? This must be chosen **before** Canvas implementation begins. Tracked in `DECISION_LOG.md`. Until then, `canvas` is a reserved-but-unspecified type.

### Templates — how the three tiers appear on disk (ADR-026)

"Template" means three distinct things in Iris (full model in `ARCHITECTURE.md` §4). Their on-disk footprint:

- **Tier 1 — Node template (copy-on-use).** Any node with `is_template: true` in frontmatter. Excluded from normal views/search; offered as a starting point; **copied** on use. The copy is an ordinary independent node with no back-link to the template — so nothing in the copy's frontmatter records its template origin, and later template edits don't affect it. This is the only tier that uses the `is_template` field.
- **Tier 2 — Component / Instance (live-linked).** A Component is a node others point at; an Instance is any node carrying an `instance-of` relation (§5) back to that Component. There is no special `is_template` flag on a Component — its "template-ness" is defined entirely by other nodes' `instance-of` relations to it (a Component may itself be an ordinary, fully-real node that also serves as a template). Field inheritance/override and change propagation are engine behavior over that relation, not extra stored fields; the only persisted trace is the `instance-of` edge. *(How an override survives a later Component edit — last-write-wins per field vs. an explicit per-field lock — is Open.)*
- **Tier 3 — Starter system (configured workflow).** Not a single node at all: a declarative bundle (node types + relations + views + statuses + activation behavior + dashboards + default queries + capture destinations + review workflows) installed as a unit. Its on-disk representation is a starter-system definition file (format TBD, aligned with the plugin/custom-type manifest) plus the nodes/relations/config it materializes on install. Because it's declarative and shares the plugin authoring form, first-party and community starter systems use one mechanism. *(Definition-file format is Open, tied to the plugin API surface.)*

---

## 7. Controlled vocabularies — domains, priorities, workflow states

Some values shouldn't be free-typed, because free-typing them causes silent fragmentation (`trading` vs `Trading` vs `#trading` becoming three different things). These live in one file, `.iris/vocabularies.yaml`, defined once and referenced everywhere (`ARCHITECTURE.md` §4):

```yaml
domains:
  - id: trading
    label: Trading
    color: "#3B82F6"
  - id: music
    label: Music
    color: "#8B5CF6"
  - id: iris-dev
    label: Iris Dev
    color: "#10B981"

priorities: [urgent, high, normal, low]

workflow_states:
  default: [todo, doing, done]
  # projects may define their own state lists
```

Because a domain is defined once with its color here, renaming it (or recoloring it) updates everywhere it's used in one operation. The `color` is what drives the heatmap and graph-cluster coloring — which is exactly why `domain` is single-valued per node (§4), so each node maps to exactly one color.

---

## 8. Attachments — images, audio, PDFs

Binary content isn't stored inline in the Markdown file. Instead (from `ARCHITECTURE.md` §5):

- The blob (image/audio/PDF) is stored in `attachments/`, named by its **content hash** (a fingerprint of the file's bytes). Identical files therefore dedupe automatically, and any corruption is detectable by re-hashing.
- The node's body references it with normal Markdown, which Iris resolves to the hash on save. So a note might contain `![chart](attachments/ab/cdef0123...)`.

**What "the canonical vault" is (Accepted — ADR-020).** The complete canonical vault is the **Markdown/git repository *plus* the content-addressed blob store, together** — *neither alone is the whole vault.* A plain `git clone` reconstructs the text and metadata but **not** the attachments (blobs are deliberately kept out of ordinary git history to avoid bloating it). Consequences that are now firm commitments, not open questions:
- The vault manifest (`.iris/manifest.json`) tracks which blobs are required, by hash.
- The integrity checker (§11 note) reports any *missing* attachment (a hash referenced by a node but absent from the store).
- Backup and restore must include **both** the node files and the blob content to count as complete — the first-run "restore from backup" flow (`ARCHITECTURE.md` §5) fetches both the repo and its blobs before declaring the restore finished.

**Still open (implementation detail, not canonicality):** the *transfer mechanism* for blobs between devices — git-LFS vs. a custom content-addressed store synced out-of-band — is a Phase 5 (sync) decision. What's settled is *that* attachments are canonical and must travel with the vault; *how* they sync is the remaining choice.

**OCR text is derived, not canonical (ADR-022).** Every image/PDF/scan/ink-note is OCR'd vault-wide and its extracted text folded into the search index. That extracted text is *derived, rebuildable cache* — it lives in the SQLite/search index, never in the canonical attachment or the node frontmatter/body, and it can be regenerated by re-OCR'ing the blobs at any time (like any other index). So OCR adds no fields to this schema; it's a search-layer concern, noted here only because it operates on the attachments defined in this section.

---

## 9. Schema versioning — how the format is allowed to change

This is the safety mechanism that makes every decision in this document *revisable* rather than permanent-and-terrifying.

- Every node file carries `schema_version` (§4).
- The vault as a whole records the current schema version in `.iris/manifest.json`, along with a machine-readable description of every node type and relation type (this is the "self-describing export" from `ARCHITECTURE.md` §5).
- When Iris opens a vault whose files declare an older `schema_version`, it runs a **migration**: a defined, tested transformation that upgrades old files to the current shape (e.g. "in v2, the `tags` field was renamed — here's how to convert a v1 file"). Migrations are one-way, versioned, and covered by the same testing rigor as everything else.

**Why this section is what makes the rest safe:** you asked, reasonably, to trust my instinct on field names and structure. Versioning is what makes that trust low-risk. If a field name here turns out to be wrong, it's not a catastrophe — it's a v1→v2 migration. The cost of a mistake in this spec is bounded by the existence of this mechanism. That's precisely why it's a required field from the very first file, not something added later.

---

## 10. A complete worked example — a small connected vault

To make the whole thing concrete, here are four nodes that reference each other: a project, a task belonging to it, a note related to the task, and an annotation on the note.

**`projects/iris-development.md`**
```markdown
---
id: 01JQZ8PROJECTID0000000000AB
type: project
created: 2026-01-10T08:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
domain: iris-dev
status: active
target_date: 2026-06-30
---

Building Iris. Long-haul personal project.
```

**`tasks/review-architecture-doc.md`**
```markdown
---
id: 01JQZ8TASKID000000000000EF
type: task
created: 2026-01-15T10:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
domain: iris-dev
status: todo
priority: high
due_date: 2026-01-20
relations:
  - type: parent_project
    target: 01JQZ8PROJECTID0000000000AB
---

Review the architecture doc before the planning session.
```

**`notes/sync-tradeoffs.md`**
```markdown
---
id: 01JQZ8NOTEID000000000000CD
type: note
created: 2026-01-15T11:00:00Z
modified: 2026-01-15T11:00:00Z
schema_version: 1
domain: iris-dev
tags: [sync, crdt]
distillation_level: raw
relations:
  - type: related-to
    target: 01JQZ8TASKID000000000000EF
---

Naive sync ships first, CRDT later. Conflict copies mean no silent data loss.
```

**`notes/sync-tradeoffs-comment.md`** (an annotation on the note above)
```markdown
---
id: 01JQZ8ANNOTID00000000000GH
type: annotation
created: 2026-01-16T09:00:00Z
modified: 2026-01-16T09:00:00Z
schema_version: 1
resolved: false
anchor:
  text_fragment: "CRDT later"
relations:
  - type: annotates
    target: 01JQZ8NOTEID000000000000CD
---

Revisit this once the CRDT library is chosen — the "later" might move earlier.
```

From these four files alone, the engine can build the entire graph: the task belongs to the project, the note relates to the task, and the annotation is anchored to the note. Delete `cache.sqlite` and every bit of that structure rebuilds by re-reading these four files — because the files *are* the truth.

---

## 11. Decision status — what's settled, provisional, and open

This section is the schema's own reconciled status list. Since the first draft, external review (see `IRIS_DOCUMENT_REVIEW_AND_VERDICT.md`) resolved several items into Accepted ADRs; those are marked here so this list agrees with `DECISION_LOG.md` rather than drifting from it.

**Resolved since first draft (Accepted — see ADRs):**
- **Archive is lifecycle, not a type** — ADR-016. `archive` removed from the type list; `lifecycle`/`archived_at` added (§4).
- **Relations stored one canonical direction; inverses derived** — ADR-017. Inverse registry defined (§5); `blocked-by` etc. are never stored.
- **Explicit project state machine + distillation trigger rules** — ADR-018 (§6, project type).
- **Lossless frontmatter editing** — ADR-019 (§1). Parser edits in place; body/comments/unknown fields preserved byte-for-byte.
- **Canonical vault = Markdown repo + blob store together** — ADR-020 (§8).
- **Canonical-write-wins transaction ordering** — ADR-021 (engine write path; not a file-format concern but governs how files are written).

**Provisional (defaulted so nothing's blocked; revisit before format freeze):**
1. **Node identity: ULID** — assumed for sortability. Trivial to switch to UUID before any files exist; expensive after. *Provisional.*
2. **Single-value `domain`** — one domain per node, for unambiguous coloring. Revisit only if cross-domain nodes prove common enough to need a coloring tie-breaker. *Provisional.*
3. **Folder-agnostic scanning** — the app scans the whole vault for `.md` files and trusts frontmatter; folders are for human convenience only. *Provisional* (could later enforce a convention, but not planned).
4. **Priority representation** — single `priority` enum for now; a two-axis Eisenhower view can be derived or added as a second field later. *Provisional.*
5. **Canonical recurrence representation** — the `{ mode, interval, rrule }` object in §6 is a provisional single shape underlying the three user-facing modes; reused by `reminder`. *Provisional.*

**Open (genuinely undecided; placeholder in use):**
6. **The `anchor` object's exact shape** (annotations) — depends on the unresolved CRDT library choice. Placeholder: a text-fragment string plus optional CRDT position. *Open.*
7. **Canvas canonical storage** — node-with-structured-body vs. separate file format vs. kind of `space` vs. plugin-defined (§6, Canvas note). Must be decided before Canvas implementation. *Open.*
8. **Distillation queue storage** — ADR-018 pins the *trigger* semantics; whether queue membership is a pure derivation (`distillation_level` + project `active` + link) or needs a persisted field remains open. *Open — leaning derived.*

---

## 12. What this unblocks

With this spec drafted, the rest of Phase 0 can proceed:

- **The parser** can be written against a defined format (read a `.md` file → a node in memory, and back → byte-identical, which the golden-file tests verify).
- **The core engine** can create/read/update/delete/relate nodes and commit them to git.
- **The cache rebuild** has a defined thing to rebuild *from*.
- **The integrity checker** has defined rules to check against (valid frontmatter, resolvable relations, known-or-gracefully-unknown types).

The recommended immediate next step after this spec is reviewed and adjusted: implement the parser + the golden-file round-trip test together, since they define and verify each other. That is the true first line of code in Iris.
