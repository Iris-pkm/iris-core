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

    /// "This note was just saved" — published so any other open editor on the
    /// same note (another window) can refresh (ADR-038 follow-up). In-process
    /// only: edits made by the CLI or another app are not seen.
    struct Change: Equatable {
        let path: String
        /// The editor that saved, so it can ignore its own echo.
        let origin: UUID
        let seq: Int
    }
    @Published private(set) var change: Change?
    private var seq = 0

    func noteChanged(path: String, from origin: UUID) {
        seq += 1
        change = Change(path: path, origin: origin, seq: seq)
    }
}

@main
struct IrisApp: App {
    @AppStorage("iris.appearance") private var appearance = "System"
    @StateObject private var session = VaultSession()

    var body: some Scene {
        WindowGroup {
            OnboardingView()
                .environmentObject(session)
                .onAppear { QuickCapturePanel.shared.install(session: session) }
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
