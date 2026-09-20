import SwiftUI
import IrisCore

/// The dedicated Distillation Queue view (`design/canvas/Distillation.dc.html`),
/// reached via "View all" from `ProjectView`'s Guided Activation preview
/// card. Resolves `navigation.md` §5's flagged open question as **both**:
/// the preview card stays a live 3-item glimpse, this is the full,
/// dedicated queue.
///
/// Backed entirely by existing backend surface — `distillation.rs`'s
/// `queue(cache, project_id)` (one project, id-ordered; the mockup's own
/// doc comment note carries over here verbatim: "a real ranking… is a
/// UI-layer concern"). Sorting by level is therefore done client-side over
/// the id-ordered result, matching the mockup exactly rather than adding
/// backend sort options for a UI-only concern.
struct DistillationQueueView: View {
    let engine: FfiEngine
    let projectId: String
    let projectTitle: String
    let onBack: () -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private enum Sort { case defaultOrder, byLevel }
    @State private var sort: Sort = .defaultOrder
    @State private var notes: [CachedNode] = []

    private static let levelOrder = ["raw", "bolded", "highlighted", "summarized"]
    private static let levelLabels = ["raw": "Raw", "bolded": "Bolded", "highlighted": "Highlighted", "summarized": "Summarized"]

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 0) {
                breadcrumb

                VStack(alignment: .leading, spacing: Space.md) {
                    HStack(alignment: .lastTextBaseline) {
                        Text("Distillation Queue").font(Typography.h1()).foregroundStyle(c.textPrimary)
                        Spacer()
                        Text("\(sortedNotes.count) items").font(Typography.caption()).foregroundStyle(c.textSecondary)
                    }
                    Text("Notes belonging to this project that aren't fully \u{201c}summarized\u{201d} yet, scoped to one project — there's no cross-project queue.")
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.textSecondary)

                    sortSegmented

                    if sortedNotes.isEmpty {
                        emptyState
                    } else {
                        rows
                    }
                }
                .padding(EdgeInsets(top: Space.xl, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)

            Divider().background(c.borderDefault)
            rightRail
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private var breadcrumb: some View {
        HStack(spacing: Space.xs) {
            Button(action: onBack) {
                Text(projectTitle).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                    .padding(.vertical, Space.xxs)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            Text("/").foregroundStyle(c.textDisabled)
            Text("Distillation Queue").font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
        }
        .padding(EdgeInsets(top: Space.md, leading: Space.xxl, bottom: 0, trailing: Space.xxl))
    }

    private var sortSegmented: some View {
        HStack(spacing: 2) {
            segment("Default order", isActive: sort == .defaultOrder) { sort = .defaultOrder }
            segment("By level", isActive: sort == .byLevel) { sort = .byLevel }
        }
        .padding(2)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func segment(_ title: String, isActive: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(Typography.bodySmall())
                .foregroundStyle(isActive ? c.textPrimary : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(isActive ? c.bgSurfaceRaised : Color.clear)
                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var emptyState: some View {
        VStack(spacing: Space.xs) {
            Text("Queue clear").font(Typography.serif(17, weight: .semibold)).foregroundStyle(c.textPrimary)
            Text("Every note in this project has reached \u{201c}summarized.\u{201d} New raw or partially-distilled notes will show up here.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 320)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, Space.xxxl)
    }

    private var rows: some View {
        VStack(spacing: 0) {
            ForEach(sortedNotes, id: \.id) { note in
                HStack(spacing: Space.md) {
                    Text(titleFor(note)).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                    Spacer()
                    levelPill(for: note)
                }
                .padding(.vertical, Space.md)
                .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
            }
        }
    }

    private func levelPill(for note: CachedNode) -> some View {
        let level = note.distillationLevel ?? "raw"
        let next = nextLevel(after: level)
        let label = Self.levelLabels[level] ?? level
        let nextLabel = Self.levelLabels[next] ?? next
        return Button {
            advance(note)
        } label: {
            Text("\(label) \u{2192} \(nextLabel)")
                .font(Typography.caption())
                .foregroundStyle(pillColor(for: level))
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(pillColor(for: level).opacity(0.15))
                .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }

    private func pillColor(for level: String) -> Color {
        switch level {
        case "bolded": return c.warning
        case "highlighted": return c.accentDefault
        default: return c.textSecondary
        }
    }

    private var rightRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("BY LEVEL").font(Typography.caption()).foregroundStyle(c.textSecondary)
                ForEach(["raw", "bolded", "highlighted"], id: \.self) { level in
                    HStack {
                        HStack(spacing: Space.xs) {
                            Circle().fill(pillColor(for: level)).frame(width: 7, height: 7)
                            Text(Self.levelLabels[level] ?? level).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                        }
                        Spacer()
                        Text("\(sortedNotes.filter { ($0.distillationLevel ?? "raw") == level }.count)")
                            .font(Typography.bodySmall())
                            .foregroundStyle(c.textSecondary)
                    }
                }
            }
            Rectangle().fill(c.borderDefault).frame(height: 1)
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("ABOUT THIS VIEW").font(Typography.caption()).foregroundStyle(c.textSecondary)
                Text("Guided Activation keeps a live 3-item preview card; this dedicated view is the full queue.")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textSecondary)
            }
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 260)
        .background(c.bgSurface)
    }

    // MARK: - Data

    private var sortedNotes: [CachedNode] {
        switch sort {
        case .defaultOrder:
            return notes
        case .byLevel:
            return notes.sorted {
                Self.levelOrder.firstIndex(of: $0.distillationLevel ?? "raw") ?? 0
                    < Self.levelOrder.firstIndex(of: $1.distillationLevel ?? "raw") ?? 0
            }
        }
    }

    private func titleFor(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func nextLevel(after level: String) -> String {
        guard let idx = Self.levelOrder.firstIndex(of: level) else { return "bolded" }
        return Self.levelOrder[min(idx + 1, Self.levelOrder.count - 1)]
    }

    private func advance(_ note: CachedNode) {
        let next = nextLevel(after: note.distillationLevel ?? "raw")
        let level: DistillationLevel
        switch next {
        case "bolded": level = .bolded
        case "highlighted": level = .highlighted
        default: level = .summarized
        }
        _ = try? engine.setDistillationLevel(relPath: note.path, level: level)
        load()
    }

    private func load() {
        notes = (try? engine.distillationQueue(projectId: projectId)) ?? []
    }
}
