import SwiftUI
import IrisCore

/// Holds the currently-open vault's engine so a secondary node window
/// (`ARCHITECTURE.md` §9 Multi-window) can reach the same in-process
/// `Engine`/cache the main window uses — no second vault, no sync protocol,
/// just another view onto the one open graph (ADR-014's single-active-vault
/// model still holds; this is windowing, not multi-vault). `OnboardingView`
/// sets `engine` the moment a vault becomes ready via any of its four paths
/// (create/open/import/restore); it's `nil` until then, which the node
/// window handles as "no vault open yet" rather than assuming non-nil.
final class VaultSession: ObservableObject {
    @Published var engine: FfiEngine?
}

@main
struct IrisApp: App {
    @AppStorage("iris.appearance") private var appearance = "System"
    @StateObject private var session = VaultSession()

    var body: some Scene {
        WindowGroup {
            OnboardingView()
                .environmentObject(session)
                .preferredColorScheme(colorScheme)
        }
        .windowResizability(.contentSize)

        // A node pulled into its own window via "Open in New Window"
        // (`NodeEditorView`'s toolbar) — the window's own identity is the
        // node's vault-relative path.
        WindowGroup("Note", for: String.self) { $relPath in
            NodeWindowView(session: session, relPath: relPath)
                .preferredColorScheme(colorScheme)
        }

        Settings {
            SettingsView()
                .preferredColorScheme(colorScheme)
        }
    }

    private var colorScheme: ColorScheme? {
        switch appearance {
        case "Light": return .light
        case "Dark": return .dark
        default: return nil
        }
    }
}
