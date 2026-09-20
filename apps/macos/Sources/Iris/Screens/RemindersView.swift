import Foundation
import SwiftUI
import UserNotifications
import IrisCore

/// Manual reminders over canonical `reminder` nodes. Delivery remains an OS
/// concern: Iris asks permission and schedules local notifications, never
/// draws an imitation notification or claims one fired without OS evidence.
struct RemindersView: View {
    let engine: FfiEngine

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var reminders: [Reminder] = []
    @State private var showNewReminder = false
    @State private var draftText = ""
    @State private var draftDate = Date().addingTimeInterval(60 * 60)

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: Space.xl) {
                TaskLensHeader(
                    title: "Reminders",
                    trailing: "\(upcoming.count) upcoming",
                    description: "User-authored only — Iris never creates one for you. A due date or event time is an in-app indicator, not a notification, until you turn it into a reminder."
                )

                Button("+ New reminder") { showNewReminder = true }
                    .buttonStyle(.plain)
                    .font(Typography.sans(14, weight: .medium))
                    .foregroundStyle(c.accentDefault)
                    .padding(.vertical, Space.xs)
                    .contentShape(Rectangle())

                section("UPCOMING", items: upcoming, empty: "No reminders are scheduled. Add one when you want Iris to ask the OS to notify you.")
                if !past.isEmpty { section("PAST — NOT CONFIRMED DELIVERED", items: past, empty: "") }
                if !recent.isEmpty { section("RECENT", items: recent, empty: "") }
                Spacer()
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

            Divider().background(c.borderDefault)
            rail
        }
        .background(c.bgCanvas)
        .task { load() }
        .sheet(isPresented: $showNewReminder) { newReminderSheet }
    }

    private func section(_ title: String, items: [Reminder], empty: String) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text(title).font(Typography.caption()).foregroundStyle(c.textSecondary)
            if items.isEmpty {
                Text(empty).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            } else {
                ForEach(items) { reminder in row(reminder) }
            }
        }
    }

    private func row(_ reminder: Reminder) -> some View {
        let (icon, iconColor, iconTint) = reminder.isRecent
            ? ("checkmark", c.success, c.successTint)
            : reminder.isPast
                ? ("questionmark", c.warning, c.warningTint)
                : ("clock", c.accentDefault, c.accentTint)
        return HStack(spacing: Space.md) {
            Image(systemName: icon)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(iconColor)
                .frame(width: 24, height: 24)
                .background(iconTint)
                .clipShape(Circle())
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(reminder.title)
                    .font(Typography.bodySans())
                    .foregroundStyle(reminder.isRecent ? c.textDisabled : c.textPrimary)
                    .strikethrough(reminder.isRecent)
                Text(reminder.detail)
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textSecondary)
            }
            Spacer()
        }
        .padding(.vertical, Space.xs)
        .accessibilityElement(children: .combine)
    }

    private var rail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            Text("HOW DELIVERY WORKS").font(Typography.caption()).foregroundStyle(c.textSecondary)
            Text("Reminders fire as native OS notifications — not an in-app popup. Iris never picks the time or content for you; it only schedules what you wrote.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textPrimary)
                .fixedSize(horizontal: false, vertical: true)
            Divider().background(c.borderDefault)
            Text("THIS WEEK").font(Typography.caption()).foregroundStyle(c.textSecondary)
            metric("Upcoming", upcoming.count)
            metric("Past, unconfirmed", past.count)
            metric("Fired or dismissed", recent.count)
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 264, alignment: .topLeading)
        .background(c.bgSurface)
    }

    private func metric(_ label: String, _ value: Int) -> some View {
        HStack {
            Text(label).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
            Spacer()
            Text("\(value)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }

    private var newReminderSheet: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            Text("New reminder").font(Typography.h1()).foregroundStyle(c.textPrimary)
            TextField("What should Iris remind you about?", text: $draftText)
                .textFieldStyle(.roundedBorder)
            DatePicker("Notify me", selection: $draftDate, displayedComponents: [.date, .hourAndMinute])
            HStack {
                Spacer()
                Button("Cancel") { showNewReminder = false }
                Button("Create") { createReminder() }
                    .buttonStyle(.borderedProminent)
                    .disabled(draftText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(Space.xl)
        .frame(width: 440)
        .background(c.bgSurfaceRaised)
    }

    private var upcoming: [Reminder] { reminders.filter { !$0.isRecent && $0.isUpcoming } }
    /// Scheduled, its fire time has passed (or no fire time was ever
    /// recorded), but nothing ever transitioned its status to "fired"/
    /// "dismissed" — this app never claims delivery without OS evidence
    /// (see the file's own doc comment), so these aren't shown as
    /// "recent" either. Without this bucket a reminder would silently
    /// vanish from every section the moment its time passed, even though
    /// the node still exists.
    private var past: [Reminder] { reminders.filter { !$0.isRecent && !$0.isUpcoming } }
    private var recent: [Reminder] { reminders.filter(\.isRecent) }

    private func load() {
        let nodes = (try? engine.search(query: "", nodeType: "reminder", domain: nil, tag: nil)) ?? []
        reminders = nodes.compactMap { cached in
            guard let parsed = try? engine.readNode(relPath: cached.path) else { return nil }
            return Reminder(id: cached.id, path: cached.path, node: parsed.node)
        }.sorted { ($0.fireDate ?? .distantFuture) < ($1.fireDate ?? .distantFuture) }
        Task { for reminder in upcoming { await ReminderDelivery.schedule(id: reminder.id, title: reminder.title, fireDate: reminder.fireDate, isRecent: reminder.isRecent) } }
    }

    private func createReminder() {
        let text = draftText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        let id = newNodeId()
        let now = ISO8601DateFormatter().string(from: Date())
        let fireAt = ISO8601DateFormatter().string(from: draftDate)
        let node = FfiNode(
            id: id, nodeType: .reminder, created: now, modified: now, schemaVersion: currentSchemaVersion(),
            lifecycle: nil, archivedAt: nil, domain: nil, tags: [], relations: [], deletedAt: nil, isTemplate: false,
            distillationLevel: nil, status: nil, priority: nil, scheduledDate: nil, dueDate: nil, estimatedPomodoros: nil,
            actualPomodoros: nil, recurrence: nil, recurrenceOccurrences: nil, checklist: [], start: nil, end: nil,
            externalId: nil, projectStatus: nil, startDate: nil, targetDate: nil, sourceUrl: nil, readStatus: nil,
            reminderText: text, fireAt: fireAt, reminderStatus: "scheduled", resolved: false, anchor: nil, pinned: [],
            activeFilter: nil, defaultView: nil, theme: nil, inkAttachment: nil, date: nil, symbol: nil, entry: nil,
            exit: nil, pnl: nil, rMultiple: nil
        )
        do {
            let path = "reminders/\(id.lowercased()).md"
            try engine.createNode(relPath: path, node: node, body: "\n\(text)\n")
            let reminder = Reminder(id: id, path: path, node: node)
            reminders.append(reminder)
            reminders.sort { ($0.fireDate ?? .distantFuture) < ($1.fireDate ?? .distantFuture) }
            Task { await ReminderDelivery.schedule(id: reminder.id, title: reminder.title, fireDate: reminder.fireDate, isRecent: reminder.isRecent) }
            draftText = ""
            showNewReminder = false
        } catch { }
    }

    private struct Reminder: Identifiable {
        let id: String
        let path: String
        let node: FfiNode
        var fireDate: Date? { node.fireAt.flatMap { ISO8601DateFormatter().date(from: $0) } }
        var isRecent: Bool { ["fired", "dismissed"].contains((node.reminderStatus ?? "").lowercased()) }
        /// Still scheduled (not fired/dismissed) with a real fire time at or
        /// after now. A reminder with no recorded fire time, or whose fire
        /// time has already passed, is `isPast` instead — never both, and
        /// never neither, so every non-recent reminder lands in exactly
        /// one of `upcoming`/`past`.
        var isUpcoming: Bool { fireDate.map { $0 >= Date() } ?? false }
        var isPast: Bool { !isRecent && !isUpcoming }
        var title: String { node.reminderText ?? (path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "") }
        var detail: String {
            guard let fireDate else { return "No delivery time recorded" }
            if isRecent { return "\(node.reminderStatus?.capitalized ?? "Handled") · \(fireDate.formatted(date: .abbreviated, time: .shortened))" }
            if isPast { return "Was due \(fireDate.formatted(date: .abbreviated, time: .shortened)) — not confirmed delivered" }
            return fireDate.formatted(date: .abbreviated, time: .shortened)
        }
    }
}

private enum ReminderDelivery {
    static func schedule(id: String, title: String, fireDate: Date?, isRecent: Bool) async {
        guard let fireDate, fireDate > Date(), !isRecent else { return }
        let center = UNUserNotificationCenter.current()
        guard (try? await center.requestAuthorization(options: [.alert, .sound])) == true else { return }
        let content = UNMutableNotificationContent()
        content.title = "Iris reminder"
        content.body = title
        content.sound = .default
        let trigger = UNCalendarNotificationTrigger(dateMatching: Calendar.current.dateComponents([.year, .month, .day, .hour, .minute], from: fireDate), repeats: false)
        try? await center.add(UNNotificationRequest(identifier: "iris.reminder.\(id)", content: content, trigger: trigger))
    }
}
