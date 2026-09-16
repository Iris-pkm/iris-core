import SwiftUI
import IrisCore

/// `design/canvas/Tasks.dc.html` — a project-scoped task list, distinct
/// from the five global task-view lenses (Inbox/Today/Upcoming/etc.):
/// reached from within a Project's own route (`design/navigation.md`'s
/// Project Tasks row), not the sidebar.
///
/// **Backend gap closed:** no query returned "every task in project X" —
/// the five lenses in `views.rs` are all global, and `distillation.rs`/
/// `activation.rs`'s project-scoped queries are each their own narrower
/// slice (undistilled notes, blocked tasks, etc.), not the full task list.
/// Added `views::project_tasks(cache, project_id)`.
///
/// **Subtask hierarchy is derived, not stored separately:** the mockup's
/// "3 subtasks" grouping comes from each task's own `parent` relation
/// (SCHEMA_SPEC §5 — Epic→Story→Subtask nesting), read via the existing
/// `connections` FFI call per task rather than adding new backend surface
/// for what's one relation lookup. One level of nesting only, matching the
/// mockup; a subtask that itself has subtasks isn't specced here.
struct ProjectTasksView: View {
    let engine: FfiEngine
    let projectId: String
    let projectTitle: String
    let onBack: () -> Void
    let onOpenTask: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private struct Group_ { let parent: CachedNode; let subtasks: [CachedNode] }

    @State private var roots: [Group_] = []
    @State private var allTasks: [CachedNode] = []

    private var doneCount: Int { allTasks.filter { $0.status == "done" }.count }
    private var totalCount: Int { allTasks.count }

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    breadcrumb

