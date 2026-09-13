import SwiftUI
import IrisCore

/// The Node Editor's right panel shell (`design/canvas/Main.dc.html` /
/// `DevMode.dc.html`) — owns the shared width/background/tab row/padding
/// for both tabs; `ConnectionsPanel` and `DevModeView` are content-only.
struct RightRail: View {
    let engine: FfiEngine
    let relPath: String

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private enum Tab { case connections, dev }
    @State private var tab: Tab = .connections

    var body: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            tabRow

            switch tab {
            case .connections:
                ConnectionsPanel(engine: engine, relPath: relPath)
            case .dev:
                DevModeView(engine: engine, relPath: relPath)
            }
        }
        .padding(Space.lg)
        .frame(width: 300)
        .frame(maxHeight: .infinity)
        .background(c.bgSurface)
    }

    private var tabRow: some View {
        HStack(spacing: Space.xs) {
            tabButton("Connections", tab: .connections)
            tabButton("Dev Mode", tab: .dev)
        }
        .padding(.bottom, Space.xs)
        .overlay(alignment: .bottom) {
            Rectangle().fill(c.borderDefault).frame(height: 1)
        }
    }

    private func tabButton(_ title: String, tab target: Tab) -> some View {
        let isActive = tab == target
        return Button {
            tab = target
        } label: {
            Text(title)
                .font(Typography.caption())
                .foregroundStyle(isActive ? c.accentDefault : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(isActive ? c.bgSelected : Color.clear)
                .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }
}
