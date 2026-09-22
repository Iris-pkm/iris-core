import SwiftUI
import IrisCore

/// `design/canvas/Gantt.dc.html` — pulled forward from Phase 3 alongside
/// Graph (both deliberate scope decisions made with the user), closing the
/// real gap `navigation.md` flagged: no critical-path computation existed
/// anywhere in `iris-core`.
///
/// **Data model, grounded in the schema, not invented:** `SCHEMA_SPEC.md`'s
/// own `project` field table names `start_date`/`target_date` explicitly
/// "for the timeline/Gantt view" — this screen is their only consumer.
/// Milestones are each project's own `event`-type child nodes (already
/// queryable via `parent_project` on `CachedNode`, no new field needed) —
/// the mockup's separately-named milestones ("Race day", "Walkthrough")
/// map onto real event nodes, not a field that doesn't exist.
///
/// **Critical path, a real judgment call, flagged:** the mockup's own
/// `ROWS` data hard-codes `critical: true/false` per row with no
/// computation behind it at all — there's nothing to reverse-engineer.
/// Since projects carry fixed `target_date`s rather than durations, a
/// classical CPM forward/backward pass doesn't apply directly. Defined
/// here instead: a project is critical if it lies on a chain (through
/// `depends-on` edges between shown projects) whose furthest reachable
/// `target_date` equals the overall maximum across every chain — i.e.
/// slipping any project on that chain would slip the latest-finishing
/// deliverable. Documented here since `ARCHITECTURE.md`/`dependencies.rs`
/// never define "critical" for date-anchored (not duration-anchored) work.
///
/// **Full replacement, own bespoke chrome** — confirmed by the mockup
/// itself (no PARA/nav sidebar, just a critical-path toggle and legend),
/// same treatment `GraphView` already gets. Resolves `screen-flow.md`'s
/// flagged "entry trigger TBD": reached via a new standing "Timeline"
/// sidebar row (`AppShell`), not gated on any node being open, since a
/// project timeline needs no single center the way Graph does.
struct TimelineView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void
    let onBack: () -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private struct Row: Identifiable {
        let project: CachedNode
        let title: String
        let start: Date
        let end: Date
        let milestones: [(date: Date, label: String)]
        var isCritical = false
        var hasDependent = false
        var id: String { project.id }
    }

    @State private var rows: [Row] = []
    @State private var showCritical = true

    private static let dayWidth: CGFloat = 17
    private static let nameColWidth: CGFloat = 180

    var body: some View {
        VStack(spacing: 0) {
            topBar
            HStack(spacing: 0) {
                sidebar
                Divider().background(c.borderDefault)
                ScrollView([.horizontal, .vertical]) {
                    chart
                        .padding(Space.xl)
                }
                .background(c.bgCanvas)
            }
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private var topBar: some View {
        HStack {
            Button(action: onBack) {
                Image(systemName: "chevron.left").font(.system(size: 12, weight: .medium)).foregroundStyle(c.textSecondary)
                    .padding(6)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            Spacer()
            HStack(spacing: Space.xs) {
                Text("Timeline").font(.system(size: 12)).foregroundStyle(c.textPrimary)
                if let range = dateRangeLabel {
                    Text("·").foregroundStyle(c.textDisabled)
                    Text(range).font(.system(size: 12)).foregroundStyle(c.textSecondary)
                }
            }
            Spacer()
            Color.clear.frame(width: 24)
        }
        .padding(.horizontal, Space.md)
        .frame(height: 40)
        .background(c.bgSurface)
        .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: Space.xl) {
            HStack(spacing: Space.sm) {
                Circle().strokeBorder(c.accentDefault, lineWidth: 1.4).frame(width: 20, height: 20)
                    .overlay(Circle().strokeBorder(c.accentHover, lineWidth: 1.4).frame(width: 11, height: 11))
                Text("Iris").font(Typography.serif(15, weight: .semibold)).foregroundStyle(c.textPrimary)
            }

            Button {
                showCritical.toggle()
            } label: {
                HStack(spacing: Space.sm) {
                    Capsule().fill(showCritical ? c.accentDefault : c.borderDefault)
                        .frame(width: 32, height: 18)
                        .overlay(Circle().fill(c.bgSurfaceRaised).frame(width: 14, height: 14).padding(2), alignment: showCritical ? .trailing : .leading)
                    Text("Highlight critical path").font(.system(size: 13)).foregroundStyle(c.textPrimary)
                }
                .padding(Space.sm)
                .background(c.bgHover)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            VStack(alignment: .leading, spacing: Space.xs) {
                Text("LEGEND").font(Typography.caption()).foregroundStyle(c.textSecondary)
                legendRow(shape: AnyView(diamond(c.accentDefault)), label: "Milestone")
                legendRow(shape: AnyView(Text("\u{2192}").font(.system(size: 13)).foregroundStyle(c.accentDefault)), label: "Dependency")
                legendRow(shape: AnyView(Rectangle().fill(c.danger).frame(width: 2, height: 12)), label: "Today")
            }

            Spacer()
        }
        .padding(Space.md)
        .frame(width: 220)
        .background(c.bgSurface)
    }

    private func legendRow(shape: AnyView, label: String) -> some View {
        HStack(spacing: Space.sm) {
            shape.frame(width: 10, alignment: .center)
            Text(label).font(.system(size: 12)).foregroundStyle(c.textSecondary)
        }
    }

    private func diamond(_ color: Color) -> some View {
        Rectangle().fill(color).frame(width: 8, height: 8).rotationEffect(.degrees(45))
    }

    private var chart: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Projects & Epics").font(Typography.h1()).foregroundStyle(c.textPrimary).padding(.bottom, Space.md)

            if rows.isEmpty {
                Text("No projects have both a start date and a target date set yet — the timeline needs both to draw a bar.")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textSecondary)
                    .frame(maxWidth: 400)
            } else {
                VStack(alignment: .leading, spacing: 0) {
                    weekHeader
                    ZStack(alignment: .topLeading) {
                        VStack(alignment: .leading, spacing: 0) {
                            ForEach(rows) { row in rowView(row) }
                        }
                        if let todayX {
                            Rectangle().fill(c.danger).frame(width: 2)
                                .frame(height: CGFloat(rows.count) * 52)
                                .offset(x: Self.nameColWidth + todayX)
                        }
                    }
                }
                .padding(Space.md)
                .background(c.bgSurface)
                .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
                .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
            }
        }
    }

    private var weekHeader: some View {
        HStack(spacing: 0) {
            Color.clear.frame(width: Self.nameColWidth)
            ForEach(weekLabels, id: \.offset) { week in
                Text(week.label)
                    .font(.system(size: 11))
                    .foregroundStyle(c.textSecondary)
                    .frame(width: 7 * Self.dayWidth, alignment: .leading)
                    .overlay(alignment: .leading) { Rectangle().fill(c.borderDefault).frame(width: 1) }
                    .padding(.leading, 4)
            }
        }
        .padding(.bottom, Space.sm)
    }

    private func rowView(_ row: Row) -> some View {
        let barX = CGFloat(daysFrom(minDate, to: row.start)) * Self.dayWidth
        let barW = max(CGFloat(daysFrom(row.start, to: row.end)) * Self.dayWidth, 4)
        let isDim = showCritical && !row.isCritical
        let barColor = showCritical && row.isCritical ? c.accentDefault : c.textDisabled
        let barBg = showCritical && row.isCritical ? c.accentDefault : c.bgHover

        return HStack(spacing: 0) {
            Button {
                onOpenNode(row.project)
            } label: {
                Text(row.title)
                    .font(.system(size: 13))
                    .foregroundStyle(c.textPrimary)
                    .lineLimit(1)
                    .frame(width: Self.nameColWidth, alignment: .leading)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            ZStack(alignment: .topLeading) {
                RoundedRectangle(cornerRadius: 6).fill(barBg).opacity(isDim ? 0.55 : 1)
                    .frame(width: barW, height: 20)
                    .offset(x: barX, y: 16)

                if row.hasDependent {
                    Text("\u{2192}").font(.system(size: 16)).foregroundStyle(barColor)
                        .offset(x: barX + barW + 2, y: 12)
                }

                ForEach(Array(row.milestones.enumerated()), id: \.offset) { _, milestone in
                    let mx = CGFloat(daysFrom(minDate, to: milestone.date)) * Self.dayWidth
                    diamond(barColor).offset(x: mx - 4, y: 21)
                    Text(milestone.label).font(.system(size: 11)).foregroundStyle(c.textSecondary)
                        .fixedSize()
                        .offset(x: mx + 10, y: 18)
                }
            }
            .frame(height: 52, alignment: .topLeading)
        }
        .frame(height: 52)
    }

    // MARK: - Date math

    private var minDate: Date { rows.map(\.start).min() ?? Date() }
    private var maxDate: Date { max(rows.map(\.end).max() ?? Date(), Date()) }

    private var weekLabels: [(offset: Int, label: String)] {
        let totalDays = max(daysFrom(minDate, to: maxDate), 7)
        let weekCount = (totalDays / 7) + 2
        let formatter = DateFormatter()
        formatter.dateFormat = "MMM d"
        return (0..<weekCount).map { i in
            let date = Calendar.current.date(byAdding: .day, value: i * 7, to: minDate) ?? minDate
            return (i, formatter.string(from: date))
        }
    }

    private var todayX: CGFloat? {
        let today = Date()
        guard today >= minDate else { return nil }
        return CGFloat(daysFrom(minDate, to: today)) * Self.dayWidth
    }

    private var dateRangeLabel: String? {
        guard !rows.isEmpty else { return nil }
        let formatter = DateFormatter()
        formatter.dateFormat = "MMM"
        return "\(formatter.string(from: minDate)) \u{2013} \(formatter.string(from: maxDate))"
    }

    private func daysFrom(_ a: Date, to b: Date) -> Int {
        Calendar.current.dateComponents([.day], from: Calendar.current.startOfDay(for: a), to: Calendar.current.startOfDay(for: b)).day ?? 0
    }

    // MARK: - Data

    private func load() {
        let projects = (try? engine.search(query: "", nodeType: "project", domain: nil, tag: nil)) ?? []
        let events = (try? engine.search(query: "", nodeType: "event", domain: nil, tag: nil)) ?? []
        let isoFormatter: () -> DateFormatter = {
            let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd"; return f
        }
        let rfc3339 = ISO8601DateFormatter()

        var built: [Row] = []
        for project in projects {
            guard let full = try? engine.readNode(relPath: project.path).node,
                  let startStr = full.startDate, let endStr = full.targetDate,
                  let start = isoFormatter().date(from: startStr),
                  let end = isoFormatter().date(from: endStr) else { continue }

            let milestones = events
                .filter { $0.parentProject == project.id }
                .compactMap { event -> (Date, String)? in
                    guard let startAt = event.eventStart, let date = rfc3339.date(from: startAt) else { return nil }
                    return (date, taskTitleFor(event))
                }

            built.append(Row(project: project, title: taskTitleFor(project), start: start, end: end, milestones: milestones))
        }
        built.sort { $0.start < $1.start }

        // Critical-path pass: depends-on edges between shown projects only.
        var forwardChildren: [String: [String]] = [:] // prereq id -> [dependent ids]
        for row in built {
            guard let conns = try? engine.connections(nodeId: row.project.id) else { continue }
            for conn in conns where conn.relType == "depends-on" && conn.direction == .outgoing {
                if built.contains(where: { $0.id == conn.node.id }) {
                    forwardChildren[conn.node.id, default: []].append(row.id)
                }
            }
        }
        let endById = Dictionary(uniqueKeysWithValues: built.map { ($0.id, $0.end) })
        var memo: [String: Date] = [:]
        func forwardReach(_ id: String, visiting: Set<String> = []) -> Date {
            if let cached = memo[id] { return cached }
            guard !visiting.contains(id) else { return endById[id] ?? .distantPast }
            var reach = endById[id] ?? .distantPast
            for child in forwardChildren[id] ?? [] {
                reach = max(reach, forwardReach(child, visiting: visiting.union([id])))
            }
            memo[id] = reach
            return reach
        }
        let globalMax = built.map { forwardReach($0.id) }.max() ?? .distantPast
        for i in built.indices {
            built[i].isCritical = forwardReach(built[i].id) == globalMax
            built[i].hasDependent = !(forwardChildren[built[i].id]?.isEmpty ?? true)
        }

        rows = built
    }
}
