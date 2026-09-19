import SwiftUI
import AppKit
import IrisCore

/// Which onboarding path is selected — mirrors `Onboarding.dc.html`'s four
/// cards. `navigation.md`'s Onboarding row: "no vault open yet... four
/// paths: create / open / import / restore."
private enum OnboardingPath: String, CaseIterable, Identifiable {
    case create, open, `import`, restore
    var id: String { rawValue }

    var title: String {
        switch self {
        case .create: return "Create new vault"
        case .open: return "Open existing vault"
        case .import: return "Import"
        case .restore: return "Restore from backup"
        }
    }
    var description: String {
        switch self {
        case .create: return "Start empty, with starter templates and a small example graph."
        case .open: return "Point Iris at a folder of markdown notes you already have."
        case .import: return "Bring your vault in from Obsidian, Notion, Evernote, and more."
        case .restore: return "Clone from a git remote — rebuilds the cache and checks integrity."
        }
    }
    var symbol: String {
        switch self {
        case .create: return "plus.rectangle"
        case .open: return "folder"
        case .import: return "square.and.arrow.down"
        case .restore: return "arrow.counterclockwise"
        }
    }
}

/// Import sources ADR-025 actually implements today (`import.rs`'s own
/// module doc: "First importers: plain-Markdown-folder and Obsidian").
/// The other four from `Onboarding.dc.html`'s source chips are real,
/// named product scope — just not built yet — so they're shown disabled
/// with their true state, not hidden and not faked as available.
private enum ImportSource: String, CaseIterable, Identifiable {
    case obsidian = "Obsidian vault"
    case markdown = "Plain Markdown folder"
    case notion = "Notion export"
    case evernote = "Evernote (.enex)"
    case roam = "Roam JSON"
    case taskCSV = "Task CSV"

    var id: String { rawValue }
    var isImplemented: Bool { self == .obsidian || self == .markdown }
}

struct OnboardingView: View {
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var selected: OnboardingPath?
    @State private var errorMessage: String?
    @State private var readyEngine: FfiEngine?
    @State private var restoreURL: String = ""
    @AppStorage("iris.launchView") private var launchView = "Today"

    var body: some View {
        VStack(spacing: 0) {
            if let engine = readyEngine {
                AppShell(engine: engine, initialLens: initialLens)
            } else {
                content
            }
        }
        .background(c.bgCanvas)
        .frame(minWidth: 900, idealWidth: 1100, minHeight: 640, idealHeight: 760)
        .alert("Couldn't complete that", isPresented: .constant(errorMessage != nil), actions: {
            Button("OK") { errorMessage = nil }
        }, message: {
            Text(errorMessage ?? "")
        })
    }

    private var initialLens: TaskLens? {
        switch launchView {
        case "Inbox": return .inbox
        default: return .today
        }
    }

