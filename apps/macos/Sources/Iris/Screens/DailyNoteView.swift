import Foundation
import SwiftUI
import IrisCore

/// A calendar-day journal lens. Its timeline is a live read over node
/// creation timestamps, not a write into one Daily Note markdown file.
struct DailyNoteView: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var captures: [CachedNode] = []

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.lg) {
                    Text(Self.titleFormatter.string(from: Date())).font(Typography.h1()).foregroundStyle(c.textPrimary)
                    Text("\(captures.count) capture\(captures.count == 1 ? "" : "s") today").font(Typography.bodySans()).foregroundStyle(c.textSecondary)
                    HStack(spacing: Space.sm) {
                        Image(systemName: "plus").foregroundStyle(c.textSecondary)
                        Text("Capture anything — use ⌘⇧C to save it to Today.").font(Typography.bodySans()).foregroundStyle(c.textDisabled)
                    }.padding(Space.md).overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
                    ForEach(captures, id: \.id) { node in row(node) }
                    if captures.isEmpty { Text("No captures yet today.").font(Typography.bodySmall()).foregroundStyle(c.textDisabled).padding(.top, Space.xl) }
                }.padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl)).frame(maxWidth: .infinity, alignment: .leading)
            }
            Divider().background(c.borderDefault)
            VStack(alignment: .leading, spacing: Space.lg) {
                Text("UNSORTED").font(Typography.caption()).foregroundStyle(c.textSecondary)
                Text("Captures stay here until you file them. Nothing forces a decision at capture time.").font(Typography.bodySans()).foregroundStyle(c.textSecondary).lineSpacing(4)
                Divider().background(c.borderDefault)
                Text("LIVE TIMELINE").font(Typography.caption()).foregroundStyle(c.textSecondary)
                Text("A derived feed of every node created today, not a second write into the Daily Note.").font(Typography.bodySans()).foregroundStyle(c.textSecondary).lineSpacing(4)
                Spacer()
            }.padding(Space.lg).frame(width: 260).background(c.bgSurface)
        }.background(c.bgCanvas).task { captures = (try? engine.dailyCaptures(day: Self.dayFormatter.string(from: Date()))) ?? [] }
    }
    private func row(_ node: CachedNode) -> some View {
        HStack(alignment: .top, spacing: Space.md) {
            Text(time(node.created)).font(Typography.caption()).foregroundStyle(c.textDisabled).frame(width: 42, alignment: .leading)
            Image(systemName: node.nodeType == "task" ? "checkmark.square" : "doc").font(.system(size: 12)).foregroundStyle(c.accentDefault).frame(width: 14)
            Text((node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")).font(Typography.serif(16)).foregroundStyle(c.textPrimary)
            Spacer()
        }.padding(.vertical, Space.sm)
    }
    private func time(_ raw: String) -> String { ISO8601DateFormatter().date(from: raw).map { Self.timeFormatter.string(from: $0) } ?? "" }
    private static let dayFormatter: DateFormatter = { let f = DateFormatter(); f.calendar = Calendar(identifier: .iso8601); f.locale = Locale(identifier: "en_US_POSIX"); f.dateFormat = "yyyy-MM-dd"; return f }()
    private static let titleFormatter: DateFormatter = { let f = DateFormatter(); f.dateStyle = .full; return f }()
    private static let timeFormatter: DateFormatter = { let f = DateFormatter(); f.dateFormat = "h:mma"; return f }()
}
