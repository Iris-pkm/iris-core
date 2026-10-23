import AppKit
import SwiftUI
import IrisCore
import IrisVaultWatch

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
    @Published private(set) var watchError: String?
    static let externalOrigin = UUID()
    private var vaultURL: URL?
    private var watcher: VaultWatcher?
    private var watcherID = UUID()
    private var quitObserver: NSObjectProtocol?

    /// One changed canonical path, or `*` for a large external burst. Editors,
    /// lists and computed displays consume the same signal regardless of writer.
    struct Change: Equatable {
        let path: String
        /// The editor that saved, so it can ignore its own echo.
        let origin: UUID
        let seq: Int
    }
    @Published private(set) var change: Change?
    private var seq = 0

    init() {
        quitObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification, object: nil, queue: .main
        ) { [weak self] _ in self?.closeVault() }
    }

    func openVault(engine: FfiEngine, path: String) {
        let url = URL(fileURLWithPath: path).standardizedFileURL
        if vaultURL == url, self.engine === engine { return }
        watcher?.stop()
        watcher = nil
        vaultURL = url
        self.engine = engine
        watchError = nil
        let id = UUID()
        watcherID = id
        watcher = VaultWatcher(root: url, onChanges: { [weak self] paths in
            DispatchQueue.main.async {
                guard let self, self.watcherID == id else { return }
                self.watchError = nil
                for path in paths { self.noteChanged(path: path, from: Self.externalOrigin) }
            }
        }, onError: { [weak self] message in
            DispatchQueue.main.async {
                guard let self, self.watcherID == id else { return }
                self.watchError = message
            }
        })
        if watcher == nil { watchError = "Could not watch this vault for external changes." }
    }

    func closeVault() {
        watcherID = UUID()
        watcher?.stop()
        watcher = nil
        vaultURL = nil
        engine = nil
        watchError = nil
    }

    func fingerprint(for path: String) -> FileFingerprint? {
        guard let vaultURL, path != VaultWatchRules.allPaths else { return nil }
        return FileFingerprint.read(vaultURL.appendingPathComponent(path))
    }

    func beginOwnWrite(path: String) { watcher?.beginOwnWrite(path: path) }
    func endOwnWrite(path: String) { watcher?.endOwnWrite(path: path) }

    func noteChanged(path: String, from origin: UUID) {
        // FSEvents ignores writes from this process. Node Editor writes also
        // register a fingerprint as a fallback against delayed own events.
        if origin != Self.externalOrigin {
            if let fingerprint = fingerprint(for: path) {
                watcher?.recordOwnSave(path: path, fingerprint: fingerprint)
            } else {
                watcher?.endOwnWrite(path: path)
            }
        }
        seq += 1
        change = Change(path: path, origin: origin, seq: seq)
    }

    deinit {
        watcher?.stop()
        if let quitObserver { NotificationCenter.default.removeObserver(quitObserver) }
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