    private var content: some View {
        ScrollView {
            VStack(spacing: Space.xxl) {
                Spacer(minLength: Space.xxxl)

                logo
                VStack(spacing: Space.sm) {
                    Text("Welcome to Iris")
                        .font(Typography.h1())
                        .foregroundStyle(c.textPrimary)
                    Text("A local-first second brain, built for one. No account, no sign-in — everything lives in a folder you choose.")
                        .font(Typography.bodySans())
                        .foregroundStyle(c.textSecondary)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: 460)
                }

                cardRow

                if selected == .import {
                    importPanel
                }
                if selected == .restore {
                    restorePanel
                }

                recentVaultsSection

                Spacer(minLength: Space.xxl)
            }
            .padding(.horizontal, Space.xxl)
            .frame(maxWidth: 1080)
        }
    }

    private var logo: some View {
        // Onboarding.dc.html's concentric-circles mark — the accent color
        // is Iris's one deliberate splash of color on this screen.
        Circle()
            .strokeBorder(c.accentDefault, lineWidth: 1.2)
            .frame(width: 44, height: 44)
            .overlay(
                Circle().strokeBorder(c.accentHover, lineWidth: 1.2).frame(width: 24, height: 24)
            )
            .overlay(Circle().fill(c.accentDefault).frame(width: 8, height: 8))
    }

    private var cardRow: some View {
        HStack(spacing: Space.md) {
            ForEach(OnboardingPath.allCases) { path in
                OnboardingCard(path: path, isSelected: selected == path, colors: c) {
                    switch path {
                    case .create:
                        // No inline panel for this card — the mockup's
                        // "Create" click goes straight to a folder picker,
                        // unlike Import/Restore which reveal more choices
                        // first. Toggling `selected` alone (the previous
                        // behavior) never called anything — a real bug,
                        // not a `selected == .create` panel that was
                        // merely unbuilt.
                        handleCreate()
                    case .open:
                        handleOpen()
                    case .import, .restore:
                        selected = (selected == path) ? nil : path
                    }
                }
            }
        }
    }

    private var importPanel: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            Text("IMPORT FROM")
                .font(Typography.caption())
                .foregroundStyle(c.textSecondary)

            FlowLayout(spacing: Space.sm) {
                ForEach(ImportSource.allCases) { source in
                    SourceChip(source: source, colors: c) {
                        guard source.isImplemented else { return }
                        pickDestinationThenImport(obsidian: source == .obsidian)
                    }
                }
            }

            Text("Links, attachments, tags, timestamps, and folder hierarchy are preserved wherever the source format allows it.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
        }
        .padding(Space.lg)
        .background(c.bgSurface)
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
    }

    private var restorePanel: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            Text("GIT REMOTE URL")
                .font(Typography.caption())
                .foregroundStyle(c.textSecondary)
            TextField("git@example.com:you/vault.git", text: $restoreURL)
                .textFieldStyle(.plain)
                .font(Typography.bodySans())
                .padding(.horizontal, Space.md)
                .frame(height: 36)
                .background(c.bgSurface)
                .foregroundStyle(c.textPrimary)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
                .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))

            Button("Choose destination & restore") { pickDestinationThenRestore() }
                .buttonStyle(PrimaryButtonStyle(colors: c))
                .disabled(restoreURL.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(Space.lg)
        .background(c.bgSurface)
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
    }

    private var recentVaultsSection: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Divider().background(c.borderDefault)
            Text("RECENT VAULTS")
                .font(Typography.caption())
                .foregroundStyle(c.textSecondary)
                .padding(.top, Space.lg)
            Text("No recent vaults yet — vaults you open or create will appear here.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textDisabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Actions

    private func pickFolder(title: String, createDirectories: Bool) -> String? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = createDirectories
        panel.allowsMultipleSelection = false
        return panel.runModal() == .OK ? panel.url?.path : nil
    }

    private func handleCreate() {
        guard let path = pickFolder(title: "Choose a location for the new vault", createDirectories: true) else { return }
        do {
            readyEngine = try FfiEngine.`init`(path: path)
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func handleOpen() {
        guard let path = pickFolder(title: "Choose an existing vault", createDirectories: false) else { return }
        do {
            readyEngine = try FfiEngine.open(path: path)
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func pickDestinationThenImport(obsidian: Bool) {
        guard let dest = pickFolder(title: "Choose a location for the new vault", createDirectories: true) else { return }
        guard let source = pickFolder(title: obsidian ? "Choose the Obsidian vault folder" : "Choose the Markdown folder to import", createDirectories: false) else { return }
        do {
            let engine = try FfiEngine.`init`(path: dest)
            if obsidian {
                _ = try engine.importObsidianVault(source: source)
            } else {
                _ = try engine.importMarkdownFolder(source: source)
            }
            readyEngine = engine
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private func pickDestinationThenRestore() {
        guard let dest = pickFolder(title: "Choose where to clone the vault", createDirectories: true) else { return }
        do {
            let result = try IrisCore.restoreFromBackup(remoteUrl: restoreURL, destPath: dest)
            readyEngine = result.engine
        } catch {
            errorMessage = String(describing: error)
        }
    }
}

// MARK: - Subviews

private struct OnboardingCard: View {
    let path: OnboardingPath
    let isSelected: Bool
    let colors: Palette.Colors
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: Space.xs) {
                ZStack {
                    RoundedRectangle(cornerRadius: Radius.md)
                        .fill(isSelected ? colors.accentTint : colors.bgHover)
                        .frame(width: 32, height: 32)
                    Image(systemName: path.symbol)
                        .foregroundStyle(isSelected ? colors.accentDefault : colors.textSecondary)
                }
                .padding(.bottom, Space.xs)

                Text(path.title)
                    .font(Typography.sans(14, weight: .medium))
                    .foregroundStyle(colors.textPrimary)
                Text(path.description)
                    .font(Typography.bodySmall())
                    .foregroundStyle(colors.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(Space.lg)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(isSelected ? colors.accentTint : colors.bgSurface)
            .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
            .overlay(
                RoundedRectangle(cornerRadius: Radius.lg)
                    .stroke(isSelected ? colors.accentDefault : colors.borderDefault, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}

private struct SourceChip: View {
    let source: ImportSource
    let colors: Palette.Colors
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space.xxs) {
                Text(source.rawValue)
                if !source.isImplemented {
                    Text("soon")
                        .font(Typography.caption())
                        .foregroundStyle(colors.textDisabled)
                }
            }
            .font(Typography.bodySmall())
            .foregroundStyle(source.isImplemented ? colors.textPrimary : colors.textDisabled)
            .padding(.horizontal, Space.md)
            .padding(.vertical, Space.xs + 2)
            .background(colors.bgHover)
            .clipShape(Capsule())
            .overlay(Capsule().stroke(colors.borderDefault, lineWidth: 1))
        }
        .buttonStyle(.plain)
        .disabled(!source.isImplemented)
    }
}

private struct PrimaryButtonStyle: ButtonStyle {
    let colors: Palette.Colors
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(Typography.sans(14, weight: .medium))
            .foregroundStyle(colors.textOnAccent)
            .padding(.horizontal, Space.lg)
            .frame(height: 36)
            .background(configuration.isPressed ? colors.accentPressed : colors.accentDefault)
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }
}

/// Minimal wrapping flow layout for the import source chips — SwiftUI has
/// no built-in one pre-macOS 14's `Layout`-based approach used here.
private struct FlowLayout: Layout {
    var spacing: CGFloat
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > width, x > 0 {
                x = 0; y += rowHeight + spacing; rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: width.isFinite ? width : x, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX; y += rowHeight + spacing; rowHeight = 0
            }
            subview.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}
