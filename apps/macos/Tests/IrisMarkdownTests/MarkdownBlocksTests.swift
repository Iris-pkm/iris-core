import XCTest
@testable import IrisMarkdown

final class MarkdownBlocksTests: XCTestCase {
    private func text(_ b: MarkdownBlock) -> String { String(b.text.characters) }

    func testEmptyAndWhitespaceYieldNoBlocks() {
        XCTAssertTrue(MarkdownBlocks.parse("").isEmpty)
        XCTAssertTrue(MarkdownBlocks.parse("  \n\n ").isEmpty)
    }

    func testHeadingsAndParagraphs() {
        let blocks = MarkdownBlocks.parse("# One\n\n## Two\n\nHello **world**.")
        XCTAssertEqual(blocks.map(\.kind), [.heading(1), .heading(2), .paragraph])
        XCTAssertEqual(text(blocks[2]), "Hello world.")
    }

    func testListsNestingOrdinalsAndMarkers() {
        let blocks = MarkdownBlocks.parse("- a\n  - b\n- c\n\n1. x\n2. y")
        XCTAssertEqual(blocks.map(text), ["a", "b", "c", "x", "y"])
        XCTAssertEqual(blocks[0].list.count, 1)
        XCTAssertEqual(blocks[1].list.count, 2)          // nested
        XCTAssertFalse(blocks[0].list[0].ordered)
        XCTAssertTrue(blocks[3].list[0].ordered)
        XCTAssertEqual(blocks[4].list[0].ordinal, 2)
        XCTAssertTrue(blocks.allSatisfy(\.showsMarker))
    }

    func testChecklistItemsAreStrippedAndFlagged() {
        let blocks = MarkdownBlocks.parse("- [ ] todo\n- [x] done\n- plain")
        XCTAssertEqual(blocks.map(\.check), [.unchecked, .checked, nil])
        XCTAssertEqual(blocks.map(text), ["todo", "done", "plain"])
    }

    func testQuoteDepthAndRule() {
        let blocks = MarkdownBlocks.parse("> quoted\n\n---\n\nafter")
        XCTAssertEqual(blocks[0].quoteDepth, 1)
        XCTAssertEqual(blocks[1].kind, .rule)
        XCTAssertEqual(blocks[2].quoteDepth, 0)
    }

    func testFencedCodeKeepsRawTextAndLanguage() {
        let blocks = MarkdownBlocks.parse("```swift\nlet x = [[not a link]] == y\n```")
        XCTAssertEqual(blocks.count, 1)
        XCTAssertEqual(blocks[0].kind, .code(language: "swift"))
        XCTAssertEqual(text(blocks[0]), "let x = [[not a link]] == y")
    }

    func testTableHeaderAndRows() {
        let blocks = MarkdownBlocks.parse("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |")
        guard case .table(let header, let rows) = blocks[0].kind else { return XCTFail("not a table") }
        XCTAssertEqual(header.map { String($0.characters) }, ["a", "b"])
        XCTAssertEqual(rows.map { $0.map { String($0.characters) } }, [["1", "2"], ["3", "4"]])
    }

    func testWikilinksBecomeLinksWithTargetAndAlias() {
        let blocks = MarkdownBlocks.parse("See [[Project Iris]] and [[Sync Design|the sync doc]] and [[A#Heading]].")
        let links = blocks[0].text.runs.compactMap { run in run.link.flatMap(MarkdownBlocks.wikiTarget(from:)).map { (String(blocks[0].text[run.range].characters), $0) } }
        XCTAssertEqual(links.map(\.0), ["Project Iris", "the sync doc", "A"])
        XCTAssertEqual(links.map(\.1), ["Project Iris", "Sync Design", "A"])
    }

    func testHighlightBecomesMarkLinkAndInlineCodeIsUntouched() {
        let blocks = MarkdownBlocks.parse("a ==hot== b `==raw==` c")
        let marked = blocks[0].text.runs.filter { $0.link?.scheme == MarkdownBlocks.markScheme }
        XCTAssertEqual(marked.map { String(blocks[0].text[$0.range].characters) }, ["hot"])
        XCTAssertTrue(text(blocks[0]).contains("==raw=="))
    }

    func testExternalLinkIsNotAWikilink() {
        let url = URL(string: "https://example.com")!
        XCTAssertNil(MarkdownBlocks.wikiTarget(from: url))
    }
}
