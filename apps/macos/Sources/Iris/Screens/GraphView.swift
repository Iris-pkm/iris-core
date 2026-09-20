import SwiftUI
import IrisCore

/// `design/canvas/Graph.dc.html` (Phase 6, pulled forward at the user's
/// explicit request — `navigation.md` otherwise defers this). A radial
/// graph centered on whichever node was open when "Open graph" (⌘G) was
/// invoked, real first/second-degree connections via the existing
/// `connections` FFI call (client-side BFS — no new backend surface
/// needed, the same generic relation lookup `ConnectionsPanel` already
/// uses), and a real Aperture slider switching between 1-hop ("Focused")
/// and 2-hop ("Wide").
///
/// **Full replacement, not a panel** — `screen-flow.md`'s own note: "Graph
/// is a genuinely separate mode (Phase 6), not a PARA sibling; treat as
/// full replace regardless." Rendered as a top-level overlay covering the
/// entire `AppShell` (sidebar included), matching the mockup's own bespoke
/// chrome rather than reusing the standard PARA sidebar.
///
/// **Honestly flagged scope:**
/// - Layout is a simplified radial placement (even angular spacing per
///   ring), not a real physics-based force-directed graph — same
///   documented ceiling `ConnectionsPanel`'s own radial glyph already
///   carries, just applied at a larger scale.
/// - The mockup's left icon rail has two more icons (a list glyph, a grid
///   glyph) with no defined destination anywhere else in the app — only
///   the graph icon (this screen) has a real meaning, so the other two
///   aren't rendered rather than inventing screens that don't exist.
/// - The mockup's legend hard-codes "Archive" as the fourth category, but
///   that's a lifecycle state (ADR-016), not a node type — a task or note
///   isn't "archived" just because it isn't a Project/Area/Resource.
///   Relabeled "Other" here instead of reproducing a mismatched claim.
struct GraphView: View {
    let engine: FfiEngine
    let centerNode: CachedNode
    let onOpenNode: (CachedNode) -> Void
    let onBack: () -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private struct Placed: Identifiable {
        let node: CachedNode
        let angle: Double
        let radius: CGFloat
        var id: String { node.id }
    }

    @State private var firstDegree: [CachedNode] = []
    @State private var secondDegree: [String: [CachedNode]] = [:] // keyed by first-degree parent id
    @State private var aperture: Double = 0.44

    private var isWide: Bool { aperture >= 0.5 }

    var body: some View {
        VStack(spacing: 0) {
            topBar
            HStack(spacing: 0) {
                iconRail
                ZStack {
                    canvas
                    legend
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            bottomBar
        }
        .background(c.bgCanvas)
        .task(id: centerNode.id) { load() }
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
                Text(titleFor(centerNode)).font(.system(size: 12)).foregroundStyle(c.textSecondary)
                Text("/").font(.system(size: 12)).foregroundStyle(c.textDisabled)
                Text("Graph").font(.system(size: 12)).foregroundStyle(c.textPrimary)
            }
            Spacer()
            Color.clear.frame(width: 24)
        }
        .padding(.horizontal, Space.md)
        .frame(height: 40)
        .background(c.bgSurface)
        .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
    }

    private var iconRail: some View {
        VStack(spacing: Space.lg) {
            Circle().strokeBorder(c.accentDefault, lineWidth: 1.4).frame(width: 20, height: 20)
                .overlay(Circle().strokeBorder(c.accentHover, lineWidth: 1.4).frame(width: 11, height: 11))
            Rectangle().fill(c.borderDefault).frame(width: 28, height: 1)
            Image(systemName: "circle.grid.cross")
                .font(.system(size: 14))
                .foregroundStyle(c.accentDefault)
                .frame(width: 32, height: 32)
                .background(c.bgSelected)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
            Spacer()
        }
        .padding(.vertical, Space.lg)
        .frame(width: 52)
        .background(c.bgSurface)
        .overlay(alignment: .trailing) { Rectangle().fill(c.borderDefault).frame(width: 1) }
    }

    private var canvas: some View {
        GeometryReader { geo in
            let center = CGPoint(x: geo.size.width / 2, y: geo.size.height / 2)
            let ring1: CGFloat = min(geo.size.width, geo.size.height) * 0.22
            let ring2: CGFloat = min(geo.size.width, geo.size.height) * 0.40

            ZStack {
                Circle().stroke(c.borderDefault, lineWidth: 1).frame(width: ring1 * 2, height: ring1 * 2).position(center)
                if isWide {
                    Circle().stroke(c.borderDefault, lineWidth: 1).frame(width: ring2 * 2, height: ring2 * 2).position(center)
                }

                ForEach(placedFirstDegree(ring: ring1), id: \.id) { placed in
                    let point = CGPoint(x: center.x + cos(placed.angle) * placed.radius, y: center.y + sin(placed.angle) * placed.radius)
                    Path { path in path.move(to: center); path.addLine(to: point) }
                        .stroke(c.borderStrong, lineWidth: 1.3)
                }

                if isWide {
                    ForEach(placedSecondDegree(firstRing: ring1, secondRing: ring2), id: \.id) { placed in
                        if let parentAngle = placedFirstDegree(ring: ring1).first(where: { $0.node.id == parentId(for: placed.node) })?.angle {
                            let from = CGPoint(x: center.x + cos(parentAngle) * ring1, y: center.y + sin(parentAngle) * ring1)
                            let to = CGPoint(x: center.x + cos(placed.angle) * placed.radius, y: center.y + sin(placed.angle) * placed.radius)
                            Path { path in path.move(to: from); path.addLine(to: to) }
                                .stroke(c.borderDefault, lineWidth: 1)
                        }
                    }
                    ForEach(placedSecondDegree(firstRing: ring1, secondRing: ring2), id: \.id) { placed in
                        nodeDot(placed.node, radius: 4)
                            .position(x: center.x + cos(placed.angle) * placed.radius, y: center.y + sin(placed.angle) * placed.radius)
                    }
                }

                ForEach(placedFirstDegree(ring: ring1), id: \.id) { placed in
                    nodeButton(placed.node)
                        .position(x: center.x + cos(placed.angle) * placed.radius, y: center.y + sin(placed.angle) * placed.radius)
                }

                Circle().fill(c.bgSurfaceRaised).stroke(c.accentDefault, lineWidth: 1.6).frame(width: 30, height: 30).position(center)
                Circle().fill(c.accentDefault).frame(width: 12, height: 12).position(center)
            }
        }
    }

