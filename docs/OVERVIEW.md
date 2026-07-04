# Iris — Product Overview

**Purpose of this document:** the single big-picture narrative of Iris — what it is, what it should become, the complete feature catalog with where each idea came from, and how each piece actually helps in practice. The other documents (`PRD.md`, `ARCHITECTURE.md`, `ROADMAP.md`, `DECISION_LOG.md`) are the structured, buildable versions of this. This one is the "why does any of this exist" document.

---

## 1. What Iris Is

**Iris is a personal second brain** — a comprehensive PKM (personal knowledge management) application built for one user, with no deadline pressure and no artificial scope limits, because breadth is the point rather than a risk to manage. The north star is simple to state and hard to build: a system to capture information, organize knowledge, and make it genuinely accessible and actionable across every device you use.

The closest shorthand: **a Notion + Obsidian hybrid, with Jira/Confluence-style task hierarchy, and a built-in AI agent.** But that shorthand undersells it, because none of those tools — individually or combined — share Iris's core primitive. Notion has databases and pages. Obsidian has markdown files and backlinks. Jira has issues and epics. Iris has a single unifying concept underneath everything: the **node**. A note, a task, an event, a project, a trading journal entry, a piece of music, a book you're reading, today's daily log — all of it is a node, all of it connects to everything else through the same typed-relation engine. That single decision is what lets one app be a notes tool, a task tracker, a calendar, a journal, and a knowledge graph without feeling like four apps stitched together.

A sharper one-line position than the tool-mashup shorthand: **Iris is a local-first personal knowledge system that turns captured information into active, working context the moment a project needs it.** The capture-and-file tools (Evernote, Notion) store; the linking tools (Obsidian, Logseq) connect; Iris's distinctive move is *activation* — resurfacing the right knowledge, tasks, and context exactly when work on something becomes live (see Guided Project Activation, below). That's the sentence to check new features against.

Iris also lets you genuinely **design your vault** — rich embedded content, handwritten ink, custom node types with their own schema, a freeform canvas for pre-structural thinking — so the vault looks and is structured the way you actually think, not a fixed template. Worth being precise about what that means, though: it's about content richness and structural customization, not vector graphics or prototyping. Iris borrows organizational *patterns* from design tools (Figma's live-linked templates, FigJam's freeform canvas) — it doesn't borrow their canvas-editing capabilities. See the "explicitly not" list below.

**Iris is explicitly not:**
- A cloud-based SaaS product being built for customers, with the growth and retention pressures that implies.
- A quick MVP with deliberately trimmed scope.
- Related to any employer or workplace — this is a personal tool, built on personal time, for a personal life.
- A vector graphics or prototyping design tool — "design your vault" means content richness and structural customization (§7, Vault Design & Customization), not shape editing or visual asset creation.

## 2. The Name — Why Iris

Three literal meanings of "iris," and each one maps onto something the product actually does — which is a big part of why the name fits rather than being decorative:

1. **The iris of the eye** — the part that controls the aperture, deciding how much light gets in and where focus lands. This is close to Iris's actual job: not showing you everything at once, but controlling how much of your accumulated knowledge is in view at any given moment — semantic zoom on the graph, a distillation queue that surfaces only what's relevant to an active project, focus mode dimming everything except the node in front of you. The product's core visual language (the graph view as a radial "iris" opening outward from a focused node, domain-based color coding) traces directly back to this meaning.

2. **Iris, the Greek messenger goddess** — she moves between the world of gods and the world of mortals, and is traditionally associated with the rainbow as the bridge between sky and earth. The functional parallel: Iris is meant to be the bridge between *raw captured information* and *usable, expressed knowledge* — exactly the gap the CODE framework's "distill" stage is meant to close, and exactly the gap every competing PKM tool leaves unbridged. A messenger doesn't just store messages; it moves them somewhere useful. That's a fair description of what a second brain should do, and a fair description of what most note-taking apps don't.

3. **The iris flower** — many distinct petals, one root system, a wide spectrum of color depending on species. A fitting metaphor for a single vault holding wildly different domains — a trading journal, music ideas, a reading pipeline, daily notes — that all connect through one underlying structure rather than living in separate apps.

None of this needs to be known to use the product day to day, but it's a genuinely useful check when a new feature idea shows up: does it help control what's in view (aperture)? Does it help bridge capture to usable output (messenger)? Does it strengthen the connections between different domains of one life (the shared root)? A feature that doesn't serve at least one of those is worth a second look before it gets built — it might be scope creep wearing a good idea's clothes.

## 3. More Than an App — Why "Product" Is the Right Word

Calling this "the Iris app" undersells it, and it's worth being precise about why. An app is a tool you open to perform a task. A **product** has a point of view: it makes opinionated choices, it has a philosophy that constrains what gets built and what gets rejected, and it has an identity someone other than its creator could eventually recognize, adopt, and extend.

Iris already has all three:

- **A point of view.** "Organize by actionability, not topic." "Distill just-in-time, not on a schedule." "AI is a multiplier, never a dependency." "The vault must outlive the app." These aren't feature specs — they're positions, and having positions is what separates a product from a utility.
- **A philosophy that constrains decisions, not just describes them.** Correctness-first isn't a slogan here — it's the actual reason certain features get deferred rather than shipped half-built (see the Engineering Philosophy section below). A checklist app doesn't need a philosophy because it rarely faces a hard tradeoff; a second brain meant to hold years of someone's actual thinking does, constantly.
- **An identity that can outlive its original context.** The plan to eventually open-source Iris under a GPL v3-style license, the attention already being paid to onboarding for people who aren't the original author, and the naming/visual language above are all signs of something built with a coherent identity — not a personal script that happens to have a UI wrapped around it.

The practical implication: every document in this set (`PRD.md`, `ARCHITECTURE.md`, `ROADMAP.md`, `DECISION_LOG.md`) should be read as *product* documents, not *app* documents. A PRD for an app asks "what does it do." A PRD for a product also asks "what does it refuse to do, and why does that refusal matter."

## 4. The Philosophical Foundation

Iris isn't organized around arbitrary feature ideas — it's grounded in two established PKM methodologies, and its single biggest differentiation is built by taking one of them more seriously than any existing tool does.

### The CODE Framework (Tiago Forte)

- **Capture** — save anything useful as you encounter it, at near-zero friction. If capture has friction, it doesn't happen, and the whole system starves.
- **Organize** — use PARA: organize by *actionability*, not topic. A note about running belongs in "Health" (an Area) if it's ongoing, or in "Marathon Training" (a Project) if it has a deadline — not in a generic "Fitness" folder.
- **Distill** — progressively summarize across multiple revisits: bold the key lines on one pass, highlight the best of those on a later pass, write a one-line summary in your own words on a final pass.
- **Express** — turn notes into output: tasks, decisions, essays, code. A note that never gets used again is digital hoarding with extra steps.

