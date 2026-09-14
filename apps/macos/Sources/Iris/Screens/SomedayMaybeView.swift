import SwiftUI
import IrisCore

/// `design/canvas/SomedayMaybe.dc.html` — tasks with no `scheduled_date`
/// and no `due_date` (`views::someday_maybe`), the GTD holding area. The
/// **only** task lens with no `has_project` filter at all — a task already
/// filed to a project still lands here if it carries no dates, so a
/// project label still matters here unlike Inbox.
struct SomedayMaybeView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var items: [CachedNode] = []
    @State private var projectTitles: [String: String] = [:]
    @State private var scheduleDates: [String: Date] = [:]

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.md) {
                    TaskLensHeader(
                        title: "Someday/Maybe", trailing: "\(items.count) items",
                        description: "Tasks with no schedule and no due date. Unlike Inbox, this isn't scoped by project."
                    )

                    if items.isEmpty {
                        TaskLensEmptyState(title: "Nothing parked here", message: "Tasks with no schedule or due date show up here. Give one a date to move it into Upcoming or Today.")
                    } else {
                        VStack(spacing: 0) {
                            ForEach(items, id: \.id) { item in
                                TaskLensRow(
                                    title: taskTitleFor(item), meta: projectLabel(item, titles: projectTitles),
                                    onOpen: { onOpenNode(item) },
                                    trailing: { scheduleControl(for: item) }
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
                TaskLensRailSection("About Someday/Maybe") {
                    Text("The only lens with no project filter at all — it cuts straight across the PARA structure.")
                        .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }
            }
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    /// A real `DatePicker` popover, not a bare "Schedule…" click action —
    /// picking a date calls `scheduleTask` and the row leaves this list,
    /// same as the mockup's demo describes (once a date is set, the item
    /// would surface in Upcoming or Today instead).
    private func scheduleControl(for item: CachedNode) -> some View {
        let binding = Binding<Date>(
            get: { scheduleDates[item.id] ?? Date() },
            set: { scheduleDates[item.id] = $0 }
        )
        return Menu {
            DatePicker("", selection: binding, displayedComponents: .date)
                .datePickerStyle(.graphical)
                .labelsHidden()
            Button("Set date") {
                scheduleTask(engine: engine, relPath: item.path, date: binding.wrappedValue)
                load()
            }
        } label: {
            Text("Schedule…").font(Typography.bodySmall()).foregroundStyle(c.accentDefault)
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
    }

    private func byProject() -> [(String, Int)] {
        var counts: [String: Int] = [:]
        for item in items {
            counts[projectLabel(item, titles: projectTitles), default: 0] += 1
        }
        return counts.sorted { $0.key < $1.key }
    }

    private func load() {
        items = (try? engine.somedayMaybe()) ?? []
        projectTitles = projectTitleMap(engine: engine)
    }
}