    private func nodeButton(_ node: CachedNode) -> some View {
        Button {
            onOpenNode(node)
        } label: {
            VStack(spacing: 4) {
                nodeDot(node, radius: 7)
                Text(titleFor(node)).font(.system(size: 12)).foregroundStyle(c.textPrimary).lineLimit(1)
            }
            .padding(6)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func nodeDot(_ node: CachedNode, radius: CGFloat) -> some View {
        Circle().fill(dotColor(for: node.nodeType)).frame(width: radius * 2, height: radius * 2)
    }

    private func dotColor(for nodeType: String) -> Color {
        switch nodeType {
        case "project": return c.warning
        case "area": return c.success
        case "resource": return c.accentDefault
        default: return c.borderStrong
        }
    }

    private var legend: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            legendRow("Projects", c.warning)
            legendRow("Areas", c.success)
            legendRow("Resources", c.accentDefault)
            legendRow("Other", c.borderStrong)
        }
        .padding(Space.md)
        .background(c.bgSurfaceRaised.opacity(0.85))
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .padding(Space.lg)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
    }

    private func legendRow(_ label: String, _ color: Color) -> some View {
        HStack(spacing: Space.xs) {
            Circle().fill(color).frame(width: 7, height: 7)
            Text(label).font(.system(size: 12)).foregroundStyle(c.textSecondary)
        }
    }

    private var bottomBar: some View {
        HStack(spacing: Space.md) {
            Text("APERTURE").font(.system(size: 12)).foregroundStyle(c.textSecondary)
            Text("Focused").font(.system(size: 12)).foregroundStyle(c.textDisabled)
            Slider(value: $aperture, in: 0...1).frame(width: 220)
            Text("Wide").font(.system(size: 12)).foregroundStyle(c.textDisabled)
        }
        .padding(.horizontal, Space.lg)
        .frame(height: 56)
        .background(c.bgSurface)
        .overlay(alignment: .top) { Rectangle().fill(c.borderDefault).frame(height: 1) }
    }

    // MARK: - Layout

    private func placedFirstDegree(ring: CGFloat) -> [Placed] {
        guard !firstDegree.isEmpty else { return [] }
        return firstDegree.enumerated().map { index, node in
            let angle = (Double(index) / Double(firstDegree.count)) * 2 * .pi - .pi / 2
            return Placed(node: node, angle: angle, radius: ring)
        }
    }

    private func placedSecondDegree(firstRing: CGFloat, secondRing: CGFloat) -> [Placed] {
        var placed: [Placed] = []
        let firstAngles = Dictionary(uniqueKeysWithValues: placedFirstDegree(ring: firstRing).map { ($0.node.id, $0.angle) })
        for (parentId, children) in secondDegree {
            guard let baseAngle = firstAngles[parentId], !children.isEmpty else { continue }
            let spread = 0.5 // radians, total arc the children fan across
            for (index, child) in children.enumerated() {
                let offset = children.count == 1 ? 0 : spread * (Double(index) / Double(children.count - 1) - 0.5)
                placed.append(Placed(node: child, angle: baseAngle + offset, radius: secondRing))
            }
        }
        return placed
    }

    private func parentId(for secondDegreeNode: CachedNode) -> String? {
        secondDegree.first(where: { _, children in children.contains(where: { $0.id == secondDegreeNode.id }) })?.key
    }

    private func titleFor(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    // MARK: - Data

    private func load() {
        let direct = (try? engine.connections(nodeId: centerNode.id))?.map(\.node) ?? []
        firstDegree = direct

        var seen = Set(direct.map(\.id))
        seen.insert(centerNode.id)
        var second: [String: [CachedNode]] = [:]
        for node in direct {
            let hop2 = (try? engine.connections(nodeId: node.id))?.map(\.node) ?? []
            let fresh = hop2.filter { !seen.contains($0.id) }
            if !fresh.isEmpty {
                second[node.id] = fresh
                fresh.forEach { seen.insert($0.id) }
            }
        }
        secondDegree = second
    }
}
