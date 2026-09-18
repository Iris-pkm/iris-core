import SwiftUI
import IrisCore

/// The shared detail view every node type opens into (`design/navigation.md`
/// §3) — one editor shell, not four per-type designs, matching the single
/// `Node` model in `iris-core`.
///
/// **Scope, honestly flagged:** renders the body as plain paragraphs, not
/// full Tier-A rich markdown (headings/lists/checklists/tables/etc., ADR-027)
/// — that's real, separate scope, not something to fake with a naive
/// Markdown-to-Text pass. Editing is also not wired yet (no `updateNode`
/// call on body/tag edits) — this is the read/display half of the screen
/// first, matching "do one thing at a time."
struct NodeEditorView: View {
    let engine: FfiEngine
    let relPath: String

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var parsed: FfiParsedNode?
    @State private var linkedCount: Int = 0
    @State private var loadError: String?
    @State private var showExport = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if let parsed {
                    metaRow(for: parsed.node)
                    HStack(alignment: .firstTextBaseline) {
                        Text(titleFor(parsed.node))
                            .font(Typography.h1())
                            .foregroundStyle(c.textPrimary)
                        Spacer()
                        Button("Export…") { showExport = true }
                            .buttonStyle(.bordered)
                    }
                    .padding(.bottom, Space.sm)

                    if !parsed.node.tags.isEmpty {
                        HStack(spacing: Space.xs) {
                            ForEach(parsed.node.tags, id: \.self) { tag in
                                Text(tag)
                                    .font(Typography.caption())
                                    .foregroundStyle(c.textSecondary)
                                    .padding(.horizontal, Space.sm)
                                    .padding(.vertical, Space.xxs)
                                    .background(c.bgHover)
                                    .clipShape(Capsule())
                                    .overlay(Capsule().stroke(c.borderDefault, lineWidth: 1))
                            }
                        }
                        .padding(.bottom, Space.xl)
                    }

                    Text(parsed.body.trimmingCharacters(in: .whitespacesAndNewlines))
                        .font(Typography.serif(16))
                        .foregroundStyle(c.textPrimary)
                        .lineSpacing(6)
                        .frame(maxWidth: 600, alignment: .leading)
                } else if let loadError {
                    Text(loadError).font(Typography.bodySans()).foregroundStyle(c.danger)
                }
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl + Space.lg, bottom: Space.xl, trailing: Space.xxxl + Space.lg))
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(c.bgCanvas)
        .id(relPath) // fresh load whenever a different node opens
        .task(id: relPath) { load() }
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

    private func load() {
        do {
            let result = try engine.readNode(relPath: relPath)
            parsed = result
            linkedCount = (try? engine.connections(nodeId: result.node.id))?.count ?? 0
            loadError = nil
        } catch {
            parsed = nil
            loadError = String(describing: error)
        }
    }
}
