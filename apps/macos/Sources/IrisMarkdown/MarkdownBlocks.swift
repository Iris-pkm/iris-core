import Foundation

/// One rendered block of a note body (Tier A, ADR-027). Pure data, no UI —
/// `Iris/Screens/MarkdownBodyView.swift` turns these into SwiftUI.
///
/// Parsing is Apple's own CommonMark/GFM parser (`AttributedString(markdown:)`,
/// no third-party dependency); this file only regroups its per-run
/// `PresentationIntent`s into blocks and adds the three Iris extensions that
/// parser lacks: `- [ ]` checklists, `[[wikilinks]]`, and `==highlight==`.
public struct MarkdownBlock: Identifiable, Equatable {
    public enum Kind: Equatable {
        case paragraph
        case heading(Int)
        case code(language: String?)
        case rule
        case table(header: [AttributedString], rows: [[AttributedString]])
    }
    public enum Check: Equatable { case unchecked, checked }
    public struct ListLevel: Equatable {
        public let ordered: Bool
        public let ordinal: Int
    }

    public let id: Int
    public var kind: Kind
    /// Inline-styled text (bold/italic/code/strikethrough/links). For a code
    /// block this is the raw code; for a table it is empty.
    public var text: AttributedString
    public var quoteDepth: Int
    /// Outermost → innermost; empty when the block is not in a list.
    public var list: [ListLevel]
    /// True for the first block of a list item (the one that draws the bullet).
    public var showsMarker: Bool
    public var check: Check?
    /// 0-based position among all checklist blocks in document order — the key
    /// `MarkdownBlocks.toggleCheck` uses to find the matching `[ ]` in the raw source.
    public var checkIndex: Int?

    public var isListItem: Bool { !list.isEmpty }
}

public enum MarkdownBlocks {
    public static let wikiScheme = "iris-wiki"
    public static let markScheme = "iris-mark"

    /// The note name a `[[wikilink]]` points at, or nil for any other URL.
    public static func wikiTarget(from url: URL) -> String? {
        let prefix = wikiScheme + ":"
        guard url.absoluteString.hasPrefix(prefix) else { return nil }
        return String(url.absoluteString.dropFirst(prefix.count)).removingPercentEncoding
    }