### PARA — Organizing by Actionability

- **Projects** — active work with a defined outcome and (usually) a deadline.
- **Areas** — ongoing responsibilities with no end date (health, finances, a hobby).
- **Resources** — reference material organized by topic, not urgency.
- **Archive** — inactive items pulled from any of the above three.

### Distillation — Where Every Existing Tool Fails, and Where Iris Wins

This is the crux of the whole project. Notion and Obsidian both have fine capture and adequate organization. Neither has any real mechanism for the "distill" stage — there's no UI in either tool whose job is "resurface old notes and force me to compress them into something usable." Notes go in, notes sit there, notes are never revisited unless you happen to remember they exist.

The trigger mechanism is the actual insight here, not just the summarization method: **Forte's rule is to distill "just in time," right before you actually need a note — not on a fixed calendar.** Time-based review (daily review habits, spaced-repetition schedules) burns attention on notes you may never need again, and misses notes that suddenly *do* matter because a project just went active. So Iris's distillation queue is triggered by **project activation** — the moment a project moves from "someday" to "active," every linked note that hasn't been distilled yet gets surfaced for processing. Relevance follows the work, not the calendar.

The method itself is progressive summarization, layered across multiple visits rather than attempted in one shot:
1. **First pass** — bold the handful of sentences that actually matter.
2. **Later revisit** — highlight the best of what you bolded.
3. **Final pass** — write a one-line summary in your own words at the top of the note.

Iris makes this trackable and low-friction: a `distillation_level` field per note (`raw → bolded → highlighted → summarized`) means the system always knows exactly how "cooked" a piece of knowledge is, and an optional LLM pass proposes the first bolding automatically so you're editing a draft instead of staring at a blank raw note.

### Engineering Philosophy — and Why It's Non-Negotiable

The CODE/PARA/distillation methodology explains *what* Iris organizes around. This explains *how* it's built — and why these are load-bearing parts of the product's actual purpose, not just nice-to-have engineering values:

- **Correctness-first, no shortcuts.** A second brain that occasionally loses or corrupts a note isn't a second brain — it's a liability with a friendly UI. The standard has to be "built right or not built yet," because the cost of a subtle data-integrity bug isn't a bad review, it's someone's actual accumulated thinking. Performance can be optimized later without anyone noticing; a corrupted vault can't be un-corrupted.
- **Local-first, no lock-in.** The whole premise of a second brain is trust — you're offloading memory and thinking to it. That trust is only rational if the underlying data is yours in a literal, portable sense: plain markdown, versioned in git, readable by any tool forever, never held hostage by a company staying in business or a subscription staying current. This is also the real reason AI is bring-your-own rather than bundled — the same trust logic applies to who gets to read your notes.
- **AI as a multiplier, never a dependency.** If a core workflow required AI to function, the product would become quietly dependent on whichever provider's API happens to be reachable that day, at whatever price and policy that provider chooses tomorrow. Keeping every core feature manually operable is what keeps Iris an actual second brain rather than a thin client for someone else's model.
- **Long-haul, no artificial scope limits.** Most PKM tools trim scope to ship faster, and the trimmed part is almost always distillation — the unglamorous, hard-to-demo stage. Refusing to trim scope here isn't scope creep; it's a direct consequence of taking the actual problem (notes that get captured but never used) seriously instead of building something that merely looks complete in a demo.

Every one of these constraints costs something — more engineering time, slower feature velocity, harder problems tackled earlier than they'd strictly need to be. That cost is accepted deliberately, because the alternative is a product that's fast to build and doesn't actually solve the problem it exists for.

## 5. What Iris Should Be

Iris should be the tool that makes "I captured this once and never saw it again" structurally impossible for anything that actually matters to an active project. It should feel less like a notes app and more like a second brain that grows with you — visibly, measurably, and without needing you to remember to tend to it.

Concretely, that means:
- **It should never lose anything.** Plain markdown, git-versioned, no proprietary format — the vault must outlive the app.
- **It should never require AI to function.** Every AI-assisted feature degrades gracefully to a manual equivalent when no AI is configured — AI is a multiplier, never a dependency.
- **It should never feel slow because of a feature you're not using.** Cost should scale with usage, not existence — a feature you never open should cost nothing at runtime.
- **It should never force a compromise between correctness and shipping.** Features are built to the standard they deserve, or they're deferred entirely — never shipped cheapened. Performance optimization can wait; correctness can't.
- **It should feel alive.** Analytics, graph visualization, and a growing vault should make the accumulation of knowledge visible and motivating, not invisible and easy to abandon.

## 6. What Iris Is Not — Competitive Distinctions

| Tool | What overlaps with Iris | Where it falls short |
|---|---|---|
| **Obsidian** | Markdown vault, backlinks, graph view | Search is keyword-only; no native tasks/calendar; graph view becomes unreadable past a few hundred nodes; no distillation mechanism at all. |
| **Notion** | Tables/databases, page hierarchy | No real graph or backlinking; weak linking model; no meaningful offline mode; any AI is generic chat, not grounded in your actual graph. |
| **Logseq** | Bullet-based linked notes, block references | Block-centric rather than typed-node-centric; no rich task hierarchy; distillation isn't modeled at all. |
| **NotebookLM** | AI synthesis over your own material | Read-and-synthesize over a fixed, uploaded source set — not a living, growing personal graph you write into; no tasks, calendar, or project structure. |
| **Jira / Linear / Confluence** | Task hierarchy (epics/stories/subtasks), long-form pages | No knowledge graph, no notes-as-first-class-citizen, no agent, and built for teams rather than a personal second brain. |

## 7. Complete Feature Catalog

Organized by function. For each area: what it does, where the idea comes from, and why it actually helps.

### The Substrate — Everything Is a Node

The one decision everything else depends on. A node has a type (note, task, event, project, area, resource, or a domain-specific type like a trading journal entry (archival is a lifecycle state, not a type — ADR-016)), typed frontmatter metadata, a markdown body, and typed relations to other nodes. *Inspired by* Roam/Logseq's "everything is a block" philosophy, generalized further — Iris generalizes it to "everything is a typed node," which is what lets task hierarchy, calendar events, and free-form notes coexist in one graph instead of three separate systems.

