import SwiftUI
import IrisCore

/// The Node Editor's right panel "Dev Mode" tab (`design/canvas/DevMode.dc.html`,
/// embedded in `RightRail`) — the node's raw frontmatter exactly as written
/// on disk, for debugging a plugin's or the AI distillation layer's actual
/// output rather than a rendered interpretation of it.
///
/// Backed by `FfiParsedNode.raw_frontmatter` (new this pass — `readNode`
/// previously only surfaced the parsed `FfiNode` + body, never the
/// preserved raw YAML text `parser.rs` already keeps for lossless
/// round-trip).
struct DevModeView: View {
    let engine: FfiEngine
    let relPath: String

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var rawFrontmatter: String = ""
    @State private var copied = false

    var body: some View {
        VStack(alignment: .leading, spacing: Space.md) {
            ScrollView([.vertical, .horizontal]) {
                Text(rawFrontmatter)
                    .font(Typography.mono(11))
                    .foregroundStyle(c.textPrimary)
                    .lineSpacing(6)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(Space.md)
            }
            .background(c.bgHover)
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
            .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
            .frame(maxHeight: .infinity)

            Text("Raw frontmatter, read from the file as-is — this is what a plugin or the AI actually wrote, not a rendered view.")
                .font(.system(size: 11.5))
                .foregroundStyle(c.textSecondary)
                .lineSpacing(4)

            copyButton
        }
        .task(id: relPath) { load() }
    }

    private var copyButton: some View {
        Button {
            copyToPasteboard()
        } label: {
            Text(copied ? "Copied" : "Copy raw")
                .font(.system(size: 12.5))
                .foregroundStyle(c.accentDefault)
                .frame(maxWidth: .infinity)
                .padding(Space.sm)
                .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
        }
        .buttonStyle(.plain)
    }

    private func load() {
        copied = false
        rawFrontmatter = (try? engine.readNode(relPath: relPath))?.rawFrontmatter ?? ""
    }

    private func copyToPasteboard() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(rawFrontmatter, forType: .string)
        copied = true
    }
}
