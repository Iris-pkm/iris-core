import SwiftUI
import IrisCore

/// The content of a secondary "Open in New Window" node window
/// (`ARCHITECTURE.md` §9 Multi-window, `IrisApp`'s `"Note"` `WindowGroup`).
/// Same shell as `AppShell`'s own non-project node route (`NodeEditorView` +
/// `RightRail`) — a node doesn't get a different layout just for being in
/// its own window.
///
/// `session.engine` can be `nil` here in one real case: macOS restoring a
/// previously-open node window on relaunch, before `OnboardingView` has
/// reopened a vault. Shown as an explicit "vault not open" state rather than
/// crashing on a force-unwrap or silently showing nothing.
struct NodeWindowView: View {
    @ObservedObject var session: VaultSession
    let relPath: String?

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        Group {
            if let engine = session.engine, let relPath {
                HStack(spacing: 0) {
                    NodeEditorView(engine: engine, relPath: relPath)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    Divider().background(c.borderDefault)
                    RightRail(engine: engine, relPath: relPath)
                }
            } else {
                VStack(spacing: Space.sm) {
                    Text("No vault open").font(Typography.h1()).foregroundStyle(c.textPrimary)
                    Text("Open Iris's main window and open a vault first.")
                        .font(Typography.bodySans())
                        .foregroundStyle(c.textSecondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(c.bgCanvas)
        .frame(minWidth: 480, idealWidth: 720, minHeight: 400, idealHeight: 600)
    }
}