**Why it helps:** you never have to decide "is this a Notion thing or an Obsidian thing" — every piece of information, regardless of shape, lives in the same graph and can link to anything else in it.

**The object hierarchy, briefly** (full technical treatment in `ARCHITECTURE.md` §3): the **Vault** sits at the top — the whole git repository, the entire second brain, and there's exactly one active at a time. Everything inside it is a **Node**. Crucially, the PARA containers (Project, Area, Resource, Archive) are node *types* connected by relations, **not** folders that physically contain other nodes — a Task relates to its Project the same way any two nodes link, which is what lets one Task simultaneously belong to a Project *and* connect to the Resource that informed it, the Reading note it came from, and the Trading entry it affects. Inside a Project, the Epic → Story → Subtask breakdown is likewise just nodes joined by parent/child relations. There are only two deliberate exceptions to "everything is a node": **checklist items** (lightweight sub-content of a Task, promotable to a real Subtask on request) and **ungraduated canvas scratch content** (freeform items living inside a Canvas until you "graduate" them into real nodes). A **Space** isn't a container in this hierarchy at all — it's a saved lens that cuts *across* Projects and Domains. The mental model to hold: **what you see is often a tree, but what's stored is always a graph** — the tree is just the default lens.

### Capture — Killing Friction at the Point of Entry

- **Global-hotkey quick capture** — a floating input accessible from anywhere on desktop. *Inspired by* Obsidian's quick-switcher and Alfred/Raycast-style launchers. Near-zero friction at capture time is worth more than almost any other single feature, because the alternative is the idea never getting recorded at all.
- **Web clipper browser extension** — clips articles/highlights as linked nodes with reader-mode extraction. *Inspired by* Pocket/Instapaper's save-for-later model, merged into the graph instead of a separate silo. The bar to aim for is Evernote's clipper, which remains the acknowledged gold standard (full page, clean article, screenshot, or selection, with tag + destination chosen before saving) — Iris's should match that fidelity, but every clip lands as a real node in the graph rather than in a flat notebook.
- **Share-sheet integration (mobile)** — share any link or article from any app straight into an Iris inbox node.
- **Voice capture** — a dedicated widget, "Hey Iris, note that…", with on-device transcription so nothing leaves the device. *Inspired by* the frictionlessness of voice memos, without the downside of a note trapped in an audio file.
- **Wearable integration** — a watch complication for today's tasks, tap to capture a voice note from your wrist.
- **Camera-to-node OCR & document scanner** — point a phone camera at a whiteboard, handwritten note, book page, receipt, or business card; it OCRs and lands as a node with the source image attached. The scanner does proper document capture — edge detection, perspective correction, and multi-page — so a photographed page comes out square and legible rather than a skewed snapshot. *Inspired by* Evernote's scanner, which remains best-in-class at this; the difference is that in Iris the scan is a real **node** (linkable, distillable, surfaced in the graph), not a filed image in a notebook.
- **Vault-wide OCR indexing (Accepted — `DECISION_LOG.md` ADR-022)** — this is the capability that makes the above genuinely useful: *every* image, PDF, scan, and ink-note in the vault is OCR'd, and the extracted text is folded into search. A word that appears only inside a chart screenshot, a scanned contract, or a photo of a whiteboard becomes findable like any typed text — closing the one retrieval gap where Evernote was still clearly ahead. It runs **on-device by default** (private, no data leaves your machine, works with zero AI configured), with an *optional* cloud OCR provider available through the same bring-your-own abstraction as the AI layer for those who want maximum accuracy and accept sending image content out. Because OCR is compute-heavy, it runs batched in the background (gated on charging/WiFi where sensible) and never blocks capture; the extracted text is derived, rebuildable index data, never a mutation of your original file.
- **Email-to-node** — forward any email to a unique Iris address; it becomes a node with sender/subject captured as properties.
- **Built-in RSS reader** — subscribed articles land directly in the Iris inbox as nodes, not in a separate reader app.
- **Location tagging** — optional GPS auto-tag on capture, useful for "what was I thinking about when I was at that coffee shop last Tuesday."

**Why capture gets this much investment:** a second brain is only as good as what makes it in. Every one of these closes a specific gap where an idea would otherwise be lost — at a whiteboard, on a walk, mid-scroll, away from a keyboard.

### Organization — PARA and Beyond

- **PARA structure** as the top-level organizing containers (Projects/Areas/Resources/Archive).
- **Backlinks panel** on every node, Obsidian-style — see everything that references this node without having to remember where you mentioned it.
- **Anchored comments** — *inspired by* Figma's pinned comments — a note pinned to a specific range of text inside a node, not the whole node. Distinct from a backlink or a full typed relation: it's "future me, look at exactly this sentence," not "these two notes are related." Supports threaded replies (a real conversation between past-you and future-you) and a resolve/unresolve state, so handled comments disappear from the default view without being deleted. Particularly useful for marking a specific assumption in a trading thesis as "this turned out wrong" *without editing the original text* — the point of keeping the original is to see what you actually believed at the time. "Unresolved anchored comments in active projects" is a natural companion query to the distillation queue.
- **Tag hierarchy + saved filters ("smart folders")** — filters that auto-populate, e.g. "all #trading tasks with no due date."
- **Saved search as a live node** — a query that lives in the graph like a real node, always showing current results, and is itself linkable. *Inspired by* Notion's linked databases, but generalized so a query is a first-class graph citizen.
- **Spaces** — *inspired by* Arc's browser Spaces — a saved context, not a container. A Space bundles pinned nodes/searches, an active domain/tag filter, a default view and its state, which panels are visible, and a theme accent, so switching between "Trading mode" and "Iris dev mode" reconfigures the whole UI in one click instead of manually re-filtering every panel. Deliberately separate from PARA — a Space can span multiple projects or cut across domains, where a Project or Area is a structural container for the knowledge itself. Starting a focus session against a project can auto-switch to that project's Space.
- **Spaced repetition** — resurface selected nodes on a schedule, distinct from the distillation queue; this is specifically for material you want to memorize, not material you're processing.
- **Daily notes with auto-agenda** — auto-created each morning, pulling in today's tasks, scheduled events, and (optionally) 3 AI-surfaced nodes relevant to current work. *Inspired by* the "daily note" pattern common to Roam/Obsidian/Logseq, extended with automatic relevant-context surfacing.

### Vault Design & Customization — Making the Vault Actually Yours

This is the "design your vault" idea made concrete: not visual design tooling, but real content richness and structural flexibility, so the vault ends up shaped like your own thinking rather than a fixed template.