    public static func parse(_ source: String) -> [MarkdownBlock] {
        let prepared = preprocess(source)
        guard !prepared.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return [] }
        guard let attributed = try? AttributedString(
            markdown: prepared,
            options: .init(interpretedSyntax: .full, failurePolicy: .returnPartiallyParsedIfPossible)
        ) else {
            // Unparseable: show the source as plain paragraphs rather than nothing.
            return prepared.components(separatedBy: "\n\n").enumerated().map { index, chunk in
                MarkdownBlock(id: index, kind: .paragraph, text: AttributedString(chunk),
                              quoteDepth: 0, list: [], showsMarker: false, check: nil)
            }
        }
        return blocks(from: attributed)
    }

    // MARK: - Grouping runs into blocks

    private enum Key: Equatable { case block(Int), table(Int) }
    private struct Group {
        let key: Key
        let intents: [PresentationIntent.IntentType]
        var runs: [(text: AttributedString, intents: [PresentationIntent.IntentType])]
    }

    private static func blocks(from attributed: AttributedString) -> [MarkdownBlock] {
        var groups: [Group] = []
        for run in attributed.runs {
            let intents = run.presentationIntent?.components ?? []
            let key: Key
            if let table = intents.first(where: { if case .table = $0.kind { return true }; return false }) {
                key = .table(table.identity)
            } else {
                key = .block(intents.first?.identity ?? -1)
            }
            let slice = AttributedString(attributed[run.range])
            if let last = groups.last, last.key == key {
                groups[groups.count - 1].runs.append((slice, intents))
            } else {
                groups.append(Group(key: key, intents: intents, runs: [(slice, intents)]))
            }
        }

        var seenItems = Set<Int>()
        var checkCount = 0
        var result: [MarkdownBlock] = []
        for (index, group) in groups.enumerated() {
            if case .table = group.key {
                result.append(table(from: group, id: index))
                continue
            }
            let intents = group.intents
            var text = AttributedString()
            for run in group.runs { text.append(run.text) }

            let kind: MarkdownBlock.Kind
            switch intents.first?.kind {
            case .header(let level): kind = .heading(level)
            case .codeBlock(let hint): kind = .code(language: (hint?.isEmpty ?? true) ? nil : hint)
            case .thematicBreak: kind = .rule
            default: kind = .paragraph
            }
            if case .code = kind {
                var raw = String(text.characters)
                if raw.hasSuffix("\n") { raw.removeLast() }
                text = AttributedString(raw)
            }

            var levels: [MarkdownBlock.ListLevel] = []
            var ordered = false
            for intent in intents.reversed() {
                switch intent.kind {
                case .orderedList: ordered = true
                case .unorderedList: ordered = false
                case .listItem(let ordinal): levels.append(.init(ordered: ordered, ordinal: ordinal))
                default: break
                }
            }
            let quoteDepth = intents.filter { if case .blockQuote = $0.kind { return true }; return false }.count
            let itemID = intents.first(where: { if case .listItem = $0.kind { return true }; return false })?.identity
            let showsMarker = itemID.map { seenItems.insert($0).inserted } ?? false

            var check: MarkdownBlock.Check?
            if !levels.isEmpty, case .paragraph = kind {
                let s = String(text.characters)
                for (prefix, value) in [("[ ] ", MarkdownBlock.Check.unchecked), ("[x] ", .checked), ("[X] ", .checked)]
                where s.hasPrefix(prefix) {
                    check = value
                    text.removeSubrange(text.startIndex..<text.characters.index(text.startIndex, offsetBy: prefix.count))
                    break
                }
            }

            var checkIndex: Int?
            if check != nil { checkIndex = checkCount; checkCount += 1 }
            result.append(MarkdownBlock(id: index, kind: kind, text: text, quoteDepth: quoteDepth,
                                        list: levels, showsMarker: showsMarker, check: check,
                                        checkIndex: checkIndex))
        }
        return result
    }

    private static func table(from group: Group, id: Int) -> MarkdownBlock {
        // header row = -1, body rows by their rowIndex; cells by column index.
        var cells: [Int: [Int: AttributedString]] = [:]
        for run in group.runs {
            var row = -1
            var column = 0
            for intent in run.intents {
                switch intent.kind {
                case .tableCell(let c): column = c
                case .tableRow(let r): row = r
                default: break
                }
            }
            cells[row, default: [:]][column, default: AttributedString()].append(run.text)
        }
        func line(_ row: Int) -> [AttributedString] {
            guard let columns = cells[row] else { return [] }
            return (0...(columns.keys.max() ?? 0)).map { columns[$0] ?? AttributedString() }
        }
        let bodyRows = cells.keys.filter { $0 >= 0 }.sorted().map(line)
        return MarkdownBlock(id: id, kind: .table(header: line(-1), rows: bodyRows), text: AttributedString(),
                             quoteDepth: 0, list: [], showsMarker: false, check: nil)
    }

    // MARK: - Toggling a checklist box

    private static let checkRegex = try! NSRegularExpression(
        pattern: #"^(\s*(?:>\s*)*(?:[-*+]|\d{1,9}[.)])\s+\[)([ xX])(\]\s+\S)"#)

    /// Flips the `index`th checklist box (see `MarkdownBlock.checkIndex`) in the
    /// raw `source` and returns the new source, changing exactly that one
    /// character — nothing else in the note moves (ADR-019). Fenced code is
    /// skipped, as in `preprocess`. Returns nil if there is no such box.
    // Ceiling: relies on this scanner and `blocks(from:)` agreeing on what a
    // checklist item is (ordinal matching); the tests pin the shared cases.
    public static func toggleCheck(in source: String, index: Int) -> String? {
        var inFence = false
        var seen = 0
        var lines = source.components(separatedBy: "\n")
        for (n, line) in lines.enumerated() {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") { inFence.toggle(); continue }
            if inFence { continue }
            let ns = line as NSString
            guard let m = checkRegex.firstMatch(in: line, range: NSRange(location: 0, length: ns.length)) else { continue }
            if seen == index {
                let flipped = ns.substring(with: m.range(at: 2)) == " " ? "x" : " "
                lines[n] = ns.replacingCharacters(in: m.range(at: 2), with: flipped)
                return lines.joined(separator: "\n")
            }
            seen += 1
        }
        return nil
    }

    // MARK: - Iris extensions (wikilinks, highlight), applied before parsing

    /// Rewrites `[[Target|alias]]` and `==text==` into ordinary links with
    /// private schemes. Fenced code and inline code spans are left alone.
    static func preprocess(_ source: String) -> String {
        var inFence = false
        var out: [String] = []
        for line in source.split(separator: "\n", omittingEmptySubsequences: false) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                inFence.toggle()
                out.append(String(line))
            } else if inFence {
                out.append(String(line))
            } else {
                var segments = String(line).components(separatedBy: "`")
                for i in stride(from: 0, to: segments.count, by: 2) { segments[i] = rewrite(segments[i]) }
                out.append(segments.joined(separator: "`"))
            }
        }
        return out.joined(separator: "\n")
    }

    private static let wikiRegex = try! NSRegularExpression(pattern: #"\[\[([^\]\|#]+)(?:#[^\]\|]*)?(?:\|([^\]]+))?\]\]"#)
    private static let markRegex = try! NSRegularExpression(pattern: #"==([^=\n]+?)=="#)

    private static func rewrite(_ text: String) -> String {
        var result = text
        let ns = result as NSString
        var pieces: [(NSRange, String)] = []
        for match in wikiRegex.matches(in: result, range: NSRange(location: 0, length: ns.length)) {
            let target = ns.substring(with: match.range(at: 1)).trimmingCharacters(in: .whitespaces)
            let alias = match.range(at: 2).location == NSNotFound ? target : ns.substring(with: match.range(at: 2))
            let encoded = target.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? target
            pieces.append((match.range, "[\(alias)](\(wikiScheme):\(encoded))"))
        }
        for (range, replacement) in pieces.reversed() {
            result = (result as NSString).replacingCharacters(in: range, with: replacement)
        }
        return markRegex.stringByReplacingMatches(
            in: result, range: NSRange(location: 0, length: (result as NSString).length),
            withTemplate: "[$1](\(markScheme):h)")
    }
}
