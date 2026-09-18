import Foundation
import SwiftUI
import IrisCore

/// Read-only Week calendar over canonical event nodes. Calendar sync,
/// capacity, and drag-to-schedule require their later planning substrate.
struct CalendarView: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var weekStart = Self.weekStart(for: Date())
    @State private var events: [CalendarEvent] = []
    @State private var unscheduled: [CachedNode] = []

    var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider().background(c.borderDefault)
            VStack(alignment: .leading, spacing: Space.lg) {
                header
                weekGrid
                Text("Events come from canonical `event` nodes. Scheduling, capacity, and external-calendar sync are not configured yet.")
                    .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xl, bottom: Space.xl, trailing: Space.xl))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("UNSCHEDULED").font(Typography.caption()).foregroundStyle(c.textSecondary)
            if unscheduled.isEmpty {
                Text("No unscheduled tasks.").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            } else {
                ForEach(unscheduled.prefix(5), id: \.id) { task in
                    Text(title(for: task)).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                }
            }
            Text("Select a task, then choose an open time once planning capacity is available.")
                .font(Typography.bodySmall()).foregroundStyle(c.textSecondary).padding(.top, Space.md)
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 220, alignment: .topLeading)
        .background(c.bgSurface)
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text("This Week").font(Typography.h1()).foregroundStyle(c.textPrimary)
                Text(weekRange).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            Spacer()
            Button { weekStart = Calendar.current.date(byAdding: .day, value: -7, to: weekStart) ?? weekStart } label: { Image(systemName: "chevron.left") }
            Button { weekStart = Self.weekStart(for: Date()) } label: { Text("Today").font(Typography.bodySmall()) }
            Button { weekStart = Calendar.current.date(byAdding: .day, value: 7, to: weekStart) ?? weekStart } label: { Image(systemName: "chevron.right") }
        }
        .buttonStyle(.bordered)
    }

    private var weekGrid: some View {
        let days = (0..<7).compactMap { Calendar.current.date(byAdding: .day, value: $0, to: weekStart) }
        return HStack(spacing: 0) {
            ForEach(days, id: \.self) { day in
                VStack(alignment: .leading, spacing: Space.sm) {
                    VStack(alignment: .leading, spacing: Space.xxs) {
                        Text(day.formatted(.dateTime.weekday(.narrow))).font(Typography.caption()).foregroundStyle(c.textSecondary)
                        Text(day.formatted(.dateTime.day())).font(Typography.sans(15, weight: .semibold)).foregroundStyle(c.textPrimary)
                    }
                    .padding(.horizontal, Space.sm).padding(.vertical, Space.md)
                    Divider().background(c.borderDefault)
                    ForEach(events(on: day)) { event in eventCard(event) }
                    Spacer(minLength: 260)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .overlay(alignment: .trailing) { Divider().background(c.borderDefault) }
            }
        }
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
    }

    private func eventCard(_ event: CalendarEvent) -> some View {
        VStack(alignment: .leading, spacing: Space.xxs) {
            Text(event.title).font(Typography.bodySmall()).foregroundStyle(c.textPrimary).lineLimit(2)
            if let start = event.start { Text(start.formatted(date: .omitted, time: .shortened)).font(Typography.caption()).foregroundStyle(c.textSecondary) }
        }
        .padding(Space.sm)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(c.accentTint)
        .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
        .padding(.horizontal, Space.xs)
        .accessibilityLabel("\(event.title), \(event.start?.formatted(date: .abbreviated, time: .shortened) ?? "time not recorded")")
    }

    private func events(on day: Date) -> [CalendarEvent] { events.filter { $0.start.map { Calendar.current.isDate($0, inSameDayAs: day) } ?? false } }
    private var weekRange: String {
        let end = Calendar.current.date(byAdding: .day, value: 6, to: weekStart) ?? weekStart
        return "\(weekStart.formatted(.dateTime.month(.abbreviated).day())) – \(end.formatted(.dateTime.month(.abbreviated).day()))"
    }
    private func load() {
        let nodes = (try? engine.search(query: "", nodeType: "event", domain: nil, tag: nil)) ?? []
        events = nodes.compactMap { cached in
            guard let node = try? engine.readNode(relPath: cached.path) else { return nil }
            return CalendarEvent(id: cached.id, title: title(for: cached), start: node.node.start.flatMap(Self.parse))
        }
        unscheduled = (try? engine.somedayMaybe()) ?? []
    }
    private func title(for node: CachedNode) -> String { (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "") }
    private static func parse(_ value: String) -> Date? { ISO8601DateFormatter().date(from: value) }
    private static func weekStart(for date: Date) -> Date { Calendar.current.dateInterval(of: .weekOfYear, for: date)?.start ?? date }

    private struct CalendarEvent: Identifiable { let id: String; let title: String; let start: Date? }
}
