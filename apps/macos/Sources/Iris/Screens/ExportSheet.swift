import AppKit
import SwiftUI
import UniformTypeIdentifiers
import IrisCore

/// Current-node Stage 1 export. Project/report scope is deliberately absent:
/// the backend's IDM currently resolves one node, so the modal does not imply
/// it can package related nodes yet.
struct ExportSheet: View {
    let engine: FfiEngine
    let relPath: String
    let title: String
    let dismiss: () -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var format: Format = .pdf
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            Text("Export").font(Typography.h1()).foregroundStyle(c.textPrimary)

            Text("SCOPE").font(Typography.caption()).foregroundStyle(c.textSecondary)
            Text("This note only · \(title)").font(Typography.bodySans()).foregroundStyle(c.textPrimary)

            Text("FORMAT").font(Typography.caption()).foregroundStyle(c.textSecondary)
            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: Space.sm) {
                ForEach(Format.allCases) { option in
                    Button {
                        format = option
                    } label: {
                        VStack(spacing: Space.xxs) {
                            Text(option.title).font(Typography.sans(14, weight: .medium))
                            Text(option.detail).font(Typography.caption()).foregroundStyle(format == option ? c.accentDefault : c.textSecondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, Space.sm)
                        .background(format == option ? c.bgSelected : c.bgHover)
                        .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(format == option ? c.accentDefault : c.borderDefault, lineWidth: 1))
                        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
                    }
                    .buttonStyle(.plain)
                }
            }

            Text("One shared document model powers every writer, so the same note stays consistent across formats.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            if let errorMessage {
                Text(errorMessage).font(Typography.bodySmall()).foregroundStyle(c.danger)
            }

            HStack {
                Button("Cancel", action: dismiss)
                Spacer()
                Button("Export \(format.title)") { save() }
                    .buttonStyle(.borderedProminent)
            }
        }
        .padding(Space.xl)
        .frame(width: 500)
        .background(c.bgSurfaceRaised)
    }

    private func save() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "\(title).\(format.extension)"
        panel.allowedContentTypes = [format.contentType]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let data = try engine.exportNode(relPath: relPath, format: format.ffiFormat)
            try data.write(to: url, options: .atomic)
            dismiss()
        } catch {
            errorMessage = String(describing: error)
        }
    }

    private enum Format: String, CaseIterable, Identifiable {
        case html, pdf, json, csv
        var id: String { rawValue }
        var title: String { rawValue.uppercased() }
        var detail: String { self == .pdf ? "Printable" : "Stage 1" }
        var `extension`: String { rawValue }
        var contentType: UTType { UTType(filenameExtension: `extension`) ?? .data }
        var ffiFormat: FfiExportFormat {
            switch self {
            case .html: return .html
            case .pdf: return .pdf
            case .json: return .json
            case .csv: return .csv
            }
        }
    }
}
