import SwiftUI
import IrisCore
import IrisMarkdown

/// The shared detail view every node type opens into (`design/navigation.md`
/// §3) — one editor shell, not four per-type designs, matching the single
/// `Node` model in `iris-core`.
///
/// The body has two modes, switched by the header control or ⌘E and
/// remembered across notes: rendered **Preview** (default — `MarkdownBodyView`,
/// Tier A reading: headings, lists, checklists, quotes, code, tables,
/// wikilinks, highlight; see its doc for what's not built) and a plain-text
/// **Edit** mode (`TextEditor`) that autosaves through `updateNodeWithBody` —
/// one engine mutation (undoable, one git commit) per save, after 2s idle, on
/// navigating away, or ⌘S. There is no WYSIWYG editing yet. Tags are
/// edited inline in the header (add field, × per chip), each an undoable update.
struct NodeEditorView: View {
    let engine: FfiEngine
    let relPath: String
    /// Opens another node (a followed `[[wikilink]]`). The main window routes
    /// it through `AppShell`, a secondary window opens a new window; nil
    /// disables wikilink navigation.
    var onOpenNode: ((CachedNode) -> Void)?

    /// "preview" (rendered Markdown, the default) or "edit" (plain-text
    /// editor). Remembered across notes and launches.
    @AppStorage("iris.editorMode") private var mode = "preview"
    @State private var linkMessage: String?

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var parsed: FfiParsedNode?
    @State private var linkedCount: Int = 0
    @State private var loadError: String?
    @State private var showExport = false
    /// The editable body (trimmed). `prefix`/`suffix` are the file's original
    /// leading/trailing whitespace, put back on save so an unedited file's
    /// bytes don't shift.
    @State private var draft = ""
    @State private var savedDraft = ""
    @State private var prefix = "\n"
    @State private var suffix = "\n"
    @State private var saveError: String?
    @State private var newTag = ""
    @EnvironmentObject private var session: VaultSession
    /// Identifies this editor so it ignores its own `session.change` echo.
    @State private var editorID = UUID()
    /// Another window saved this note while this one had unsaved edits.
    @State private var changedElsewhere = false
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 0) {
                if let parsed {
                    metaRow(for: parsed.node)
                    HStack(alignment: .firstTextBaseline) {
                        Text(titleFor(parsed.node))
                            .font(Typography.h1())
                            .foregroundStyle(c.textPrimary)
                        Spacer()
                        Picker("Mode", selection: $mode) {
                            Text("Preview").tag("preview")
                            Text("Edit").tag("edit")
                        }
                        .pickerStyle(.segmented).labelsHidden().frame(width: 140)
                        .accessibilityLabel("Note mode")
                        Button("Open in New Window") { openWindow(value: relPath) }
                            .buttonStyle(.bordered)
                        Button("Export…") { showExport = true }
                            .buttonStyle(.bordered)
                    }
                    .padding(.bottom, Space.sm)

                    HStack(spacing: Space.xs) {
                        ForEach(parsed.node.tags, id: \.self) { tag in
                            HStack(spacing: Space.xxs) {
                                Text(tag)
                                Button { removeTag(tag) } label: { Image(systemName: "xmark").font(.system(size: 8, weight: .bold)) }
                                    .buttonStyle(.plain)
                                    .accessibilityLabel("Remove tag \(tag)")
                            }
                            .font(Typography.caption())
                            .foregroundStyle(c.textSecondary)
                            .padding(.horizontal, Space.sm)
                            .padding(.vertical, Space.xxs)
                            .background(c.bgHover)
                            .clipShape(Capsule())
                            .overlay(Capsule().stroke(c.borderDefault, lineWidth: 1))
                        }
                        TextField("Add tag", text: $newTag)
                            .textFieldStyle(.plain)
                            .font(Typography.caption())
                            .frame(width: 80)
                            .onSubmit(addTag)
                            .accessibilityLabel("Add tag")
                    }
                    .padding(.bottom, Space.xl)

                } else if let loadError {
                    Text(loadError).font(Typography.bodySans()).foregroundStyle(c.danger)
                }
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl + Space.lg, bottom: 0, trailing: Space.xxxl + Space.lg))
            .frame(maxWidth: .infinity, alignment: .leading)

            if parsed != nil && mode == "preview" {
                ScrollView {
                    Group {
                        if draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                            Text("Nothing here yet — switch to Edit to start writing.")
                                .font(Typography.serif(16)).italic().foregroundStyle(c.textDisabled)
                        } else {
                            MarkdownBodyView(source: draft, onWikiLink: openWikiLink, onToggleCheck: toggleCheck)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(EdgeInsets(top: Space.sm, leading: Space.xxxl + Space.lg, bottom: Space.xl, trailing: Space.xxxl + Space.lg))
                }
                .background(Button("", action: toggleMode).keyboardShortcut("e", modifiers: .command).opacity(0).accessibilityHidden(true))
                if let linkMessage {
                    Text(linkMessage).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                        .padding(.horizontal, Space.xxxl + Space.lg).padding(.vertical, Space.sm)
                }
            } else if parsed != nil {
                TextEditor(text: $draft)
                    .font(Typography.serif(16))
                    .foregroundStyle(c.textPrimary)
                    .lineSpacing(6)
                    .scrollContentBackground(.hidden)
                    .frame(maxWidth: 600 + Space.xxxl, maxHeight: .infinity, alignment: .leading)
                    .padding(EdgeInsets(top: 0, leading: Space.xxxl + Space.lg - 5, bottom: 0, trailing: 0))
                    .accessibilityLabel("Note body")
                    .background(Button("", action: { save() }).keyboardShortcut("s", modifiers: .command).opacity(0).accessibilityHidden(true))
                    .background(Button("", action: toggleMode).keyboardShortcut("e", modifiers: .command).opacity(0).accessibilityHidden(true))
                    .task(id: draft) {
                        guard draft != savedDraft else { return }
                        try? await Task.sleep(for: .seconds(2))
                        // Never autosave over another window's version until the banner is answered.
                        if !Task.isCancelled && !changedElsewhere { save() }
                    }
                if changedElsewhere {
                    HStack(spacing: Space.md) {
                        Text("This note was changed in another window.")
                            .font(Typography.bodySmall()).foregroundStyle(c.warning)
                        Spacer()
                        Button("Reload") { load(); changedElsewhere = false }
                            .accessibilityHint("Discard my unsaved edits and show the other window's version")
                        Button("Keep mine") { save(); changedElsewhere = false }
                            .accessibilityHint("Save my edits over the other window's version")
                    }
                    .buttonStyle(.bordered)
                    .padding(.horizontal, Space.xxxl + Space.lg)
                    .padding(.vertical, Space.sm)
                    .background(c.warningTint)
                }
                if let saveError {
                    Text("Not saved: \(saveError)")
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.danger)
                        .padding(.horizontal, Space.xxxl + Space.lg)
                        .padding(.vertical, Space.sm)
                }
            }
        }
        .background(c.bgCanvas)
        .onDisappear { save() }
        .id(relPath) // fresh load whenever a different node opens
        .task(id: relPath) { load() }
        .onReceive(session.$change) { change in
            guard let change, change.path == relPath, change.origin != editorID else { return }
            if draft == savedDraft {
                load()   // nothing unsaved here: take the other window's version
                changedElsewhere = false
            } else {
                // Unsaved edits here: keep them and ask (Reload / Keep mine banner);
                // autosave is paused until answered. Closing the window still saves —
                // the other window's version stays recoverable in git history.
                changedElsewhere = true
            }
        }
        .sheet(isPresented: $showExport) {
            ExportSheet(engine: engine, relPath: relPath, title: titleFor(parsed?.node), dismiss: { showExport = false })
        }
    }

    private func metaRow(for node: FfiNode) -> some View {
        HStack(spacing: Space.sm) {
            Text(typeLabel(node.nodeType).uppercased())
            Text("·").foregroundStyle(c.textDisabled)
            Text("\(linkedCount) linked note\(linkedCount == 1 ? "" : "s")")
            Text("·").foregroundStyle(c.textDisabled)
            distillationDots(node.distillationLevel)
        }
        .font(Typography.bodySmall())
        .foregroundStyle(c.textSecondary)
        .padding(.bottom, Space.md)
    }

    private func distillationDots(_ level: DistillationLevel?) -> some View {
        let stages: [DistillationLevel] = [.raw, .bolded, .highlighted, .summarized]
        let current = level ?? .raw
        let activeIndex = stages.firstIndex(of: current) ?? 0
        return HStack(spacing: 5) {
            HStack(spacing: 5) {
                ForEach(Array(stages.enumerated()), id: \.offset) { index, _ in
                    Circle()
                        .fill(index == activeIndex ? c.accentDefault : c.borderStrong)
                        .frame(width: 5, height: 5)
                }
            }
            Text(labelFor(current)).foregroundStyle(c.accentDefault)
        }
    }

    private func labelFor(_ level: DistillationLevel) -> String {
        switch level {
        case .raw: return "raw"
        case .bolded: return "bolded"
        case .highlighted: return "highlighted"
        case .summarized: return "summarized"
        }
    }

    private func typeLabel(_ type: NodeType) -> String {
        switch type {
        case .note: return "note"
        case .task: return "task"
        case .event: return "event"
        case .project: return "project"
        case .area: return "area"
        case .resource: return "resource"
        case .space: return "space"
        case .annotation: return "annotation"
        case .inkNote: return "ink note"
        case .reminder: return "reminder"
        case .dailyNote: return "daily note"
        case .tradingJournalEntry: return "trading journal"
        case .musicIdea: return "music idea"
        case .readingItem: return "reading item"
        case .custom(let name): return name
        }
    }

    /// No dedicated `title` field in the schema yet — the file's basename
    /// stands in, same stand-in `search.rs`/`Sidebar` already use.
    private func titleFor(_ node: FfiNode?) -> String {
        (relPath as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func addTag() {
        let tag = newTag.trimmingCharacters(in: .whitespacesAndNewlines)
        newTag = ""
        if !tag.isEmpty { editTags { if !$0.contains(tag) { $0.append(tag) } } }
    }

    private func removeTag(_ tag: String) { editTags { $0.removeAll { $0 == tag } } }

    /// Re-read the node fresh (so a body/metadata change made elsewhere isn't
    /// overwritten), apply the tag edit, and save it as one undoable update.
    private func editTags(_ change: (inout [String]) -> Void) {
        do {
            var node = try engine.readNode(relPath: relPath).node
            change(&node.tags)
            try engine.updateNode(relPath: relPath, node: node)
            parsed?.node.tags = node.tags
            saveError = nil
            session.noteChanged(path: relPath, from: editorID)
        } catch {
            saveError = String(describing: error)
        }
    }

    /// A clicked Preview checkbox: flip that one `[ ]`/`[x]` in the draft and
    /// save immediately (a click is a discrete action, not typing).
    private func toggleCheck(_ index: Int) {
        guard let flipped = MarkdownBlocks.toggleCheck(in: draft, index: index) else { return }
        draft = flipped
        save()
    }

    private func toggleMode() { mode = mode == "preview" ? "edit" : "preview" }

    /// Resolve `[[Target]]` to a node by file-name stem (case-insensitive;
    /// hyphens count as spaces, matching how the app slugs new notes).
    private func openWikiLink(_ target: String) {
        func stem(_ path: String) -> String {
            ((path as NSString).lastPathComponent as NSString).deletingPathExtension.lowercased()
        }
        let wanted = target.lowercased()
        let hits = (try? engine.search(query: target, nodeType: nil, domain: nil, tag: nil)) ?? []
        let hit = hits.first { stem($0.path) == wanted }
            ?? hits.first { stem($0.path).replacingOccurrences(of: "-", with: " ") == wanted }
        if let hit, let onOpenNode {
            linkMessage = nil
            onOpenNode(hit)
        } else {
            linkMessage = "No note named “\(target)”."
        }
    }

    /// Write the draft back (frontmatter re-read fresh so a metadata change
    /// made elsewhere isn't overwritten). No-op when nothing changed.
    private func save() {
        guard parsed != nil, draft != savedDraft else { return }
        do {
            let current = try engine.readNode(relPath: relPath)
            try engine.updateNodeWithBody(relPath: relPath, node: current.node, body: prefix + draft + suffix)
            savedDraft = draft
            saveError = nil
            changedElsewhere = false
            session.noteChanged(path: relPath, from: editorID)
        } catch {
            saveError = String(describing: error)
        }
    }

    private func load() {
        do {
            let result = try engine.readNode(relPath: relPath)
            parsed = result
            let raw = result.body
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            draft = trimmed
            savedDraft = trimmed
            if trimmed.isEmpty {
                prefix = "\n"; suffix = "\n"
            } else {
                prefix = String(raw.prefix(while: { $0.isWhitespace || $0.isNewline }))
                suffix = String(String(raw.reversed().prefix(while: { $0.isWhitespace || $0.isNewline })).reversed())
            }
            saveError = nil
            linkedCount = (try? engine.connections(nodeId: result.node.id))?.count ?? 0
            loadError = nil
        } catch {
            parsed = nil
            loadError = String(describing: error)
        }
    }
}
