import SwiftUI
import IrisCore

/// `design/canvas/Upcoming.dc.html` — `scheduled_date BETWEEN from AND
/// from+days`, both ends inclusive (`views::upcoming`). `days` has no
/// fixed backend default, so the 3/7/14-day segmented control here *is*
/// that caller-supplied parameter, live — not a client-side filter over a
/// fixed fetch.
struct UpcomingView: View {
    let engine: FfiEngine
    let onOpenNode: (CachedNode) -> Void

    private enum Window: Int, CaseIterable { case three = 3, seven = 7, fourteen = 14 }

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var window: Window = .seven
    @State private var items: [CachedNode] = []
    @State private var projectTitles: [String: String] = [:]

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.md) {
                    TaskLensHeader(
                        title: "Upcoming", trailing: "\(items.count) items",
                        description: "Scheduled within the window below. No overdue catch-all here — that's Today's job."
                    )

                    windowSegmented

                    if items.isEmpty {
                        TaskLensEmptyState(title: "Nothing scheduled", message: "No tasks scheduled in this window. Try a wider one, or check Inbox for unfiled work.")
                    } else {
                        VStack(alignment: .leading, spacing: 0) {
                            ForEach(groupedByDate(), id: \.label) { group in
                                Text(group.label.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary).padding(.top, Space.sm)
                                ForEach(group.rows, id: \.id) { item in
                                    TaskLensRow(
                                        title: taskTitleFor(item), meta: projectLabel(item, titles: projectTitles),
                                        onOpen: { onOpenNode(item) }
                                    )
                                }
                            }
                        }
                    }
                }
                .padding(EdgeInsets(top: Space.xl, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider().background(c.borderDefault)
            TaskLensRail {
                TaskLensRailSection("In this window") {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(items.count)").font(Typography.serif(28, weight: .semibold)).foregroundStyle(c.textPrimary)
                        Text("Next \(window.rawValue) days").font(Typography.caption()).foregroundStyle(c.textSecondary)
                    }
                }
                TaskLensRailDivider()
                TaskLensRailSection("About Upcoming") {
                    Text("`days` is a caller-supplied parameter, not a fixed default — this window control is that parameter, live.")
                        .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }
            }
        }
        .background(c.bgCanvas)
        .task(id: window) { load() }
    }

    private var windowSegmented: some View {
        HStack(spacing: 2) {
            ForEach(Window.allCases, id: \.self) { w in
                Button {
                    window = w
                } label: {
                    Text("\(w.rawValue) days")
                        .font(Typography.bodySmall())
                        .foregroundStyle(window == w ? c.textPrimary : c.textSecondary)
                        .padding(.horizontal, Space.md)
                        .padding(.vertical, Space.xs)
                        .background(window == w ? c.bgSurfaceRaised : Color.clear)
                        .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .padding(2)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private struct DateGroup { let label: String; let rows: [CachedNode] }

    /// Backend order is `scheduled_date, id` already — grouping here is
    /// purely presentational over that real order, not a re-sort.
    private func groupedByDate() -> [DateGroup] {
        var groups: [(String, [CachedNode])] = []
        for item in items {
            let label = dayLabel(item.scheduledDate)
            if let idx = groups.firstIndex(where: { $0.0 == label }) {
                groups[idx].1.append(item)
            } else {
                groups.append((label, [item]))
            }
        }
        return groups.map { DateGroup(label: $0.0, rows: $0.1) }
    }

    private func dayLabel(_ iso: String?) -> String {
        guard let iso, let date = isoFormatter().date(from: iso) else { return "No date" }
        let calendar = Calendar.current
        if calendar.isDateInToday(date) { return "Today" }
        if calendar.isDateInTomorrow(date) { return "Tomorrow" }
        let formatter = DateFormatter()
        formatter.dateFormat = "EEE, MMM d"
        return formatter.string(from: date)
    }

    private func isoFormatter() -> DateFormatter {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }

    private func load() {
        let formatter = isoFormatter()
        let from = formatter.string(from: Date())
        items = (try? engine.upcoming(from: from, days: UInt32(window.rawValue))) ?? []
        projectTitles = projectTitleMap(engine: engine)
    }
}
