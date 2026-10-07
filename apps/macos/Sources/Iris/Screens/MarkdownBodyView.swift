import SwiftUI
import IrisMarkdown

/// Read-only rendering of a note body (Tier A, ADR-027): headings, paragraphs
/// with inline styling, bullet/numbered/nested lists, checklists, block
/// quotes, fenced code, GFM tables, rules, `[[wikilinks]]` and `==highlight==`.
/// Parsing lives in the testable `IrisMarkdown` target; this file is layout.
///
/// **Not built, flagged:** LaTeX math, callouts, footnotes, embeds, syntax
/// highlighting in code blocks, raw HTML. Clicking a checklist box calls
/// `onToggleCheck` with the box's `checkIndex`; the owner rewrites the source
/// (`MarkdownBlocks.toggleCheck`). Other editing is the plain-text mode in
/// `NodeEditorView`.
struct MarkdownBodyView: View {
    let source: String
    let onWikiLink: (String) -> Void
    var onToggleCheck: ((Int) -> Void)?

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        let blocks = MarkdownBlocks.parse(source)
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(blocks.enumerated()), id: \.element.id) { index, block in
                row(block)
                    .padding(.top, index == 0 ? 0 : gap(before: block, after: blocks[index - 1]))
            }
        }
        .frame(maxWidth: 640, alignment: .leading)
        .textSelection(.enabled)
        .environment(\.openURL, OpenURLAction { url in
            if let target = MarkdownBlocks.wikiTarget(from: url) {
                onWikiLink(target)
                return .handled
            }
            return .systemAction
        })
    }

    private func gap(before block: MarkdownBlock, after previous: MarkdownBlock) -> CGFloat {
        if case .heading = block.kind { return Space.xl }
        if block.isListItem && previous.isListItem { return Space.xs }
        return Space.md
    }

    @ViewBuilder
    private func row(_ block: MarkdownBlock) -> some View {
        switch block.kind {
        case .rule:
            Divider().background(c.borderDefault).padding(.vertical, Space.sm)
        case .heading(let level):
            Text(styled(block.text)).font(headingFont(level)).foregroundStyle(c.textPrimary)
                .accessibilityAddTraits(.isHeader)
        case .code(let language):
            codeBlock(block.text, language: language)
        case .table(let header, let rows):
            table(header: header, rows: rows)
        case .paragraph:
            paragraph(block)
        }
    }

    private func headingFont(_ level: Int) -> Font {
        switch level {
        case 1: return Typography.serif(24, weight: .semibold)
        case 2: return Typography.serif(19, weight: .semibold)
        case 3: return Typography.sans(15, weight: .semibold)
        default: return Typography.sans(14, weight: .semibold)
        }
    }

    private func paragraph(_ block: MarkdownBlock) -> some View {
        let text = Text(styled(block.text))
            .font(Typography.serif(16))
            .foregroundStyle(block.quoteDepth > 0 ? c.textSecondary : c.textPrimary)
            .lineSpacing(6)
        return HStack(alignment: .firstTextBaseline, spacing: Space.sm) {
            if block.isListItem {
                marker(block).frame(width: 20, alignment: .trailing)
            }
            text.fixedSize(horizontal: false, vertical: true)
        }
        .padding(.leading, CGFloat(max(0, block.list.count - 1)) * 22)
        .padding(.leading, CGFloat(block.quoteDepth) * 14)
        .overlay(alignment: .leading) {
            if block.quoteDepth > 0 {
                Rectangle().fill(c.borderStrong).frame(width: 2)
            }
        }
    }

    @ViewBuilder
    private func marker(_ block: MarkdownBlock) -> some View {
        if !block.showsMarker {
            Color.clear
        } else if let check = block.check {
            Button {
                if let i = block.checkIndex { onToggleCheck?(i) }
            } label: {
                Image(systemName: check == .checked ? "checkmark.square.fill" : "square")
                    .foregroundStyle(check == .checked ? c.accentDefault : c.textSecondary)
            }
            .buttonStyle(.plain)
            .disabled(onToggleCheck == nil)
            .accessibilityLabel(check == .checked ? "Done" : "Not done")
            .accessibilityAddTraits(.isToggle)
        } else if let level = block.list.last, level.ordered {
            Text("\(level.ordinal).").font(Typography.serif(16)).monospacedDigit().foregroundStyle(c.textSecondary)
        } else {
            Text(["•", "◦", "▪"][min(block.list.count - 1, 2)]).font(Typography.serif(16)).foregroundStyle(c.textSecondary)
        }
    }

    private func codeBlock(_ code: AttributedString, language: String?) -> some View {
        VStack(alignment: .leading, spacing: Space.xs) {
            if let language {
                Text(language.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            }
            ScrollView(.horizontal, showsIndicators: false) {
                Text(String(code.characters)).font(Typography.mono(13)).foregroundStyle(c.textPrimary)
                    .fixedSize(horizontal: true, vertical: true)
            }
        }
        .padding(Space.md)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(c.bgSurface)
        .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func table(header: [AttributedString], rows: [[AttributedString]]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            Grid(alignment: .leading, horizontalSpacing: 0, verticalSpacing: 0) {
                if !header.isEmpty {
                    GridRow { ForEach(Array(header.enumerated()), id: \.offset) { cell($1, header: true) } }
                }
                ForEach(Array(rows.enumerated()), id: \.offset) { _, cells in
                    GridRow { ForEach(Array(cells.enumerated()), id: \.offset) { cell($1, header: false) } }
                }
            }
            .overlay(Rectangle().stroke(c.borderDefault, lineWidth: 1))
        }
    }

    private func cell(_ text: AttributedString, header: Bool) -> some View {
        Text(styled(text))
            .font(header ? Typography.sans(14, weight: .semibold) : Typography.sans(14))
            .foregroundStyle(c.textPrimary)
            .padding(.horizontal, Space.md).padding(.vertical, Space.sm)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(header ? c.bgHover : Color.clear)
            .overlay(Rectangle().stroke(c.borderDefault, lineWidth: 0.5))
    }

    /// Link and highlight colouring from the palette (the parser only tags them).
    private func styled(_ text: AttributedString) -> AttributedString {
        var out = text
        for run in text.runs {
            guard let link = run.link else { continue }
            if link.scheme == MarkdownBlocks.markScheme {
                out[run.range].link = nil
                out[run.range].backgroundColor = c.warningTint
            } else {
                out[run.range].foregroundColor = c.accentDefault
                out[run.range].underlineStyle = .single
            }
        }
        return out
    }
}