- **Rich inline content** — images, an embedded PDF viewer, a scrubbable inline audio player (for voice notes and music captures — not just a download link), syntax-highlighted code blocks, LaTeX/math rendering, and video embeds, all rendered directly inside a node's body. The authoring format underneath stays plain markdown/embed syntax throughout, so richness in the rendered view never costs portability or git-diffability.
- **Handwritten notes as a real node type** — on iPad/Android tablet with a stylus, a genuine ink-canvas node stores your raw strokes as the canonical content *and* runs OCR alongside it for search, so you get both the personal feel of actual handwriting and full searchability — two representations of one node, not a lossy conversion. A meaningfully richer take than plain camera-to-node OCR (see Capture, above), which only ever produces the text.
- **Custom node types** — beyond the built-in types, define your own via the plugin API: a "Recipe" type with `prep_time`/`ingredients`/`cuisine`, a "Workout" type with `exercises`/`sets`/`reps`. Your vault's schema becomes genuinely yours, not limited to what shipped in v1.
- **Components & Instances (live-linked templates — Tier 2 of three)** — *inspired by* Figma's live-linked components — a custom (or even built-in) node type can be defined once as a Component, with every node created from it an Instance that inherits that schema while still allowing local overrides. Editing the Component later propagates the change to every Instance that hasn't overridden that specific field — turning "I need a new field on every trading journal entry" from a manual migration into a single template edit. This is deliberately *different* from a plain node template (Tier 1, below), which is a one-time copy with no lasting link — here the link persists and changes flow down. See `DECISION_LOG.md` ADR-026 for the full three-tier template model.
- **Starter systems (whole configured workflows — Tier 3)** — the most powerful sense of "template": not a page but an entire working setup for a domain (software project, research paper, trading journal, reading pipeline, job search, fitness plan…). A starter system can bundle the node types, relations, views, statuses, activation behavior, dashboards, default queries, capture destinations, and review workflows for that domain, installed as a unit — so you adopt a sophisticated workflow without building it from scratch. First-party starter systems and community-authored ones share one declarative mechanism. This is the adoption lever the missing-capabilities review specifically called out, and it's built on top of custom types + Components (Plugin phase and beyond).
- **Controlled vocabularies** — domains, priority levels, and workflow states are defined once and referenced everywhere, rather than free-typed strings scattered across nodes. Rename a domain once, every node referencing it updates — preventing `trading`, `Trading`, and `#trading` from silently fragmenting into three untethered tags over time. Free-form tags remain available alongside for genuinely open-ended labeling.
- **Canvas mode** — *inspired by* FigJam — a freeform, infinite 2D surface for thinking before anything deserves to be a structured node: drop images, sticky-note fragments, and references, arrange them spatially, cluster by proximity, no schema required. Canvas content is disposable by default and stays out of search/distillation/analytics until deliberately **graduated** into a real typed node — it's a genuine pre-structural thinking space, not a disguised inbox you're obligated to process.
- **Named checkpoints and branching** — since the vault is a real git repository underneath, "save a checkpoint here" and "try a different reorganization without committing to it" are git tags and git branches, surfaced in plain language rather than requiring you to think in git directly. Anything done through the UI stays inspectable and recoverable with any standard git tool.
- **Themes, appearance, and per-Space visual identity** — dark mode, custom accent colors, typography, and (via Spaces, above) a distinct visual identity per context, so switching contexts is visually obvious, not just functionally different.

### Structure — Jira/Confluence-Flavored, for a Personal Graph