                    VStack(alignment: .leading, spacing: Space.sm) {
                        HStack(alignment: .lastTextBaseline) {
                            Text("Tasks").font(Typography.h1()).foregroundStyle(c.textPrimary)
                            Spacer()
                            Text("\(doneCount) of \(totalCount) done").font(Typography.caption()).foregroundStyle(c.textSecondary)
                        }

                        progressBar

                        VStack(alignment: .leading, spacing: 2) {
                            ForEach(roots, id: \.parent.id) { group in
                                taskRow(group.parent, isParent: !group.subtasks.isEmpty, subtaskCount: group.subtasks.count)
                                ForEach(group.subtasks, id: \.id) { sub in
                                    taskRow(sub, isParent: false, subtaskCount: 0, indent: true)
                                }
                            }
                            newTaskRow
                        }
                    }
                    .padding(EdgeInsets(top: Space.lg, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
                }
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
            }
            .buttonStyle(.plain)
            Text("/").foregroundStyle(c.textDisabled)
            Text("Tasks").font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
        }
        .padding(EdgeInsets(top: Space.md, leading: Space.xxl, bottom: 0, trailing: Space.xxl))
    }

    private var progressBar: some View {
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                Capsule().fill(c.bgHover).frame(height: 4)
                Capsule().fill(c.accentDefault)
                    .frame(width: totalCount == 0 ? 0 : geo.size.width * CGFloat(doneCount) / CGFloat(totalCount), height: 4)
            }
        }
        .frame(height: 4)
        .padding(.bottom, Space.sm)
    }

    /// A parent row with subtasks shows a decorative mixed/empty/done
    /// checkbox aggregated from its subtasks' status (no direct click —
    /// there's nothing single to toggle); a leaf task's checkbox is the
    /// real, clickable one via the same `toggleTaskDone` helper the
    /// task-view lenses use.
    private func taskRow(_ task: CachedNode, isParent: Bool, subtaskCount: Int, indent: Bool = false) -> some View {
        HStack(spacing: Space.md) {
            if isParent {
                aggregateCheckbox(for: task)
            } else {
                Button {
                    toggleTaskDone(engine: engine, relPath: task.path, currentlyDone: task.status == "done")
                    load()
                } label: {
                    checkboxView(checked: task.status == "done")
                }
                .buttonStyle(.plain)
            }

            Button {
                onOpenTask(task)
            } label: {
                Text(taskTitleFor(task))
                    .font(Typography.sans(14, weight: isParent ? .medium : .regular))
                    .foregroundStyle(task.status == "done" ? c.textSecondary : c.textPrimary)
                    .strikethrough(task.status == "done" && !isParent)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)

            if isParent {
                Text("\(subtaskCount) subtask\(subtaskCount == 1 ? "" : "s")").font(Typography.caption()).foregroundStyle(c.textSecondary)
            } else {
                statusBadge(task.status)
                if let due = task.dueDate ?? task.scheduledDate {
                    Text(shortDate(due)).font(Typography.caption()).foregroundStyle(c.textSecondary).frame(width: 52, alignment: .trailing)
                }
            }
        }
        .padding(.vertical, Space.sm)
        .padding(.leading, indent ? Space.xxl : 0)
        .background(task.status == "doing" ? c.bgSelected : Color.clear)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func checkboxView(checked: Bool) -> some View {
        ZStack {
            RoundedRectangle(cornerRadius: 4)
                .strokeBorder(checked ? c.success : c.borderStrong, lineWidth: 1.5)
                .background(RoundedRectangle(cornerRadius: 4).fill(checked ? c.success : Color.clear))
                .frame(width: 16, height: 16)
            if checked {
                Image(systemName: "checkmark").font(.system(size: 9, weight: .bold)).foregroundStyle(c.bgCanvas)
            }
        }
    }

    private func aggregateCheckbox(for parent: CachedNode) -> some View {
        let subs = roots.first(where: { $0.parent.id == parent.id })?.subtasks ?? []
        let doneSubs = subs.filter { $0.status == "done" }.count
        if subs.isEmpty || doneSubs == 0 {
            return AnyView(RoundedRectangle(cornerRadius: 4).strokeBorder(c.borderStrong, lineWidth: 1.5).frame(width: 16, height: 16))
        } else if doneSubs == subs.count {
            return AnyView(checkboxView(checked: true))
        } else {
            return AnyView(
                RoundedRectangle(cornerRadius: 4)
                    .strokeBorder(c.borderStrong, lineWidth: 1.5)
                    .background(RoundedRectangle(cornerRadius: 4).fill(Color.clear))
                    .overlay(RoundedRectangle(cornerRadius: 1).fill(c.borderStrong).padding(4))
                    .frame(width: 16, height: 16)
            )
        }
    }

    private func statusBadge(_ status: String?) -> some View {
        let (label, color, tint): (String, Color, Color) = {
            switch status {
            case "done": return ("done", c.success, c.successTint)
            case "doing": return ("doing", c.accentDefault, c.accentTint)
            default: return ("todo", c.textSecondary, c.bgHover)
            }
        }()
        return Text(label)
            .font(.system(size: 11))
            .padding(.horizontal, Space.sm)
            .padding(.vertical, 2)
            .background(tint)
            .foregroundStyle(color)
            .clipShape(Capsule())
    }

    /// Not wired — creating a task needs a destination-path/template
    /// decision this screen alone shouldn't make (same category of gap as
    /// Onboarding's disabled import sources): shown, honestly inert.
    private var newTaskRow: some View {
        HStack(spacing: Space.md) {
            Text("+").font(.system(size: 14))
            Text("New task").font(Typography.bodySans())
        }
        .foregroundStyle(c.textDisabled)
        .padding(.vertical, Space.sm)
    }

    private var rightRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("STATUS").font(Typography.caption()).foregroundStyle(c.textSecondary)
                statusCountRow("Todo", count: allTasks.filter { $0.status == nil || $0.status == "todo" }.count, dot: c.borderStrong)
                statusCountRow("Doing", count: allTasks.filter { $0.status == "doing" }.count, dot: c.accentDefault)
                statusCountRow("Done", count: allTasks.filter { $0.status == "done" }.count, dot: c.success)
            }
            Rectangle().fill(c.borderDefault).frame(height: 1)
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("DUE THIS WEEK").font(Typography.caption()).foregroundStyle(c.textSecondary)
                let due = dueThisWeek()
                if due.isEmpty {
                    Text("Nothing due").font(Typography.bodySmall()).foregroundStyle(c.textDisabled)
                }
                ForEach(due, id: \.id) { task in
                    HStack {
                        Text(taskTitleFor(task)).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                        Spacer()
                        Text(shortDate((task.dueDate ?? task.scheduledDate)!)).font(Typography.caption()).foregroundStyle(c.textSecondary)
                    }
                }
            }
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 260)
        .frame(maxHeight: .infinity)
        .background(c.bgSurface)
    }

    private func statusCountRow(_ label: String, count: Int, dot: Color) -> some View {
        HStack {
            HStack(spacing: Space.xs) {
                Circle().fill(dot).frame(width: 7, height: 7)
                Text(label).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
            }
            Spacer()
            Text("\(count)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }

    private func dueThisWeek() -> [CachedNode] {
        let formatter = isoFormatter()
        let today = Date()
        guard let weekOut = Calendar.current.date(byAdding: .day, value: 7, to: today) else { return [] }
        return allTasks.filter { task in
            guard let iso = task.dueDate ?? task.scheduledDate, let date = formatter.date(from: iso) else { return false }
            return date >= Calendar.current.startOfDay(for: today) && date <= weekOut
        }
    }

    private func shortDate(_ iso: String) -> String {
        guard let date = isoFormatter().date(from: iso) else { return iso }
        let formatter = DateFormatter()
        formatter.dateFormat = "EEE"
        return formatter.string(from: date)
    }

    private func isoFormatter() -> DateFormatter {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }

    private func load() {
        let tasks = (try? engine.projectTasks(projectId: projectId)) ?? []
        allTasks = tasks

        var childrenByParent: [String: [CachedNode]] = [:]
        var isSubtask = Set<String>()

        for task in tasks {
            guard let conns = try? engine.connections(nodeId: task.id) else { continue }
            for conn in conns where conn.relType == "parent" {
                if conn.direction == .outgoing {
                    isSubtask.insert(task.id)
                    childrenByParent[conn.node.id, default: []].append(task)
                }
            }
        }

        roots = tasks
            .filter { !isSubtask.contains($0.id) }
            .map { Group_(parent: $0, subtasks: childrenByParent[$0.id] ?? []) }
    }
}
