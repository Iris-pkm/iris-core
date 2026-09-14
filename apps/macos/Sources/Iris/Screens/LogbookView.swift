import SwiftUI
import IrisCore

/// `design/canvas/Logbook.dc.html` — completed tasks (`views::logbook`,
/// `status = 'done'`). Last of the five task-view lenses. Ordered by id,
/// not completion time — flagged directly in `views.rs`'s own doc comment
/// (no `completed_at` field exists yet), not a design choice made here.
struct LogbookView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var items: [CachedNode] = []
    @State private var projectTitles: [String: String] = [:]

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.md) {
                    TaskLensHeader(
                        title: "Logbook", trailing: "\(items.count) items",
                        description: "Completed tasks. Ordered by id, not completion time — there's no completed_at field yet."
                    )

                    if items.isEmpty {
                        TaskLensEmptyState(title: "Nothing completed yet", message: "Finished tasks land here. Check one off anywhere in Iris and it shows up.")
                    } else {
                        VStack(spacing: 0) {
                            ForEach(items, id: \.id) { item in
                                TaskLensRow(
                                    title: taskTitleFor(item), meta: projectLabel(item, titles: projectTitles),
                                    titleStruckThrough: true,
                                    checkbox: .init(checked: true, toggle: { reopen(item) }),
                                    onOpen: { onOpenNode(item) }
                                )
                            }
                        }
                    }
                }
                .padding(EdgeInsets(top: Space.xl, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider().background(c.borderDefault)
            TaskLensRail {
                TaskLensRailSection("By project") {
                    VStack(spacing: Space.sm) {
                        ForEach(byProject(), id: \.0) { label, count in
                            TaskLensCountRow(label: label, count: count)
                        }
                    }
                }
                TaskLensRailDivider()
                TaskLensRailSection("About Logbook") {
                    Text("Last of the five filtered lenses over the same task nodes. Order here is a known backend gap, not a design choice.")
                        .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }
            }
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private func byProject() -> [(String, Int)] {
        var counts: [String: Int] = [:]
        for item in items {
            counts[projectLabel(item, titles: projectTitles), default: 0] += 1
        }
        return counts.sorted { $0.key < $1.key }
    }

    private func reopen(_ item: CachedNode) {
        toggleTaskDone(engine: engine, relPath: item.path, currentlyDone: true)
        load()
    }

    private func load() {
        items = (try? engine.logbook()) ?? []
        projectTitles = projectTitleMap(engine: engine)
    }
}
