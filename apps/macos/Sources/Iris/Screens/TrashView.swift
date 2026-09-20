import Foundation
import SwiftUI
import IrisCore

/// Recovery view over the canonical soft-delete substrate. Restoring is an
/// in-place action: `restoreNode` clears `deleted_at`, so the row leaves this
/// derived list without routing the user away from Trash.
struct TrashView: View {
    let engine: FfiEngine

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var nodes: [CachedNode] = []
    @State private var errorMessage: String?

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    header
                    if nodes.isEmpty {
                        emptyState
                    } else {
                        ForEach(nodes, id: \.id) { node in
                            trashRow(node)
                        }
                    }
                    if let errorMessage {
                        Text(errorMessage)
                            .font(Typography.bodySmall())
                            .foregroundStyle(c.danger)
                            .padding(.top, Space.md)
                    }
                }
                .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider().background(c.borderDefault)
            aboutRail
        }
        .background(c.bgCanvas)
        .task { reload() }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: Space.xs) {
                Text("Trash").font(Typography.h1()).foregroundStyle(c.textPrimary)
                Text("Soft-deleted nodes, any type, newest first. Items are permanently removed after 30 days — every prior state stays recoverable from git history after that.")
                    .font(Typography.bodySans())
                    .foregroundStyle(c.textSecondary)
                    .frame(maxWidth: 680, alignment: .leading)
            }
            Spacer()
            Text("\(nodes.count) item\(nodes.count == 1 ? "" : "s")")
                .font(Typography.bodySans())
                .foregroundStyle(c.textSecondary)
        }
        .padding(.bottom, Space.xl)
    }

    private func trashRow(_ node: CachedNode) -> some View {
        HStack(spacing: Space.md) {
            Circle().fill(dotColor(for: node.nodeType)).frame(width: 10, height: 10)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title(for: node)).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                Text(deletedDescription(for: node))
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textSecondary)
            }
            Spacer()
            typePill(node.nodeType)
            Button("Restore") { restore(node) }
                .buttonStyle(.plain)
                .font(Typography.bodySans())
                .foregroundStyle(c.accentDefault)
                .padding(.vertical, Space.xs)
                .contentShape(Rectangle())
        }
        .padding(.vertical, Space.md)
        .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Image(systemName: "trash").font(.system(size: 22)).foregroundStyle(c.textDisabled)
            Text("Trash is empty").font(Typography.bodySans()).foregroundStyle(c.textPrimary)
            Text("Deleted nodes remain here for 30 days before their files are purged.")
                .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
        .padding(.vertical, Space.xxxl)
    }

    private var aboutRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            railSection("About Trash") {
                Text("Restoring is in-place — the row just leaves this list, you’re not taken anywhere. Permanently deleting uses the system’s own confirmation dialog, not an Iris one.")
            }
            Divider().background(c.borderDefault)
            railSection("Trashed ≠ archived") {
                Text("Archived items stay visible in their normal views, just marked inactive. Trashed items are hidden everywhere until restored.")
            }
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 300)
        .background(c.bgSurface)
    }

    private func railSection(_ title: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            content().font(Typography.bodySans()).foregroundStyle(c.textSecondary).lineSpacing(4)
        }
    }

    private func typePill(_ type: String) -> some View {
        Text(type.capitalized)
            .font(Typography.caption())
            .foregroundStyle(c.textSecondary)
            .padding(.horizontal, Space.sm)
            .padding(.vertical, Space.xxs)
            .background(c.bgHover)
            .clipShape(Capsule())
    }

    private func restore(_ node: CachedNode) {
        do {
            try engine.restoreNode(relPath: node.path)
            withAnimation(.easeInOut(duration: 0.12)) {
                nodes.removeAll { $0.id == node.id }
            }
            errorMessage = nil
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func reload() {
        do {
            nodes = try engine.trash()
            errorMessage = nil
        } catch {
            nodes = []
            errorMessage = String(describing: error)
        }
    }

    private func title(for node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func deletedDescription(for node: CachedNode) -> String {
        guard let raw = node.deletedAt, let date = ISO8601DateFormatter().date(from: raw) else {
            return "Deleted recently"
        }
        let days = max(0, Calendar.current.dateComponents([.day], from: date, to: Date()).day ?? 0)
        if days == 0 { return "Deleted today" }
        let remaining = max(0, 30 - days)
        return remaining <= 1 ? "Deleted \(days) days ago · purges in \(remaining) day" : "Deleted \(days) days ago · purges in \(remaining) days"
    }

    private func dotColor(for type: String) -> Color {
        switch type {
        case "project": c.warning
        case "area": c.success
        case "resource": c.accentDefault
        default: c.borderStrong
        }
    }
}
