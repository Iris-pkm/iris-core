import SwiftUI
import IrisCore

/// Saved-lens manager. Applying a Space changes the shell's local context;
/// its persisted node fields remain the canonical description of that lens.
struct SpacesView: View {
    let engine: FfiEngine
    @Binding var activeSpaceID: String?
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var spaces: [CachedNode] = []
    @State private var details: [String: FfiNode] = [:]
    @State private var errorMessage: String?

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.xl) {
                    header
                    if spaces.isEmpty { emptyState } else {
                        ForEach(spaces, id: \.id) { space in row(space) }
                    }
                    if let errorMessage { Text(errorMessage).font(Typography.bodySmall()).foregroundStyle(c.danger) }
                }
                .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            Divider().background(c.borderDefault)
            aboutRail
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Space.xs) {
            Text("Spaces").font(Typography.h1()).foregroundStyle(c.textPrimary)
            Text("A Space is a saved lens — pinned nodes, an active filter, a default view, a theme accent — not a PARA container. Switching one is an instant context change within this vault.")
                .font(Typography.bodySans()).foregroundStyle(c.textSecondary).frame(maxWidth: 760, alignment: .leading)
        }
    }

    private func row(_ space: CachedNode) -> some View {
        let node = details[space.id]
        let current = activeSpaceID == space.id
        return HStack(spacing: Space.md) {
            RoundedRectangle(cornerRadius: Radius.sm).fill(color(for: node?.theme)).frame(width: 12, height: 48)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title(space)).font(Typography.sans(18, weight: .semibold)).foregroundStyle(c.textPrimary)
                Text(summary(node)).font(Typography.bodySans()).foregroundStyle(c.textSecondary)
            }
            Spacer()
            if current {
                Text("Current").font(Typography.caption()).foregroundStyle(c.success).padding(.horizontal, Space.sm).padding(.vertical, Space.xxs).background(c.successTint).clipShape(Capsule())
            } else {
                Button("Apply") { activeSpaceID = space.id }
                    .buttonStyle(.borderedProminent).tint(c.accentDefault)
            }
        }
        .padding(Space.md).background(current ? c.bgSelected : c.bgSurface)
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(current ? c.accentDefault : c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("No saved Spaces yet").font(Typography.bodySans()).foregroundStyle(c.textPrimary)
            Text("A Space captures a future view context; creating one needs the app-wide context-capture flow, which is not wired yet.")
                .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }.padding(.vertical, Space.xxxl)
    }

    private var aboutRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            section("Active Space", activeSpaceID.flatMap { id in spaces.first(where: { $0.id == id }).map(title) } ?? "None")
            Divider().background(c.borderDefault)
            section("About Spaces", "Modeled as nodes, so saved lenses receive version history, sync, and export without a separate config system. A Space can span projects and domains; it is not a PARA container.")
            Divider().background(c.borderDefault)
            section("Space vs. Vault", "A Space switch changes point of view within one vault. A vault switch opens a different graph and belongs in Onboarding’s recent-vault picker.")
            Spacer()
        }.padding(Space.lg).frame(width: 300).background(c.bgSurface)
    }

    private func section(_ heading: String, _ text: String) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text(heading.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            Text(text).font(Typography.bodySans()).foregroundStyle(c.textSecondary).lineSpacing(4)
        }
    }

    private func load() {
        do {
            spaces = try engine.search(query: "", nodeType: "space", domain: nil, tag: nil)
            details = Dictionary(uniqueKeysWithValues: spaces.compactMap { space in
                (try? engine.readNode(relPath: space.path)).map { (space.id, $0.node) }
            })
            errorMessage = nil
        } catch { errorMessage = String(describing: error) }
    }

    private func title(_ node: CachedNode) -> String { (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "") }
    private func summary(_ node: FfiNode?) -> String {
        let pinned = node?.pinned.count ?? 0
        let filter = node?.activeFilter ?? "no filter"
        let view = node?.defaultView ?? "default view"
        return "\(pinned) pinned · filter: \(filter) · opens on \(view)"
    }
    private func color(for theme: String?) -> Color {
        switch theme { case "gold": c.warning; case "green": c.success; default: c.accentDefault }
    }
}
