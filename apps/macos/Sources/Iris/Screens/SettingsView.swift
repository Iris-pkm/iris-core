import SwiftUI

/// Native macOS Preferences content. These are app-local preferences; vault
/// sync and AI configuration stay visibly unavailable until their phases have
/// real backend settings to persist.
struct SettingsView: View {
    private enum Section: String, CaseIterable, Identifiable {
        case general = "General", appearance = "Appearance", ai = "AI", vault = "Vault & Sync", shortcuts = "Shortcuts"
        var id: String { rawValue }
        var icon: String {
            switch self {
            case .general: return "gearshape"
            case .appearance: return "circle.lefthalf.filled"
            case .ai: return "sparkles"
            case .vault: return "archivebox"
            case .shortcuts: return "keyboard"
            }
        }
    }

    @State private var selected: Section = .general
    @AppStorage("iris.launchView") private var launchView = "Today"
    @AppStorage("iris.appearance") private var appearance = "System"

    var body: some View {
        HStack(spacing: 0) {
            List(Section.allCases, selection: $selected) { section in
                Label(section.rawValue, systemImage: section.icon).tag(section)
            }
            .listStyle(.sidebar)
            .frame(width: 180)

            VStack(alignment: .leading, spacing: Space.xl) {
                Text(selected.rawValue).font(Typography.h1())
                content
                Spacer()
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xxl, trailing: Space.xxxl))
            .frame(minWidth: 520, minHeight: 360, alignment: .topLeading)
        }
    }

    @ViewBuilder private var content: some View {
        switch selected {
        case .general:
            preferenceRow("Launch view", detail: "What opens when Iris starts") {
                Picker("Launch view", selection: $launchView) {
                    Text("Today").tag("Today")
                    Text("Inbox").tag("Inbox")
                }.labelsHidden().frame(width: 130)
            }
            preferenceRow("Quick capture", detail: "Focused-app shortcut") {
                Text("⌘⇧C").font(Typography.mono(13)).padding(.horizontal, Space.sm).padding(.vertical, Space.xs).background(Color.secondary.opacity(0.12)).clipShape(RoundedRectangle(cornerRadius: Radius.sm))
            }
        case .appearance:
            preferenceRow("Appearance", detail: "Choose how Iris follows macOS appearance") {
                Picker("Appearance", selection: $appearance) {
                    Text("System").tag("System")
                    Text("Light").tag("Light")
                    Text("Dark").tag("Dark")
                }.labelsHidden().pickerStyle(.segmented).frame(width: 230)
            }
        case .ai:
            unavailable("AI configuration arrives with Phase 4's optional distillation provider. Iris does not store provider credentials yet.")
        case .vault:
            unavailable("Vault sync configuration arrives with Phase 5. Your vault remains local and git-backed today.")
        case .shortcuts:
            shortcut("Search vault", "⌘K")
            shortcut("Quick capture", "⌘⇧C")
            shortcut("Open graph", "⌘G", note: "Graph is a later Phase 6 surface.")
        }
    }

    private func preferenceRow<Control: View>(_ title: String, detail: String, @ViewBuilder control: () -> Control) -> some View {
        HStack(alignment: .center, spacing: Space.lg) {
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title).font(Typography.sans(14, weight: .medium))
                Text(detail).font(Typography.bodySmall()).foregroundStyle(.secondary)
            }
            Spacer()
            control()
        }
        .padding(.vertical, Space.md)
        .overlay(alignment: .bottom) { Divider() }
    }

    private func unavailable(_ copy: String) -> some View {
        Text(copy).font(Typography.bodySans()).foregroundStyle(.secondary).frame(maxWidth: 420, alignment: .leading)
    }

    private func shortcut(_ title: String, _ keys: String, note: String? = nil) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title).font(Typography.bodySans())
                if let note { Text(note).font(Typography.bodySmall()).foregroundStyle(.secondary) }
            }
            Spacer()
            Text(keys).font(Typography.mono(13)).foregroundStyle(.secondary)
        }
        .padding(.vertical, Space.sm)
    }
}
