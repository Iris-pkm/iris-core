import AppKit
import Carbon.HIToolbox
import SwiftUI
import IrisCore

/// Global Quick Capture (⌘⇧C): a borderless floating panel that works even
/// when Iris isn't the frontmost app (`screen-flow.md` kind C). The hotkey
/// uses Carbon `RegisterEventHotKey` — it needs no Accessibility permission,
/// unlike an `NSEvent` global monitor.
final class QuickCapturePanel {
    static let shared = QuickCapturePanel()

    private var panel: KeyablePanel?
    private var recents: [CaptureItem] = []
    private var hotKeyRef: EventHotKeyRef?
    private weak var session: VaultSession?

    /// Registers ⌘⇧C. Idempotent.
    func install(session: VaultSession) {
        self.session = session
        guard hotKeyRef == nil else { return }
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(GetApplicationEventTarget(), { _, _, _ in
            DispatchQueue.main.async { QuickCapturePanel.shared.toggle() }
            return noErr
        }, 1, &spec, nil, nil)
        RegisterEventHotKey(UInt32(kVK_ANSI_C), UInt32(cmdKey | shiftKey),
                            EventHotKeyID(signature: OSType(0x49524953), id: 1), // 'IRIS'
                            GetApplicationEventTarget(), 0, &hotKeyRef)
    }

    func toggle() {
        if panel?.isVisible == true { hide() } else { show() }
    }

    func hide() { panel?.orderOut(nil) }

    private func show() {
        // No vault open yet (onboarding still showing): nothing to capture into.
        guard let engine = session?.engine else { NSSound.beep(); return }
        let panel = self.panel ?? KeyablePanel()
        self.panel = panel

        let appearance = UserDefaults.standard.string(forKey: "iris.appearance") ?? "System"
        let scheme: ColorScheme? = appearance == "Light" ? .light : appearance == "Dark" ? .dark : nil
        // Rebuilt on every show so the field starts empty and focused.
        let root = QuickCaptureView(
            engine: engine, recentCaptures: recents,
            dismiss: { [weak self] in self?.hide() },
            saved: { [weak self] item in
                self?.recents.insert(item, at: 0)
                self?.hide()
            },
            floating: true
        ).preferredColorScheme(scheme)
        panel.contentView = NSHostingView(rootView: root)

        let size = NSSize(width: 600, height: 400)
        let screen = NSScreen.screens.first { $0.frame.contains(NSEvent.mouseLocation) } ?? NSScreen.main
        if let f = screen?.visibleFrame {
            panel.setFrame(NSRect(x: f.midX - size.width / 2, y: f.maxY - f.height * 0.2 - size.height,
                                  width: size.width, height: size.height), display: true)
        }
        panel.makeKeyAndOrderFront(nil)
    }
}

/// Borderless panels refuse key status by default, which would leave the
/// text field unable to type. Non-activating so it doesn't pull Iris's main
/// window forward over whatever the user was doing.
private final class KeyablePanel: NSPanel {
    init() {
        super.init(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel],
                   backing: .buffered, defer: true)
        isFloatingPanel = true
        level = .floating
        isOpaque = false
        backgroundColor = .clear
        hasShadow = false // the card draws its own
        hidesOnDeactivate = false
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
    }
    override var canBecomeKey: Bool { true }

    /// Clicking away dismisses, like any transient palette.
    override func resignKey() {
        super.resignKey()
        orderOut(nil)
    }
}
