import SwiftUI
import IrisCore

/// `design/canvas/PARAWorkbench.dc.html` — one screen, three PARA
/// categories, three rendering lenses (List/Table/Board) over the same
/// graph (`ARCHITECTURE.md` §3), not separate data per lens.
///
/// **Resolves `screen-flow.md`'s flagged sidebar correction:** the mockup's
/// sidebar has *two* independently clickable things per PARA section — the
/// section **header** ("Projects"/"Areas"/"Resources") opens this
/// workbench for that category (kind A, a sibling swap), while an
/// individual named item below it still pushes straight into the Node
/// Editor (kind B) exactly as `AppShell`'s `Sidebar` already does. Both
/// were true; the correction only ever concerned the header, which didn't
/// exist as a destination until this screen was built.
///
/// **Known simplification, flagged:** `CachedNode` (what `engine.search`
/// returns) only carries the columns the five task-view lenses need —
/// no `project_status`, `target_date`, or `read_status`. Rather than
/// widening the cache schema for this one screen, each row's extra detail
/// is fetched with one `readNode` call per item (bounded by realistic PARA
/// counts — tens of items, not thousands). A ceiling worth knowing about,
/// not a hidden cost: this doesn't scale to a very large vault without
/// eventually adding those columns to `cache.rs`.
struct PARAWorkbenchView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void

    enum Category: String, CaseIterable { case projects, areas, resources }
    private enum ViewMode { case list, table, board }

    let initialCategory: Category
    @State private var category: Category
    @State private var mode: ViewMode = .list

    init(engine: FfiEngine, initialCategory: Category, onOpenNode: @escaping (CachedNode) -> Void) {
        self.engine = engine
        self.initialCategory = initialCategory
        self.onOpenNode = onOpenNode
        _category = State(initialValue: initialCategory)
    }

    private struct Row {
        let node: CachedNode
        let title: String
        let meta: String
        let tags: [String]
        let statusLabel: String
        let statusColor: Color
        /// Only set for `.projects` — drives the Board lens's lane grouping.
        let projectStatus: ProjectStatus?
        /// Only meaningful for `.areas` — the linked-note count, kept as a
        /// real `Int` rather than parsed back out of `meta`'s display text.
        var linkedCount: Int = 0
    }

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var rows: [Row] = []

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.md) {
                    header

                    segmented

                    switch mode {
                    case .list: listView
                    case .table: tableView
                    case .board: boardView
                    }
                }
                .padding(EdgeInsets(top: Space.xl, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)

            Divider().background(c.borderDefault)
            rightRail
        }
        .background(c.bgCanvas)
        .task(id: category) {
            if mode == .board && category != .projects { mode = .list }
            load()
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Space.xs) {
            HStack(alignment: .lastTextBaseline) {
                Text(categoryLabel).font(Typography.h1()).foregroundStyle(c.textPrimary)
                Spacer()
                Text("\(rows.count) items").font(Typography.caption()).foregroundStyle(c.textSecondary)
            }
            Text(categoryHint).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }

    private var categoryLabel: String {
        switch category {
        case .projects: return "Projects"
        case .areas: return "Areas"
        case .resources: return "Resources"
        }
    }

    private var categoryHint: String {
        switch category {
        case .projects: return "Node type: project — list/table/board are rendering modes over the same graph, not separate data."
        case .areas: return "Node type: area — ongoing responsibilities, no lifecycle state machine."
        case .resources: return "Node type: resource — read_status tracks the reading pipeline."
        }
    }

    private var segmented: some View {
        HStack(spacing: 2) {
            segButton("List", .list)
            segButton("Table", .table)
            if category == .projects {
                segButton("Board", .board)
            }
        }
        .padding(2)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func segButton(_ title: String, _ target: ViewMode) -> some View {
        Button {
            mode = target
        } label: {
            Text(title)
                .font(Typography.bodySmall())
                .foregroundStyle(mode == target ? c.textPrimary : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(mode == target ? c.bgSurfaceRaised : Color.clear)
                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    // MARK: - List

    private var listView: some View {
        VStack(spacing: 0) {
            ForEach(rows, id: \.node.id) { row in
                Button {
                    onOpenNode(row.node)
                } label: {
                    HStack(spacing: Space.md) {
                        Circle().fill(row.statusColor).frame(width: 7, height: 7)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(row.title).font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
                            Text(row.meta).font(Typography.caption()).foregroundStyle(c.textSecondary)
                        }
                        Spacer()
                        HStack(spacing: Space.xs) {
                            ForEach(row.tags, id: \.self) { tag in
                                Text(tag).font(.system(size: 11)).padding(.horizontal, Space.sm).padding(.vertical, 2)
                                    .background(c.bgHover).foregroundStyle(c.textSecondary).clipShape(Capsule())
                            }
                        }
                        statusBadge(row)
                    }
                    .padding(.vertical, Space.md)
                    .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func statusBadge(_ row: Row) -> some View {
        Text(row.statusLabel)
            .font(.system(size: 11))
            .padding(.horizontal, Space.sm)
            .padding(.vertical, 2)
            .background(row.statusColor.opacity(0.15))
            .foregroundStyle(row.statusColor)
            .clipShape(Capsule())
    }

    // MARK: - Table

    private var tableView: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Name").font(Typography.caption()).frame(maxWidth: .infinity, alignment: .leading)
                Text(statusColumnLabel).font(Typography.caption()).frame(width: 100, alignment: .leading)
                Text(metaColumnLabel).font(Typography.caption()).frame(width: 140, alignment: .leading)
                Text("Tags").font(Typography.caption()).frame(width: 140, alignment: .leading)
            }
            .foregroundStyle(c.textSecondary)
            .padding(Space.sm)
            .background(c.bgHover)

            ForEach(rows, id: \.node.id) { row in
                Button {
                    onOpenNode(row.node)
                } label: {
                    HStack {
                        Text(row.title).font(Typography.bodySans()).foregroundStyle(c.textPrimary).frame(maxWidth: .infinity, alignment: .leading)
                        statusBadge(row).frame(width: 100, alignment: .leading)
                        Text(row.meta).font(Typography.bodySmall()).foregroundStyle(c.textSecondary).frame(width: 140, alignment: .leading)
                        HStack(spacing: Space.xs) {
                            ForEach(row.tags, id: \.self) { tag in
                                Text(tag).font(.system(size: 11)).padding(.horizontal, Space.sm).padding(.vertical, 2)
                                    .background(c.bgHover).foregroundStyle(c.textSecondary).clipShape(Capsule())
                            }
                        }
                        .frame(width: 140, alignment: .leading)
                    }
                    .padding(Space.sm)
                    .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private var statusColumnLabel: String {
        switch category {
        case .resources: return "Read status"
        case .areas: return "State"
        case .projects: return "Status"
        }
    }
    private var metaColumnLabel: String {
        switch category {
        case .projects: return "Target date"
        case .areas: return "Linked notes"
        case .resources: return "Source"
        }
    }

    // MARK: - Board (Projects only)

    private var boardView: some View {
        let order: [ProjectStatus] = [.someday, .planned, .active, .paused, .completed, .cancelled]
        return ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: Space.md) {
                ForEach(order, id: \.self) { status in
                    let lane = rows.filter { $0.projectStatus == status }
                    VStack(alignment: .leading, spacing: Space.sm) {
                        HStack {
                            Text(projectStatusLabel(status)).font(Typography.caption()).foregroundStyle(c.textPrimary)
                            Text("\(lane.count)").font(Typography.caption()).foregroundStyle(c.textSecondary)
                        }
                        .padding(.bottom, Space.xs)
                        .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }

                        ForEach(lane, id: \.node.id) { row in
                            Button {
                                onOpenNode(row.node)
                            } label: {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(row.title).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                                    Text(row.meta).font(.system(size: 11)).foregroundStyle(c.textSecondary)
                                }
                                .padding(Space.sm)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(c.bgCanvas)
                                .overlay(RoundedRectangle(cornerRadius: Radius.sm).stroke(c.borderDefault, lineWidth: 1))
                                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(Space.sm)
                    .frame(width: 200, alignment: .top)
                    .background(c.bgSurface)
                    .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
                    .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
                }
            }
        }
    }

    // MARK: - Right rail

    private var rightRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            VStack(alignment: .leading, spacing: Space.sm) {
                Text(rightRailTitle).font(Typography.caption()).foregroundStyle(c.textSecondary)
                ForEach(rightRailRows(), id: \.0) { label, count, color in
                    HStack {
                        HStack(spacing: Space.xs) {
                            Circle().fill(color).frame(width: 7, height: 7)
                            Text(label).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                        }
                        Spacer()
                        Text("\(count)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                    }
                }
            }
            Rectangle().fill(c.borderDefault).frame(height: 1)
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("REMINDER").font(Typography.caption()).foregroundStyle(c.textSecondary)
                Text(categoryTip).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 240)
        .frame(maxHeight: .infinity)
        .background(c.bgSurface)
    }

    private var rightRailTitle: String {
        switch category {
        case .projects: return "STATUS"
        case .areas: return "LINKED NOTES"
        case .resources: return "READ STATUS"
        }
    }

    private var categoryTip: String {
        switch category {
        case .projects: return "A Project doesn't \u{201c}contain\u{201d} its tasks — membership is a typed relation (parent_project), the same mechanism used for any node-to-node link."
        case .areas: return "Areas are container nodes, not a property — they get their own dashboard, distinct from a domain tag stamped on a node."
        case .resources: return "Archival is a lifecycle state on any node (ADR-016), not a PARA type — there is no separate \u{201c}Archive\u{201d} container here."
        }
    }

    private func rightRailRows() -> [(String, Int, Color)] {
        switch category {
        case .projects:
            let order: [ProjectStatus] = [.someday, .planned, .active, .paused, .completed, .cancelled]
            return order.map { status in
                (projectStatusLabel(status), rows.filter { $0.projectStatus == status }.count, statusColor(for: status))
            }
        case .areas:
            return rows.map { ($0.title, $0.linkedCount, c.success) }
        case .resources:
            return ["unread", "reading", "read"].map { rs in
                (rs.capitalized, rows.filter { $0.statusLabel.lowercased() == rs }.count, readStatusColor(rs))
            }
        }
    }

    // MARK: - Color helpers

    private func statusColor(for status: ProjectStatus) -> Color {
        switch status {
        case .active: return c.accentDefault
        case .completed: return c.success
        case .paused: return c.warning
        case .cancelled: return c.danger
        default: return c.borderStrong
        }
    }

    private func readStatusColor(_ rs: String) -> Color {
        switch rs {
        case "read": return c.success
        case "reading": return c.accentDefault
        default: return c.borderStrong
        }
    }

    private func projectStatusLabel(_ status: ProjectStatus) -> String {
        switch status {
        case .someday: return "Someday"
        case .planned: return "Planned"
        case .active: return "Active"
        case .paused: return "Paused"
        case .completed: return "Completed"
        case .cancelled: return "Cancelled"
        }
    }

    // MARK: - Data

    private func load() {
        let nodeType = category.rawValue.dropLast() // "projects" -> "project", etc. ("resources" -> "resource" too)
        let cached = (try? engine.search(query: "", nodeType: String(nodeType), domain: nil, tag: nil)) ?? []

        switch category {
        case .projects:
            rows = cached.map { node in
                let full = try? engine.readNode(relPath: node.path).node
                let status = full?.projectStatus ?? .someday
                return Row(
                    node: node, title: taskTitleFor(node),
                    meta: "Target \(full?.targetDate ?? "—")",
                    tags: node.tags.split(separator: ",").map(String.init),
                    statusLabel: projectStatusLabel(status), statusColor: statusColor(for: status),
                    projectStatus: status
                )
            }
        case .areas:
            rows = cached.map { node in
                let linked = (try? engine.connections(nodeId: node.id))?.count ?? 0
                return Row(
                    node: node, title: taskTitleFor(node), meta: "\(linked) linked notes",
                    tags: node.tags.split(separator: ",").map(String.init),
                    statusLabel: "Ongoing", statusColor: c.textSecondary, projectStatus: nil,
                    linkedCount: linked
                )
            }
        case .resources:
            rows = cached.map { node in
                let full = try? engine.readNode(relPath: node.path).node
                let readStatus = full?.readStatus ?? "unread"
                return Row(
                    node: node, title: taskTitleFor(node),
                    meta: full?.sourceUrl ?? "No source",
                    tags: node.tags.split(separator: ",").map(String.init),
                    statusLabel: readStatus, statusColor: readStatusColor(readStatus), projectStatus: nil
                )
            }
        }
    }
}
