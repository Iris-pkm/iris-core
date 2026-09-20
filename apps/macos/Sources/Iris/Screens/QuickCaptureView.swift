import Foundation
import SwiftUI
import IrisCore

/// A lightweight capture overlay (screen-flow kind C). Its one committed
/// action creates a normal task with today's scheduled date, so the existing
/// `views::today` query picks it up without a second write to Daily Note.
struct QuickCaptureView: View {
    let engine: FfiEngine
    let recentCaptures: [CaptureItem]
    let dismiss: () -> Void
    let saved: (CaptureItem) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var text = ""
    @State private var errorMessage: String?
    @FocusState private var inputFocused: Bool

    var body: some View {
        ZStack(alignment: .top) {
            c.bgCanvas.opacity(0.82).ignoresSafeArea()

            VStack {
                capturePanel
                    .padding(.top, 72)
                Spacer()
            }
        }
        .onAppear { inputFocused = true }
        .onExitCommand(perform: dismiss)
    }

    private var capturePanel: some View {
        VStack(spacing: 0) {
            HStack(spacing: Space.md) {
                irisMark
                TextField("Capture anything…", text: $text)
                    .textFieldStyle(.plain)
                    .font(Typography.sans(15))
                    .foregroundStyle(c.textPrimary)
                    .focused($inputFocused)
                    .onSubmit(saveToToday)
                Spacer()
                Image(systemName: "scope")
                    .font(.system(size: 14))
                    .foregroundStyle(c.textSecondary)
            }
            .padding(Space.lg)
            .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }

            VStack(alignment: .leading, spacing: Space.sm) {
                Text("RECENT")
                    .font(Typography.caption())
                    .foregroundStyle(c.textSecondary)

                if recentCaptures.isEmpty {
                    Text("New captures will appear here for this session.")
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.textDisabled)
                        .padding(.vertical, Space.xs)
                } else {
                    ForEach(recentCaptures.prefix(4)) { item in
                        recentRow(item)
                    }
                }

                if let errorMessage {
                    Text(errorMessage)
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.danger)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(Space.lg)
            .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }

            HStack(spacing: Space.sm) {
                keyHint("↵")
                Button("save to Today", action: saveToToday)
                    .buttonStyle(.plain)
                    .font(Typography.bodySmall())
                    .foregroundStyle(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? c.textDisabled : c.textPrimary)
                    .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .padding(.vertical, Space.xs)
                    .contentShape(Rectangle())

                keyHint("⌘↵")
                Text("save & file to project")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textDisabled)
                    .help("Project filing is not designed yet.")

                keyHint("esc")
                Text("dismiss")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textSecondary)
                Spacer()
            }
            .padding(.horizontal, Space.lg)
            .padding(.vertical, Space.md)
        }
        .frame(width: 520)
        .background(c.bgSurfaceRaised)
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .shadow(color: .black.opacity(colorScheme == .dark ? 0.55 : 0.25), radius: 35, y: 20)
    }

    private var irisMark: some View {
        Circle().strokeBorder(c.accentDefault, lineWidth: 1.4).frame(width: 18, height: 18)
            .overlay(Circle().strokeBorder(c.accentHover, lineWidth: 1.4).frame(width: 10, height: 10))
    }

    private func recentRow(_ item: CaptureItem) -> some View {
        HStack(spacing: Space.sm) {
            RoundedRectangle(cornerRadius: 2)
                .stroke(item.color, lineWidth: 1)
                .frame(width: 8, height: 8)
            Text(item.title)
                .font(Typography.bodySans())
                .foregroundStyle(c.textPrimary)
                .lineLimit(1)
            Spacer()
            Text(item.relativeTime)
                .font(Typography.caption())
                .foregroundStyle(c.textDisabled)
        }
    }

    private func keyHint(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(c.textSecondary)
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
            .overlay(RoundedRectangle(cornerRadius: Radius.sm).stroke(c.borderDefault, lineWidth: 1))
    }

    private func saveToToday() {
        let title = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return }

        let id = newNodeId()
        let now = ISO8601DateFormatter().string(from: Date())
        let today = Self.dayFormatter.string(from: Date())
        let node = FfiNode(
            id: id, nodeType: .task, created: now, modified: now,
            schemaVersion: currentSchemaVersion(), lifecycle: nil, archivedAt: nil,
            domain: nil, tags: [], relations: [], deletedAt: nil, isTemplate: false,
            distillationLevel: nil, status: nil, priority: nil, scheduledDate: today,
            dueDate: nil, estimatedPomodoros: nil, actualPomodoros: nil,
            recurrence: nil, recurrenceOccurrences: nil, checklist: [], start: nil,
            end: nil, externalId: nil, projectStatus: nil, startDate: nil,
            targetDate: nil, sourceUrl: nil, readStatus: nil, reminderText: nil,
            fireAt: nil, reminderStatus: nil, resolved: false, anchor: nil,
            pinned: [], activeFilter: nil, defaultView: nil, theme: nil,
            inkAttachment: nil, date: nil, symbol: nil, entry: nil, exit: nil,
            pnl: nil, rMultiple: nil
        )

        do {
            try engine.createNode(relPath: "tasks/capture-\(id.lowercased()).md", node: node, body: "\n\(title)\n")
            saved(CaptureItem(title: title, createdAt: Date(), color: c.accentDefault))
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private static let dayFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .iso8601)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }()
}

struct CaptureItem: Identifiable {
    let id = UUID()
    let title: String
    let createdAt: Date
    let color: Color

    var relativeTime: String {
        let seconds = max(0, Int(Date().timeIntervalSince(createdAt)))
        if seconds < 60 { return "now" }
        if seconds < 3_600 { return "\(seconds / 60)m" }
        return "\(seconds / 3_600)h"
    }
}
