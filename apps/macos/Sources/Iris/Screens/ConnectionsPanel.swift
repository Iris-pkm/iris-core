import SwiftUI
import IrisCore

/// The Node Editor's right panel "Connections" tab (`design/navigation.md`
/// §1, embedded in `RightRail`) — real backlinks/relations, not the
/// mockup's static example data. Backed by `connections.rs` (new this
/// pass — the generic bidirectional relation lookup, not just the
/// `blocks`/`depends-on` inversion `dependencies.rs` already had).
///
/// **Scope, honestly flagged:** the mockup's radial mini-graph is decorative
/// (a fixed ring/line illustration, not laid out from real connection
/// positions) — rendering an actual force/radial layout from arbitrary
/// connection counts is real graph-visualization work (closer to Phase 6's
/// full graph view than this panel's job). Renders a simplified static
/// version scaled to the real connection count instead of faking a precise
/// layout, and the relation list below it is fully real.
///
/// Content only — `RightRail` owns the shared width/background/tab row/
/// outer padding for both this and `DevModeView`.
struct ConnectionsPanel: View {
    let engine: FfiEngine
    let relPath: String

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var connections: [Connection] = []

    var body: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            if connections.isEmpty {
                Text("No connections yet.")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textDisabled)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, Space.xxl)
            } else {
                radialGlyph
            }

            VStack(alignment: .leading, spacing: Space.sm) {
                ForEach(Array(connections.enumerated()), id: \.offset) { _, conn in
                    HStack(spacing: Space.sm) {
                        Circle().fill(dotColor(for: conn)).frame(width: 7, height: 7)
                        Text(titleFor(conn.node))
                            .font(Typography.bodySmall())
                            .foregroundStyle(c.textSecondary)
                            .lineLimit(1)
                        Spacer()
                        Text(conn.label)
                            .font(Typography.caption())
                            .foregroundStyle(c.textDisabled)
                    }
                }
            }

            Spacer()
        }
        .task(id: relPath) { load() }
    }

    /// A simplified radial illustration — real connection count, decorative
    /// positions (see the file-level doc comment on why a precise layout
    /// isn't built here).
    private var radialGlyph: some View {
        ZStack {
            Circle().stroke(c.borderDefault, lineWidth: 1).frame(width: 190, height: 190)
            Circle().stroke(c.borderStrong, lineWidth: 1).frame(width: 120, height: 120)
            ForEach(Array(connections.prefix(6).enumerated()), id: \.offset) { index, conn in
                let angle = Angle.degrees(Double(index) / Double(max(connections.count, 1)) * 360)
                let radius: CGFloat = 95
                Circle()
                    .fill(dotColor(for: conn))
                    .frame(width: 9, height: 9)
                    .offset(x: cos(angle.radians) * radius, y: sin(angle.radians) * radius)
            }
            Circle().fill(c.accentDefault).frame(width: 14, height: 14)
        }
        .frame(height: 190)
        .padding(.vertical, Space.md)
    }

    /// No dot-color rule is specified in `design/tokens.md`/`components.md`
    /// for this panel — flagged judgment call: colors by the *connected*
    /// node's own completion state (an incomplete task reads as needing
    /// attention/warning; anything without an open status — a reference,
    /// a completed task — reads as settled/success), rather than a static
    /// per-relation-type color, since that's the signal actually available
    /// on `CachedNode` and matches the mockup's example data (its "child of"
    /// items happen to be open tasks, its "refs" item a static resource).
    private func dotColor(for conn: Connection) -> Color {
        if let status = conn.node.status, status != "done" {
            return c.warning
        }
        return c.success
    }

    private func titleFor(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func load() {
        guard let parsed = try? engine.readNode(relPath: relPath) else {
            connections = []
            return
        }
        connections = (try? engine.connections(nodeId: parsed.node.id)) ?? []
    }
}
