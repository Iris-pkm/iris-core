import SwiftUI
import AppKit
import IrisCore

/// `design/canvas/Plugins.dc.html` — the last screen in the whole
/// inventory, pulled forward from Phase 7 alongside Graph/Timeline, with a
/// real WASM runtime behind it this time, not just UI wired to an existing
/// backend: `iris-core/src/plugins.rs` (ADR-034) is a from-scratch
/// `wasmtime` sandbox, permission-checked host API, and YAML manifest
/// format — there was nothing to reuse, the permission model was
/// explicitly listed as "genuinely open" before this pass.
///
/// **Installed tab is fully real:** every card comes from `listPlugins()`;
/// enable/disable is a real `Toggle` wired to `setPluginEnabled`; "Run"
/// actually executes the plugin's sandboxed `plugin_run` via `runPlugin`
/// and shows what it logged — real proof of execution, not a static card.
/// "Install…" opens a real `NSOpenPanel` for a bundle folder (manifest.yaml
/// + plugin.wasm), same pattern `OnboardingView.pickFolder` already uses.
///
/// **Browse tab stays illustrative, deliberately** — matching the mockup's
/// own banner almost verbatim: community plugin distribution is Phase 8 in
/// `ROADMAP.md`, a different phase entirely from the runtime this pass
/// built, not a corner cut here. No working Install button on these cards.
struct PluginsView: View {
    let engine: FfiEngine

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private enum Tab { case installed, browse }
    @State private var tab: Tab = .installed
    @State private var plugins: [FfiPluginInfo] = []
    @State private var runOutput: [String: [String]] = [:]
    @State private var errorMessage: String?

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: Space.xl) {
                header
                tabRow

                switch tab {
                case .installed: installedList
                case .browse: browseGrid
                }

                Spacer()
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

            Divider().background(c.borderDefault)
            rail
        }
        .background(c.bgCanvas)
        .task { load() }
        .alert("Plugin error", isPresented: .constant(errorMessage != nil), actions: {
            Button("OK") { errorMessage = nil }
        }, message: {
            Text(errorMessage ?? "")
        })
    }

    private var header: some View {
        HStack(alignment: .lastTextBaseline) {
            Text("Plugins").font(Typography.h1()).foregroundStyle(c.textPrimary)
            Spacer()
            Button("Install…") { installFromFolder() }
                .buttonStyle(.plain)
                .font(Typography.bodySmall())
                .foregroundStyle(.white)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(c.accentDefault)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
    }

    private var tabRow: some View {
        HStack(spacing: 2) {
            segment("Installed", isActive: tab == .installed) { tab = .installed }
            segment("Browse", isActive: tab == .browse) { tab = .browse }
        }
        .padding(2)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func segment(_ title: String, isActive: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(Typography.bodySmall())
                .foregroundStyle(isActive ? c.textPrimary : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(isActive ? c.bgSurfaceRaised : Color.clear)
                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    // MARK: - Installed

    private var installedList: some View {
        Group {
            if plugins.isEmpty {
                VStack(spacing: Space.xs) {
                    Text("No plugins installed").font(Typography.serif(17, weight: .semibold)).foregroundStyle(c.textPrimary)
                    Text("Install the bundled hello-world example (iris-core/examples/plugins/hello-world) to see the sandbox actually run.")
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.textSecondary)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: 360)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, Space.xxxl)
            } else {
                VStack(spacing: Space.md) {
                    ForEach(plugins, id: \.id) { plugin in
                        pluginCard(plugin)
                    }
                }
            }
        }
    }

    private func pluginCard(_ plugin: FfiPluginInfo) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            HStack(spacing: Space.md) {
                Circle().fill(c.accentTint)
                    .frame(width: 32, height: 32)
                    .overlay(Text(String(plugin.name.prefix(1))).font(Typography.caption()).foregroundStyle(c.accentDefault))

                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: Space.sm) {
                        Text(plugin.name).font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
                        Text(plugin.version).font(Typography.mono(11)).foregroundStyle(c.textSecondary)
                    }
                    Text(plugin.description).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                }

                Spacer()

                Toggle("", isOn: Binding(
                    get: { plugin.enabled },
                    set: { setEnabled(plugin, $0) }
                ))
                .labelsHidden()
                .toggleStyle(.switch)
            }

            HStack(spacing: Space.xs) {
                Image(systemName: "lock").font(.system(size: 10)).foregroundStyle(c.textSecondary)
                Text(permissionSummary(plugin)).font(Typography.caption()).foregroundStyle(c.textSecondary)
            }

            if let output = runOutput[plugin.id] {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(output.enumerated()), id: \.offset) { _, line in
                        Text(line).font(Typography.mono(11)).foregroundStyle(c.textPrimary)
                    }
                }
                .padding(Space.sm)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(c.bgHover)
                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
            }

            Button("Run") { run(plugin) }
                .buttonStyle(.plain)
                .font(Typography.bodySmall())
                .foregroundStyle(plugin.enabled ? c.accentDefault : c.textDisabled)
                .padding(.vertical, Space.xxs)
                .contentShape(Rectangle())
                .disabled(!plugin.enabled)
        }
        .padding(Space.md)
        .background(c.bgSurface)
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
    }

    private func permissionSummary(_ plugin: FfiPluginInfo) -> String {
        var parts: [String] = []
        if !plugin.nodeTypePermissions.isEmpty {
            parts.append("Node types: \(plugin.nodeTypePermissions.joined(separator: ", "))")
        }
        if !plugin.networkHostPermissions.isEmpty {
            // Declared only — v1's host API has no function to actually make
            // a network call, so this is shown as intent, not a live grant.
            parts.append("Network (declared, not yet usable): \(plugin.networkHostPermissions.joined(separator: ", "))")
        }
        return parts.isEmpty ? "No permissions declared" : parts.joined(separator: " · ")
    }

    // MARK: - Browse (illustrative only — Phase 8, not this pass)

    private struct MarketEntry: Identifiable {
        let id = UUID()
        let name: String
        let author: String
        let description: String
    }

    private let marketEntries = [
        MarketEntry(name: "Broker Sync", author: "community", description: "Illustrative only — pulls trade fills from a brokerage API into trading-journal-entry nodes."),
        MarketEntry(name: "Habit Tracker", author: "community", description: "Illustrative only — a custom node type for daily habit streaks."),
    ]

    private var browseGrid: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            Text("Community plugin distribution isn't built yet — this view is illustrative of the eventual shape, not a working marketplace.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
                .padding(Space.md)
                .background(c.accentTint)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))

            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: Space.md) {
                ForEach(marketEntries) { entry in
                    VStack(alignment: .leading, spacing: Space.xs) {
                        Text(entry.name).font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
                        Text("by \(entry.author)").font(Typography.caption()).foregroundStyle(c.textSecondary)
                        Text(entry.description).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
                        Text("Install").font(Typography.bodySmall()).foregroundStyle(c.textDisabled)
                    }
                    .padding(Space.md)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(c.bgSurface)
                    .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
                    .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
                }
            }
        }
    }

    // MARK: - Right rail

    private var rail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("SANDBOXING").font(Typography.caption()).foregroundStyle(c.textSecondary)
                Text("Plugins run in a WASM sandbox against a permissioned API — a plugin declares what it needs (node types, network hosts) up front, and can't reach past that.")
                    .font(Typography.bodySmall())
                    .foregroundStyle(c.textPrimary)
            }
            Rectangle().fill(c.borderDefault).frame(height: 1)
            VStack(alignment: .leading, spacing: Space.sm) {
                Text("STATUS").font(Typography.caption()).foregroundStyle(c.textSecondary)
                statRow("Enabled", plugins.filter { $0.enabled }.count)
                statRow("Disabled", plugins.filter { !$0.enabled }.count)
            }
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 260)
        .frame(maxHeight: .infinity)
        .background(c.bgSurface)
    }

    private func statRow(_ label: String, _ value: Int) -> some View {
        HStack {
            Text(label).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
            Spacer()
            Text("\(value)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }

    // MARK: - Actions

    private func installFromFolder() {
        let panel = NSOpenPanel()
        panel.title = "Choose a plugin bundle folder (manifest.yaml + plugin.wasm)"
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let path = panel.url?.path else { return }
        do {
            _ = try engine.installPlugin(bundleDir: path)
            load()
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func setEnabled(_ plugin: FfiPluginInfo, _ enabled: Bool) {
        do {
            try engine.setPluginEnabled(id: plugin.id, enabled: enabled)
            load()
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func run(_ plugin: FfiPluginInfo) {
        do {
            let logs = try engine.runPlugin(id: plugin.id)
            runOutput[plugin.id] = logs.isEmpty ? ["(no output)"] : logs
            load()
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func load() {
        plugins = (try? engine.listPlugins()) ?? []
    }
}
