import SwiftUI
import IrisCore

/// `design/canvas/Today.dc.html` — scheduled-today OR overdue-and-not-done
/// (`views::today`). The mockup's own demo note is a real backend
/// asymmetry, not a UI choice: only the overdue half of the query checks
/// `status`, so checking off an overdue item removes it, but checking off
/// a today-scheduled item leaves it in the list, just crossed out.
struct TodayView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var items: [CachedNode] = []

    private var overdue: [CachedNode] {
        items.filter { isOverdue($0) }
    }
    private var scheduled: [CachedNode] {
        items.filter { !isOverdue($0) }
    }

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.md) {
                    TaskLensHeader(
                        title: "Today", trailing: todayLabel(),
                        description: "Scheduled for today, or overdue and not done yet."
                    )

                    if items.isEmpty {
                        TaskLensEmptyState(title: "Nothing due today", message: "Scheduled and overdue tasks show up here. Clear day ahead.")
                    } else {
                        if !overdue.isEmpty {
                            groupLabel("Overdue")
                            ForEach(overdue, id: \.id) { item in
                                TaskLensRow(
                                    title: taskTitleFor(item), meta: overdueLabel(item),
                                    trailingTag: nil,
                                    checkbox: .init(checked: false, toggle: { toggle(item) }),
                                    onOpen: { onOpenNode(item) }
                                )
                            }
                        }
                        if !scheduled.isEmpty {
                            groupLabel("Scheduled today")
                            ForEach(scheduled, id: \.id) { item in
                                TaskLensRow(
                                    title: taskTitleFor(item), meta: item.status == "done" ? "Done" : "Scheduled today",
                                    titleStruckThrough: item.status == "done",
                                    checkbox: .init(checked: item.status == "done", toggle: { toggle(item) }),
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
                TaskLensRailSection("Today at a glance") {
                    VStack(spacing: Space.sm) {
                        TaskLensCountRow(label: "Overdue", count: overdue.count, dotColor: c.danger)
                        TaskLensCountRow(label: "Scheduled", count: scheduled.count, dotColor: c.accentDefault)
                    }
                }
                TaskLensRailDivider()
                TaskLensRailSection("About Today") {
                    Text("One of five filtered lenses over the same task nodes — the app's default launch view.")
                        .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }
            }
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private func groupLabel(_ text: String) -> some View {
        Text(text.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary).padding(.top, Space.sm)
    }

    /// `views::today` doesn't tag which half of the OR a row satisfied — a
    /// row counts as "overdue" here if it has a `dueDate` in the past,
    /// mirroring the query's own overdue condition (`due_date <= today`)
    /// client-side, since `CachedNode` doesn't carry that distinction.
    private func isOverdue(_ node: CachedNode) -> Bool {
        guard let due = node.dueDate, node.status != "done" else { return false }
        return due <= isoToday()
    }

    private func overdueLabel(_ node: CachedNode) -> String {
        guard let due = node.dueDate else { return "Overdue" }
        return "Overdue since \(due)"
    }

    private func todayLabel() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "EEEE, MMM d"
        return formatter.string(from: Date())
    }

    private func isoToday() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: Date())
    }

    private func toggle(_ item: CachedNode) {
        toggleTaskDone(engine: engine, relPath: item.path, currentlyDone: item.status == "done")
        load()
    }

    private func load() {
        items = (try? engine.today(today: isoToday())) ?? []
    }
}
