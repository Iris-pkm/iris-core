import SwiftUI
import IrisCore

/// Plain-language surface over the vault's actual git tags and local
/// branches. The engine owns checkout's cache rebuild and undo/redo reset.
struct HistoryView: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var branches: [String] = []
    @State private var checkpoints: [String] = []
    @State private var currentBranch: String?
    @State private var branchName = ""
    @State private var checkpointName = ""
    @State private var addingBranch = false
    @State private var addingCheckpoint = false
    @State private var errorMessage: String?

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.xxl) {
                    header
                    section(title: "Branches", buttonTitle: addingBranch ? "Cancel" : "+ New branch (off current HEAD)", adding: $addingBranch, name: $branchName, placeholder: "branch-name", save: createBranch) {
                        if branches.isEmpty { empty("No branches yet.") }
                        ForEach(branches, id: \.self) { branch in
                            row(branch, symbol: "arrow.triangle.branch", current: branch == currentBranch, actionTitle: "Switch") { checkout(branch) }
                        }
                    }
                    section(title: "Checkpoints", buttonTitle: addingCheckpoint ? "Cancel" : "+ Tag current HEAD", adding: $addingCheckpoint, name: $checkpointName, placeholder: "checkpoint-name", save: createCheckpoint) {
                        if checkpoints.isEmpty { empty("No checkpoints yet.") }
                        ForEach(checkpoints, id: \.self) { checkpoint in
                            row(checkpoint, symbol: "diamond", current: false, actionTitle: "View") { checkout(checkpoint) }
                        }
                    }
                    if let errorMessage { Text(errorMessage).font(Typography.bodySmall()).foregroundStyle(c.danger) }
                }
                .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            Divider().background(c.borderDefault)
            aboutRail
        }
        .background(c.bgCanvas)
        .task { reload() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Space.xs) {
            Text("History").font(Typography.h1()).foregroundStyle(c.textPrimary)
            Text("Named checkpoints and branches over the vault’s own git history — a plain-language surface over real git tags and branches.")
                .font(Typography.bodySans()).foregroundStyle(c.textSecondary).frame(maxWidth: 700, alignment: .leading)
            Text(currentBranch.map { "On branch \($0) — new changes commit here." } ?? "Viewing a checkpoint — this is read-only until you branch from here.")
                .font(Typography.bodySans()).foregroundStyle(currentBranch == nil ? c.warning : c.success)
                .frame(maxWidth: .infinity, alignment: .leading).padding(Space.md)
                .background(currentBranch == nil ? c.warningTint : c.successTint).clipShape(RoundedRectangle(cornerRadius: Radius.md)).padding(.top, Space.sm)
        }
    }

    private func section<Content: View>(title: String, buttonTitle: String, adding: Binding<Bool>, name: Binding<String>, placeholder: String, save: @escaping () -> Void, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: Space.md) {
            HStack {
                Text(title).font(Typography.serif(18, weight: .semibold)).foregroundStyle(c.textPrimary)
                Spacer()
                Button(buttonTitle) { adding.wrappedValue.toggle(); name.wrappedValue = "" }.buttonStyle(.bordered).tint(c.accentDefault)
            }
            if adding.wrappedValue {
                HStack(spacing: Space.sm) {
                    TextField(placeholder, text: name).textFieldStyle(.roundedBorder).onSubmit(save)
                    Button("Create", action: save).buttonStyle(.borderedProminent).tint(c.accentDefault).disabled(name.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            content()
        }
    }

    private func row(_ name: String, symbol: String, current: Bool, actionTitle: String, action: @escaping () -> Void) -> some View {
        HStack(spacing: Space.md) {
            Image(systemName: symbol).font(.system(size: 13)).foregroundStyle(c.textSecondary).frame(width: 18)
            Text(name).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
            Spacer()
            if current {
                Text("Current").font(Typography.caption()).foregroundStyle(c.textSecondary).padding(.horizontal, Space.sm).padding(.vertical, Space.xxs).background(c.bgHover).clipShape(Capsule())
            } else {
                Button(actionTitle, action: action).buttonStyle(.borderedProminent).tint(c.accentDefault)
            }
        }
        .padding(Space.md).background(current ? c.bgSelected : Color.clear).clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func empty(_ text: String) -> some View { Text(text).font(Typography.bodySmall()).foregroundStyle(c.textDisabled) }

    private var aboutRail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            explanation("About checkpoints", "A lightweight git tag on the current HEAD — a named snapshot, not a branch.")
            Divider().background(c.borderDefault)
            explanation("About branches", "Branches off the current HEAD without switching to it — try a different reorganization without committing to it.")
            Divider().background(c.borderDefault)
            explanation("Switching", "Checking out a branch or checkpoint rebuilds the cache and clears undo/redo — those stacks describe content on the line of history you just left.")
            Spacer()
        }.padding(Space.lg).frame(width: 300).background(c.bgSurface)
    }

    private func explanation(_ title: String, _ text: String) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            Text(text).font(Typography.bodySans()).foregroundStyle(c.textSecondary).lineSpacing(4)
        }
    }

    private func createBranch() {
        let name = branchName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        perform { try engine.createBranch(name: name) }; addingBranch = false; branchName = ""
    }
    private func createCheckpoint() {
        let name = checkpointName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        perform { try engine.createCheckpoint(name: name) }; addingCheckpoint = false; checkpointName = ""
    }
    private func checkout(_ name: String) { perform { try engine.checkout(name: name) } }
    private func perform(_ action: () throws -> Void) { do { try action(); reload() } catch { errorMessage = String(describing: error) } }
    private func reload() {
        do { branches = try engine.listBranches(); checkpoints = try engine.listCheckpoints(); currentBranch = try engine.currentBranch(); errorMessage = nil }
        catch { errorMessage = String(describing: error) }
    }
}