- **Spaces/projects** as top-level containers.
- **Full nested task hierarchy** — epics → stories → subtasks → checklist items, borrowed directly from Jira, because personal projects genuinely benefit from the same decomposition team projects do. Checklist items are lightweight inline items rather than full nodes, with a one-click promotion to a proper subtask when something grows beyond a simple checkbox.
- **Custom workflow states per project** rather than one global status list.
- **Node templates (Tier 1)** for recurring note/page structures — a daily-note layout, a meeting-note skeleton, a book-review structure. The simple kind: pick one, get a pre-filled node. It's a one-time **copy** — later edits to the template don't touch notes you already made from it (that's Tier 2 Components, above, if you want a lasting link). See ADR-026 for how the three template tiers differ.
- **Labels/components** as a second categorization axis alongside tags — *inspired by* Jira's label + component split, useful when a single tag taxonomy isn't expressive enough.
- **Task dependencies** — canonical `blocks` / `depends-on` relations (the inverse `blocked-by` is derived, not stored — ADR-017), reusing the same relation engine as everything else. A blocked task is visually distinct and, by default, excluded from the Today view until its blocker clears. A dependency conflict (a blocking task due *after* the task it blocks) is flagged automatically.
- **Task views as filtered lenses over the same task nodes**, not separate storage:
  - **Inbox** — freshly captured tasks with no project, no date, no priority; the raw triage queue.
  - **Today** — tasks *scheduled* for today, plus overdue tasks surfaced but visually distinct from what you actually planned. `scheduled_date` (when you intend to work on it) and `due_date` (when it's actually due) are deliberately separate fields — collapsing them, which most todo apps do, loses real information.
  - **Upcoming** — a rolling 7/14/30-day lookahead, grouped by day, drag-to-reschedule.
  - **Someday/Maybe** — the GTD holding area for tasks you're not ready to commit to; no date, no schedule, periodically resurfaced during weekly review for triage.
  - **Logbook** — a chronological, automatically-maintained record of every completed task, kept out of active views but always queryable.
- **Eisenhower matrix triage** — a 2×2 urgent-×-important view for genuine prioritization, not just a linear high/medium/low field.
- **Recurrence, modeled properly** — three distinct behaviors, not one: **fixed** (next instance due a fixed interval after the *original* due date, regardless of completion time — e.g. rent), **flexible** (next instance due an interval after *actual completion* — e.g. "review the garden 7 days after I last did it"), and **custom RRULE** for irregular patterns ("every second Tuesday"). Most apps only implement one of these and get it wrong for the other use cases.
- **Natural-language capture** — typing `"Review architecture doc tomorrow at 3pm high priority #iris"` parses into title, scheduled time, priority, and tag inline, entirely on-device, no API call required.
- **Smart lists as saved queries** — "high priority with no due date" (the danger zone), "quick wins" (≤1 pomodoro estimate, no blockers), "waiting for" (blocked by something external, not another task), "stale tasks in active projects" (untouched 30+ days — the task-side analogue of the distillation queue's "note graveyard" problem).
- **Time estimates vs. actuals** — every task can carry an `estimated_pomodoros` value and an auto-filled `actual_pomodoros` from completed focus sessions, building a personal velocity model over time ("your coding tasks consistently run 2x your estimate").

### Planning — Sprints, Timeline, and Calendar as One Stack

Three surfaces, three time horizons, one underlying data model — this isn't three features, it's one planning stack viewed at different zoom levels:

```text
Sprints   — what am I committing to this week/fortnight (short horizon, capacity-bounded)
Timeline  — how do my projects/epics fit together over months (medium horizon, dependency-aware)
Calendar  — what is actually happening on a specific day (ground-level, time-of-day precision)
```

- **Calendar view** — day/week/month/rolling-4-week views. Drag a task onto a time slot to schedule it; drag an existing event to reschedule; resize a block to adjust duration. **Time-blocking** lets you reserve a slot for a specific node/project as protected focus time, distinct from a scheduled task or an event.
- **Bidirectional Google/Apple Calendar sync** — external events become Iris event nodes and stay in sync in both directions; edit in Iris, it reflects in your real calendar, and vice versa. You don't have to choose between Iris and your existing calendar — Iris absorbs it, and every synced event is linkable to the notes/tasks it relates to (an agenda node, action items extracted afterward) in a way Google Calendar itself can't offer.
- **Sprint planning** — a time-boxed commitment ("these specific tasks, this specific fortnight") pulled from a project's backlog. Capacity is *computed*, not manually entered — available hours come from the calendar (working hours minus existing events minus already-placed time blocks) — so a sprint can't be over-committed without Iris flagging it before you start, not halfway through.
- **Burndown and velocity** — tasks remaining vs. days remaining, updated daily; velocity tracked in **pomodoros**, not abstract story points, because personal work is better measured in real logged time than a relative complexity score that needs a team to calibrate.
- **Sprint review/retrospective** — committed vs. completed, incomplete tasks triaged (carry over / back to backlog / won't-do), and a retrospective note written as a real node in the graph — linked to the sprint, queryable later ("have I hit this same blocker before?").
- **Timeline/Gantt view** — projects and epics rendered as horizontal bars across weeks/months, milestones as markers, dependencies as arrows between bars, **today** as a vertical line through everything. **Critical path highlighting** shows the dependency chain that determines the earliest possible completion date — the difference from a generic Gantt chart is that this one is grounded in your actual available time from the calendar layer, so it can tell you whether a deadline is realistic, not just whether it's drawn on the chart.
- **Weekly review flow** — a guided, templated close-the-loop process: collect (sweep the inbox), process (triage every item), review (scan active projects for stalled tasks), plan (pick next sprint against real capacity), schedule (drag tasks onto specific days), close (auto-generate a weekly note summarizing what moved). This is the mechanism that keeps the whole planning stack honest week over week rather than becoming aspirational.

### Views — Many Windows Onto the Same Data

All of the following read and write the exact same underlying nodes — switching views never means switching data models:

- **Table** — spreadsheet-style, filter/sort/group by any property on any node type.
- **Graph** — force-directed link explorer, with semantic zoom (see Visualization below).
- **Kanban board** — drag-and-drop across status columns, with swimlanes by project or tag.
- **Calendar / Timeline** — pan across days/weeks/months, drag to reschedule, filter by tag or project.
- **Document / Page** — Confluence-style long-form view, with nested page hierarchy modeled as parent-child relations.
- **Relationship matrix** — a node × node grid where cells show relation type; sparse cells surface *missing* connections, which is genuinely hard to see in a pure graph view.
- **Gallery / moodboard** — an image-forward grid of cards for nodes with visual attachments (music references, trading chart screenshots, a moodboard node type) — the visual-browsing view none of the above quite cover, since they're all text/data-forward.
- **Canvas** — the freeform, pre-structural surface described under Vault Design & Customization above; the one view where content doesn't need to be a typed node at all until you decide it should be.

**Why multiple views matter:** the same knowledge graph looks like a task board when you're executing, a calendar when you're scheduling, and a graph when you're exploring connections — you shouldn't need three separate tools for three separate mental modes.

### Search — Four Layers, Not One

- **Lexical** — traditional full-text search.
- **Semantic / vector** — embedding-based similarity search, so "find things related to this idea" works even without exact keyword overlap.
- **Structural / graph-topology** — search by relationship shape, not just content (e.g., "notes two hops from this project that aren't yet linked to it").
- **Temporal** — search by when things happened or were captured, not just what they say.

*Inspired by* the observation that Obsidian's search is keyword-only and Notion's is barely adequate — no mainstream PKM tool combines all four layers, and each one answers a genuinely different question ("what did I write," "what's related in meaning," "what's related in structure," "what happened when").

**Important design note:** only the semantic layer strictly needs a model (and even that can be a small local embedding model, not a full LLM) — lexical, structural, and temporal search are entirely AI-free, so three of the four search layers work identically with zero AI configured.

**How the four layers combine into one ranked list:** the layers score results on incompatible scales (an unbounded lexical score can't be averaged against a 0–1 semantic similarity), so Iris uses **Reciprocal Rank Fusion** — it ignores the raw scores and combines results by their *rank position* in each layer's list. The practical effect is that a result which ranks well *consistently across several signals* beats a result that's the single best hit on just one axis — which is almost always the one you actually wanted, since relevance corroborated from multiple independent angles is stronger evidence than one lucky strong match. Recency (the temporal signal) is applied as a gentle *boost* on top of the fused ranking rather than as a co-equal factor, so "edited recently" nudges ordering among similarly-relevant results without letting a fresh-but-mediocre note outrank a highly relevant older one. Full mechanism in `ARCHITECTURE.md` §15.

**Search reaches inside attachments, too (ADR-022).** Because every image, PDF, scan, and ink-note is OCR'd vault-wide, the lexical layer (and the semantic layer, if embeddings are configured) searches the text *inside* those files, not just typed note bodies — the same capability that keeps long-time Evernote users on that platform, now native to Iris. And when a search term is found inside an image, Iris highlights it *on the image* at retrieval time (*inspired by* Evernote's in-image highlighting), so you can see at a glance where in a scanned page or whiteboard photo the match actually is.

**Results explain why they surfaced (ADR-024).** Iris doesn't just hand you a ranked list to trust — each result can show a compact "why this?" rationale drawn from the signals that actually ranked it: matched terms, link/project relationship, backlink relevance, distillation level, recent use, active-project relevance, OCR/attachment source, semantic and temporal relevance. For example: *"Linked to Project Iris · contains 'lossless YAML' · referenced by 2 active tasks."* Retrieval becomes something you can *see the reasoning behind* rather than an opaque black box — the same honest-signals stance that led Iris to prefer a transparent diagnostic panel over a single mystery score.

### Productivity Mechanics

- **Sprints/cycles with velocity/burndown** — genuinely optional even long-term, borrowed from Jira for the projects where that rhythm actually helps.
- **Time-blocking directly on the calendar** — drag tasks onto time slots rather than keeping a disconnected to-do list.
- **Pomodoro/focus sessions tied to a specific node** — actual time logged against that node vs. estimated time.
- **Habit tracker with streaks** — recurring nodes, doubling as a practice log for recurring activities (e.g. an instrument).
- **Daily/weekly review templates** — auto-generated notes pulling in completed tasks, new links created, and upcoming deadlines.
- **Reminders — manual, user-authored, never automatic.** A reminder is its own lightweight node: you write the text, you set the time (a specific moment, or a recurrence like "every Monday at 9am" reusing the same recurrence model as recurring tasks), and optionally point it at any node. It can stand alone ("call the broker at 9am") or attach to something ("remind me about *this* note at 3pm"). The deliberate design choice: Iris **never** auto-generates reminders or fires a notification you didn't ask for. Task due dates, event start times, sprint deadlines, and streak-at-risk all stay as *in-app indicators* (they show up in Today, the burndown, the streak counter) — but none of them push a notification on their own. If you want to be nudged about a due task, you create a reminder for it (one tap from the task pre-fills its title as an editable starting point). This is the same "the app doesn't decide when to interrupt you" principle that led to rejecting gamification — see `DECISION_LOG.md` ADR-015. Delivery is OS-native local scheduling, so it works offline and costs nothing in battery when idle.
- **Notifications** exist *only* as the delivery mechanism for the manual reminders above — actionable inline (snooze/dismiss/open, deep-linking to the target node if there is one), respecting both OS Do Not Disturb and an Iris-specific quiet-hours setting.

### Domain Modules — Life-Specific, Not Generic

These exist because a truly personal second brain needs first-class support for the specific things a life actually contains, not just generic "notes":

- **Trading journal** — thesis, entry/exit, P&L, linked notes, and performance analytics as a native node type rather than a bolted-on template.
- **Music capture** — audio snippets, riffs, BPM/key/mood tags, and song-project nodes that tie ideas together over time.
- **Reading pipeline** — Inbox → Reading → Read → Extracted notes, a full Pocket replacement that lives inside the same graph instead of a separate app.
- **Kindle highlights sync** — book annotations land automatically as linked nodes, grouped by book.

### Integrations — Pulling the Live World In

Distinct from the one-time "Import from everywhere" (Meta/System, below) — these are *live*, ongoing connections to services you already use, so the domain modules above have real data to work with rather than requiring manual re-entry:

- **Google/Apple Calendar, bidirectional** — the backbone of the Planning stack above; not an import, a continuously synced adapter.
- **GitHub** — repos, commits, and issues become linkable nodes; a coding-project node in Iris genuinely knows its own git history rather than just linking out to it.
- **Spotify/Apple Music** — the song playing at capture time gets auto-linked to the note, useful mood/context metadata for the music-capture domain module.
- **Robinhood / brokerage data** — auto-create trading journal entries from real brokerage activity via the plugin API, rather than manually transcribing trades after the fact.
- **RSS** — subscribed feeds land as inbox nodes directly, folding read-it-later into the graph instead of a separate reader app.
- **Email-to-node** — forward any email to a unique Iris address; it becomes a node with sender/subject captured as properties.

### Distillation — The Differentiator, Restated as Features

Covered philosophically in §2; here's the concrete feature set:

- Distillation queue **auto-populates** the moment a project activates.
- An optional **LLM first pass** proposes what to bold — the user confirms or edits, never starts from a blank note.
- **Per-note `distillation_level`** metadata (`raw → bolded → highlighted → summarized`), queryable directly: "show me all undistilled notes linked to project X."
- Distillation stays **human-in-the-loop by design** — the system never silently rewrites your notes; it proposes, you decide.

### Guided Project Activation — the Differentiator You Can See

Activating a project isn't a status change — it's the moment Iris **assembles a focused working environment** for that work (`ARCHITECTURE.md` §11.5, `DECISION_LOG.md` ADR-023). This is the single most important test of whether Iris earns its place: *when you activate a project, does Iris pull together the knowledge, actions, context, and next steps you need better than opening five separate tools?* One activation view brings together, all from data already in your graph:

- the project's linked raw/undistilled notes (the distillation queue),
- unresolved decisions and open questions attached to it,
- its dependent and blocked tasks,
- related resources and reference material,
- upcoming calendar constraints touching the project,
- recently-added related material,
- and a recommended set of next actions to start on.

Crucially, this works with **zero AI configured** — Iris surfaces and groups everything and you choose where to begin. With AI, the "recommended starting set" additionally becomes a suggested *ordering* (tackle the unblocked, on-critical-path work first). AI sharpens the recommendation; it never gates the environment. This is the whole "resurface the right knowledge exactly when work becomes active" thesis made tangible — the thing no notebook-and-folders tool does.

### The AI Agent — Four Capability Levels

Designed as a first-class citizen from day one, but never a dependency (see the graceful-degradation table below):

1. **On-demand query** — select nodes, ask a question, the agent reads that context and responds, grounded in your actual graph rather than generic knowledge.
2. **Auto-linking suggestions** — the agent scans new notes and proposes links to existing nodes you didn't manually connect.
3. **Agentic actions** — "turn this note into 5 tasks," "summarize everything tagged #trading from this week," "reschedule my overdue tasks."
4. **Standing nightly digest** — runs on a schedule, surfacing stale tasks, newly-connectable notes, upcoming event conflicts, and cross-domain insight suggestions.

Additional AI features layered on top: an inline writing assistant grounded in your graph (not generic knowledge), contradiction detection (flags when a new note conflicts with something written earlier), knowledge-gap detection ("you've linked this concept 12 times but never written a node explaining it"), auto-summarization of long notes, meeting-notes-to-action-items extraction, an "idea incubator" that periodically pairs two half-baked notes and asks whether they connect, and — over time — a personal writing-style model that can draft in your own voice.

**Graceful degradation is a hard requirement, not a nice-to-have.** Every AI feature has a defined manual fallback:

| AI feature | With AI | Without AI |
|---|---|---|
| Distillation first pass | LLM proposes bolding | Blank note, bolded manually |
| Auto-linking | Agent suggests related nodes | Manual link creation |
| Auto-tagging | Agent suggests tags | Manual tagging |
| Contradiction detection | Agent flags conflicts | User notices manually (or not) |
| Knowledge gap detection | Agent surfaces gaps | Not available |
| Nightly digest | Agent generates summary | Not available |
| Semantic search | Vector embeddings | Lexical + structural + temporal still work (3 of 4 layers) |
| Writing assistant | Agent drafts in your voice | Standard editor, no suggestions |

The provider layer also separates **completion/chat models** (Claude, GPT, Gemini, Deepseek — used for reasoning tasks) from **embedding models** (used for semantic search and similarity), since these have different cost/privacy profiles and a user might reasonably run embeddings locally while using a hosted model for heavier reasoning.

### Visualization — Making the Graph Usable at Scale

The standard failure mode in existing tools (Obsidian especially) is a graph view that's beautiful at 50 nodes and unreadable at 500. Iris's answer is **semantic zoom** rather than a better force-directed layout:

- **Maximum zoom out** — no individual nodes at all; **domain clusters** appear as shapes, sized by node count and colored by domain. A map of your mind from altitude.
- **Zoom in one level** — clusters resolve into their most-connected **hub nodes** — the concepts everything else links through.
- **Zoom in further** — individual nodes appear with their immediate neighborhoods.
- **Full zoom** — a single node and its direct relations.

Additional visualization modes: **activity heatmap** (a GitHub/LeetCode-style contribution grid, but domain-colored rather than single-intensity — each day's cell is colored by whichever domain dominated that day's activity, so a year view becomes a visual autobiography: "obsessed with #trading in March, shifted to the #music project in May." Clicking any cell opens that day's daily note directly, making the heatmap a navigation tool, not just a display. Tracks multiple signals — nodes created, links formed, tasks completed, pomodoros finished, notes distilled — without collapsing them into one number), **temporal graph replay** (watch the knowledge graph grow over time as an animation — which clusters formed first, which ideas connected to which), the **relationship matrix** described above, and **focus-mode graph** (zoom into one node, see only 1st/2nd-degree connections, everything else dimmed).

**Streaks, kept honest rather than punishing:** a plain streak counter (current + longest) for daily capture, distillation, and pomodoro completion — deliberately *not* attached to any point/reward system (see the gamification exclusion under Meta/System, below). A one-freeze-per-month allowance and a short grace window mean a single missed day from travel or illness doesn't erase months of a genuine habit — the goal is an honest reflection of consistency, not a mechanic that punishes real life.

**Why this matters:** zoom level becomes a semantic filter, not just a visual resize — the graph stays useful whether you have 50 nodes or 5,000.

### Writing & Publishing

- **Built-in long-form editor** — a proper distraction-free writing mode for when a node grows into an essay or document, with word count, reading time, and an outline sidebar.
- **Publish to web** — one-click publish any node or nested document tree as a public read-only page, a self-hosted personal wiki/blog.
- **Digital garden mode** — a browsable public-facing subset of the graph, in the style popular in the PKM community.
- **Version diff view** — side-by-side comparison of any two versions of a node from git history, so you can see exactly how your thinking on a topic evolved.
- **Flow connections** — *inspired by* Figma's prototyping arrows — a typed relation defining explicit reading/assembly order between distilled notes, distinct from a general semantic link. Used when composing several notes into an essay or decision document where sequence genuinely matters, not just relatedness.

### Focus & Deep Work

- **Focus sessions tied to a node, structured as Pomodoro cycles** — declare what you're working on, run a configurable work/break timer (25/5 default, fully adjustable), and log time directly against that node's properties: `pomodoros_completed`, `total_focused_time`, `estimated_vs_actual`. An interrupted pomodoro can be voided per classic Pomodoro methodology while still logging the interruption as a signal, and completed-count feeds the activity heatmap, the daily note, and a home-screen/desktop widget so today's count is glanceable without opening the app.
- **Distraction filtering** — while a session is active, optionally surface only nodes related to the declared project.
- **Daily note with auto-agenda** — see Organization above.
- **End-of-day review prompt** — the agent summarizes what was captured today, what tasks moved, and asks one reflection question to add to the daily note.

### Physical World / Ambient

- **Voice-first capture, opt-in "always listening" mode** — local processing only, triggered by a wake phrase.
- **Wearable integration** — watch complication for tasks, tap-to-capture voice notes.
- **Camera capture with OCR** — see Capture above.
- **Location tagging** — see Capture above.

### Widgets

Desktop and mobile widgets for today's tasks, recent nodes, quick capture, and the active Pomodoro timer/today's completed count, plus a mini live-graph widget showing recently active nodes. **Widgets are treated as an early architectural validator** — if the local cache can power a widget query fast enough, the underlying data layer is solid enough to power everything else built on top of it. Platform constraints differ meaningfully (WidgetKit/Glance are read-mostly, periodic-refresh, sandboxed; desktop widgets can talk to the local process directly) — see `ARCHITECTURE.md` §9.

### Meta / System

- **Personal knowledge analytics dashboard** — not vanity metrics, but signals that actually mean something: *knowledge velocity* (nodes created + linked per week), *domain balance* (is your focus aligned with your declared priorities), *connectivity score per node* (your most-linked, load-bearing concepts), *idea half-life* (how long nodes stay active before going dormant), and *cross-domain link ratio* (high = integrative thinking, low = siloed thinking).
- **Vault health — a diagnostic panel first, a score second.** The *primary* surface is a transparent diagnostic panel that lists concrete, actionable signals: undistilled notes attached to active projects, broken relations, orphaned annotations, unindexed nodes, stale active projects, missing attachments, recent retrieval/use, and any sync or integrity problems. These are specific and fixable, not a number to optimize.
- **The "Iris Score"** — a single composite health indicator derived from those signals (graph connectivity, distillation depth — what fraction of notes reach `summarized` — and retrieval rate — how often captured notes actually get resurfaced and used). It is deliberately *secondary* to the diagnostic panel, and kept honest by two rules: its formula is **transparent** (you can see exactly what feeds it and how it's weighted), and it is **never framed as a measure of personal productivity or worth** — only of vault health. A large vault of disconnected, never-distilled, never-retrieved notes scores *low*; a smaller well-connected, well-distilled one scores high. The caution behind keeping it secondary: any single number can quietly become a thing users game (you optimize what you measure), which is exactly the pull ADR-009 rejects — so the score exists as an at-a-glance summary of the panel beneath it, not as a target in its own right. Its exact formula and weightings are an open design detail, not a committed calculation.
- **Deliberately no gamification layer.** XP, levels, achievements, and skill trees were explored seriously — including a redesigned "outcome-based" version rewarding retrieval and traversal rather than raw actions, specifically to resist being gamed. All of it was rejected. Gamification solves a motivation problem Iris shouldn't have by design: the distillation queue's just-in-time trigger and the honest signals above (heatmap, streaks, this dashboard) are meant to make using Iris its own reward. A points system is also, definitionally, meaningless outside Iris — the opposite of "the vault must outlive the app." See `DECISION_LOG.md` ADR-009 for the full reasoning; this is a standing constraint on future feature proposals, not a one-time call.
- **Plugin/extension API** — the single most strategically important meta feature. Lets new node types, custom views, and custom agent actions be added without forking the core, exactly how Obsidian's simple-core-plus-plugins model made it so extensible. Future-proofs Iris against changing personal needs without a rewrite.
- **Dev Mode** — *inspired by* Figma's Inspect/Dev Mode — a toggle on any node showing its raw frontmatter/YAML and relation list instead of the rendered markdown. Useful for debugging your own vault, understanding exactly what a plugin or the AI actually wrote, or learning the file format well enough to trust it completely.
- **Conflict-safe sync** — if the same note gets edited on two devices before they sync (the classic "edited on the flight, also edited at home" case), Iris never silently picks a winner and discards the other edit. The losing version is preserved as a linked, clearly-flagged conflict copy until you reconcile it — the same underlying problem as a GitHub merge conflict, resolved with two clean side-by-side versions instead of inline merge markers.
- **Trash & undo — layered data safety.** Deleting a node doesn't erase it; it moves to Trash (out of normal views, but recoverable in one click) for a retention window before becoming git-history-only — which handles the most common "oops, I deleted that" moment far more directly than undo alone. On top of that: an in-session undo stack (Cmd/Ctrl+Z) for immediate edit mistakes, and full git history as the permanent backstop for anything older. Three layers for three different timescales of "I want that back." Full detail in `ARCHITECTURE.md` §5 (Data Integrity & Recovery).
- **Malformed-file resilience** — because the vault is hand-editable outside Iris, a broken YAML frontmatter block is inevitable eventually. Iris quarantines a file it can't parse (surfaced in a "Needs Attention" list with the actual error, raw content viewable via Dev Mode) rather than crashing or silently dropping it — one broken file can only ever affect itself, never the whole vault.
- **Restore from backup** — a first-run option alongside create/open: point Iris at a git remote, and it clones, rebuilds the cache, and runs the integrity checker before declaring the restore complete.
- **Zero telemetry, ever** — Iris has no phone-home, no server-side usage analytics, nothing transmitted in the background. A second brain holds your rawest thinking; nothing about how you use it leaves your devices. If crash reporting is ever added it's strictly opt-in and scrubbed to stack traces with no vault content. See `DECISION_LOG.md` ADR-013 — exactly the property worth being able to point to before an open-source release.
- **Accessibility as a standing requirement** — not a late polish pass. Full keyboard navigation across every view including graph and canvas, screen-reader labels on custom-drawn components, a colorblind-safe default palette for the domain-colored heatmap and graph clusters (reassignable — this matters because so much of Iris's information design is color-coded), OS font-scaling respected, and a reduced-motion setting that disables the temporal-replay animation and canvas transitions.
- **Themes and appearance** — dark mode, custom accents, typography. A shallow feature, but a deeply motivating one for daily use.
- **Onboarding graph** — a starter set of Tier-1 node templates (daily-note, project, reading-list) plus a small example graph, so a fresh vault doesn't feel like an empty void. Over time this grows into first-party Tier-3 starter systems (ADR-026) for common domains.
- **Import from everywhere — an onboarding-critical capability, not a someday nicety (ADR-025).** Nobody abandons years of accumulated notes to start empty, so moving an existing vault *in* — with links, attachments, tags, timestamps, task states, and hierarchy preserved — is a precondition for adoption, offered at first-run alongside create/open/restore. Sources: Obsidian vaults (wikilinks → Iris relations), plain Markdown folders (the safest universal path), Notion export (databases → node types, properties → frontmatter), Evernote ENEX (notes + attachments + timestamps + tags — pairs with the OCR work in ADR-022), Roam JSON, task CSV, calendar ICS, browser bookmarks. Because Iris's own format is plain Markdown + YAML, the Markdown-family imports are high-fidelity; the first importers built are plain-Markdown and Obsidian (least mapping risk), with ENEX/Notion following. Iris meets existing knowledge where it lives instead of demanding a fresh start.
- **Backup to anywhere** — beyond git, scheduled export to iCloud/Google Drive/Dropbox as a zipped vault, set and forget.

## 8. Use Cases — How This Actually Helps, Day to Day

**Morning planning.** A daily note auto-generates with today's tasks, scheduled events, and a few AI-surfaced nodes relevant to whatever's currently active — instead of manually assembling a to-do list from three different apps.

**A project goes active.** You move "Redesign the home studio" from Areas to an active Project. Every raw, undistilled note you've captured that touches that project — half-formed ideas about acoustic treatment, a voice memo about speaker placement, a clipped article on room modes — surfaces in the distillation queue. You process a few, the LLM has already proposed what to bold, you confirm or adjust. Nothing sat there for months waiting for a review habit that never happened.

**Reading something on your phone.** You share an article straight into the Iris inbox from any app. It lands as a node, gets read later, and when you extract a highlight it becomes a linked node — connected to whatever else in your graph it relates to, not stranded in a separate read-it-later app.

**A trading idea.** A thesis note becomes a trading journal entry — entry/exit, P&L, and the reasoning notes that led to it, all linked, all queryable later ("what was my thesis the last three times I traded this setup").

**Standing at a whiteboard.** A photo of a sketched-out architecture diagram becomes a node with OCR'd text and the image attached, without manually retyping anything.

**Reviewing months later.** The knowledge graph's temporal replay shows how a cluster of ideas around a topic formed and connected over time — useful for seeing how your thinking on something actually evolved, not just what you currently believe.

**Zero AI configured.** Every one of the above still mostly works — capture, PARA organization, task hierarchy, calendar, lexical/structural/temporal search, manual distillation — because AI is additive by construction, never load-bearing.

## 9. A Closing Note on Scope

This feature set is, honestly, larger than what most funded startups ship in three years. That's acceptable *because* this is a long-haul personal project with no external deadline — but it's also exactly why the roadmap (`ROADMAP.md`) exists as a forcing function: Phase 0 and Phase 1 are what unblock everything else, and this document is meant to be the reference you return to for "why," not a thing you keep re-drafting instead of building against.
