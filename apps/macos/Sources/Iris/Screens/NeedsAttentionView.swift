import AppKit
import Foundation
import SwiftUI
import IrisCore

/// Read-only view over `FfiEngine.checkIntegrity()` (T17). A file the cache
/// quarantines (unparseable, or sharing a node id with another file) silently
/// drops out of every list; this is the one place it stays visible. Iris never
/// repairs or deletes anything here (ADR-002/019: the vault always wins) — the
/// only actions are "reveal the file" and "rescan" once the user has fixed it.
///
/// Known limit: malformed files offer no "Open in Dev Mode" because Dev Mode
/// needs a parsed node.
struct NeedsAttentionView: View {
    let engine: FfiEngine

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var report = IntegrityReport(malformedFiles: [], danglingRelations: [], orphanedAnnotations: [])
    @State private var errorMessage: String?

    private var total: Int {
        report.malformedFiles.count + report.danglingRelations.count + report.orphanedAnnotations.count
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                header
                if total == 0 {
                    emptyState
                }
                if !report.malformedFiles.isEmpty {
                    sectionTitle("Files Iris can't read", count: report.malformedFiles.count)
                    ForEach(report.malformedFiles, id: \.path) { file in
                        fileRow(file)
                    }
                }
                if !report.danglingRelations.isEmpty {
                    sectionTitle("Relations pointing at nothing", count: report.danglingRelations.count)
                    ForEach(report.danglingRelations, id: \.self) { relation in
                        row(
                            title: "\(relation.relType) → \(relation.targetId)",
                            detail: "From \(relation.sourcePath). The target id isn't in the vault.",
                            revealPath: relation.sourcePath
                        )
                    }
                }
                if !report.orphanedAnnotations.isEmpty {
                    sectionTitle("Orphaned annotations", count: report.orphanedAnnotations.count)
                    ForEach(report.orphanedAnnotations, id: \.self) { annotation in
                        row(
                            title: "Annotation \(annotation.annotationId)",
                            detail: "\(annotation.path) — \(annotation.reason)",
                            revealPath: annotation.path
                        )
                    }
                }
                if let errorMessage {
                    Text(errorMessage)
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.danger)
                        .padding(.top, Space.md)
                }
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(c.bgCanvas)
        .task { await rescan() }
        .refreshOnVaultChange { Task { await rescan() } }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: Space.xs) {
                Text("Needs Attention").font(Typography.h1()).foregroundStyle(c.textPrimary)
                Text("Problems in your vault that Iris won't fix for you. Files listed here are skipped by every list and search until they're repaired — edit or move them in your editor or Finder, then rescan.")
                    .font(Typography.bodySans())
                    .foregroundStyle(c.textSecondary)
                    .frame(maxWidth: 680, alignment: .leading)
            }
            Spacer()
            Button("Rescan") { Task { await rescan() } }
                .buttonStyle(.plain)
                .font(Typography.bodySans())
                .foregroundStyle(c.accentDefault)
                .padding(.vertical, Space.xs)
                .contentShape(Rectangle())
                .accessibilityLabel("Rescan vault for problems")
        }
        .padding(.bottom, Space.xl)
    }

    private func sectionTitle(_ title: String, count: Int) -> some View {
        Text("\(title.uppercased()) · \(count)")
            .font(Typography.caption())
            .foregroundStyle(c.textSecondary)
            .padding(.top, Space.lg)
            .padding(.bottom, Space.sm)
    }

    private func fileRow(_ file: MalformedFile) -> some View {
        HStack(spacing: Space.md) {
            Circle().fill(c.danger).frame(width: 10, height: 10)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(file.path).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                Text(file.error).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            Spacer()
            revealButton(file.path)
        }
        .padding(.vertical, Space.md)
        .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }
        .accessibilityElement(children: .contain)
    }

    private func revealButton(_ relPath: String) -> some View {
        Button("Reveal in Finder") { reveal(relPath) }
            .buttonStyle(.plain)
            .font(Typography.bodySans())
            .foregroundStyle(c.accentDefault)
            .padding(.vertical, Space.xs)
            .contentShape(Rectangle())
            .accessibilityLabel("Reveal \(relPath) in Finder")
    }

    private func row(title: String, detail: String, revealPath: String) -> some View {
        HStack(spacing: Space.md) {
            Circle().fill(c.warning).frame(width: 10, height: 10)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                Text(detail).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            Spacer()
            revealButton(revealPath)
        }
        .padding(.vertical, Space.md)
        .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }
        .accessibilityElement(children: .contain)
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Image(systemName: "checkmark.circle").font(.system(size: 22)).foregroundStyle(c.textDisabled)
            Text("Nothing needs attention").font(Typography.bodySans()).foregroundStyle(c.textPrimary)
            Text("Every file parsed, every relation resolves. This row only shows in the sidebar while something is wrong.")
                .font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
        .padding(.vertical, Space.xxxl)
    }

    /// Off the main thread: the check walks every file in the vault.
    private func rescan() async {
        let engine = engine
        let result = await Task.detached { Result { try engine.checkIntegrity() } }.value
        switch result {
        case .success(let r):
            report = r
            errorMessage = nil
        case .failure(let error):
            errorMessage = "Couldn't check the vault: \(error)"
        }
    }

    private func reveal(_ relPath: String) {
        let url = URL(fileURLWithPath: engine.vaultRoot()).appendingPathComponent(relPath)
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }
}
