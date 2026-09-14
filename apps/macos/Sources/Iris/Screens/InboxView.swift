import SwiftUI
import IrisCore

/// `design/canvas/Inbox.dc.html` — tasks with no project yet, in capture
/// order (`views::inbox`, `has_project = 0 ORDER BY id`). First of the five
/// task-view lenses (ARCHITECTURE.md §12).
///
/// **Flagged, matching the mockup's own note:** the query only checks "no
/// project" — the fuller no-date/no-priority definition from the PRD isn't
/// enforced at the backend, so an Inbox item can still carry a due date and
/// priority (shown here, not hidden, same as the mockup deliberately does).
struct InboxView: View {
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
                        title: "Inbox", trailing: "\(items.count) items",
                        description: "Tasks with no project yet, in the order you captured them. File one to a project to clear it from here."
                    )

                    if items.isEmpty {
                        TaskLensEmptyState(title: "Inbox zero", message: "Nothing waiting to be filed. New captures with no project land here first.")
                    } else {
                        VStack(spacing: 0) {
                            ForEach(items, id: \.id) { item in
                                TaskLensRow(
                                    title: taskTitleFor(item),
                                    meta: item.dueDate.map { "due \($0)" } ?? (item.status == "doing" ? "In progress" : "No date"),
                                    priority: item.priority,
                                    onOpen: { onOpenNode(item) },
                                    trailing: { fileMenu(for: item) }
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
                TaskLensRailSection("By priority") {
                    VStack(spacing: Space.sm) {
                        ForEach(["urgent", "high", "normal", "low"], id: \.self) { p in
                            TaskLensCountRow(label: priorityLabel(p) ?? p, count: items.filter { $0.priority == p }.count)
                        }
                    }
                }
                TaskLensRailDivider()
                TaskLensRailSection("About Inbox") {
                    Text("Inbox, Today, Upcoming, Someday/Maybe, and Logbook are filtered lenses over the same task nodes, not separate data.")
                        .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }
            }
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    /// A real "File…" needs a project picker, not a bare click action —
    /// only real, existing projects (via `engine.search`) are offered.
    private func fileMenu(for item: CachedNode) -> some View {
        Menu {
            if projectTitles.isEmpty {
                Text("No projects yet")
            }
            ForEach(Array(projectTitles.keys.sorted { projectTitles[$0]! < projectTitles[$1]! }), id: \.self) { projectId in
                Button(projectTitles[projectId] ?? projectId) {
                    fileToProject(engine: engine, relPath: item.path, projectId: projectId)
                    load()
                }
            }
        } label: {
            Text("File…").font(Typography.bodySmall()).foregroundStyle(c.accentDefault)
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
    }

    private func load() {
        items = (try? engine.inbox()) ?? []
        projectTitles = projectTitleMap(engine: engine)
    }
}
