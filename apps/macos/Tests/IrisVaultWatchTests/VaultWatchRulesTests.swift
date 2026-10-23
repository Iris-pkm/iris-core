import XCTest
@testable import IrisVaultWatch

final class VaultWatchRulesTests: XCTestCase {
    private let root = URL(fileURLWithPath: "/tmp/iris-vault")

    func testRelevantPathsStayInsideVault() {
        XCTAssertEqual(VaultWatchRules.relativePath(root: root, eventPath: "/tmp/iris-vault/tasks/a.md"), "tasks/a.md")
        XCTAssertEqual(VaultWatchRules.relativePath(root: root, eventPath: "/tmp/iris-vault/.iris/computed.yaml"), ".iris/computed.yaml")
        for path in ["/tmp/iris-vault-old/a.md", "/tmp/iris-vault/.git/HEAD", "/tmp/iris-vault/.iris/cache.sqlite-wal", "/tmp/iris-vault/image.png"] {
            XCTAssertNil(VaultWatchRules.relativePath(root: root, eventPath: path), path)
        }
    }

    func testBurstCollapsesToOneSignal() {
        var burst = WatchBurst()
        for index in 0..<51 { burst.insert("tasks/\(index).md") }
        burst.insert("tasks/0.md")
        XCTAssertEqual(burst.drain(), [VaultWatchRules.allPaths])
        XCTAssertEqual(burst.drain(), [])
    }

    func testOwnContentSuppressesDuplicateEventsButExternalContentDoesNot() {
        let own = FileFingerprint(modified: 1, size: 3, digest: "abc")
        let ownAfterMetadataChange = FileFingerprint(modified: 2, size: 3, digest: "abc")
        let externalSameSize = FileFingerprint(modified: 2, size: 3, digest: "xyz")
        var matcher = SelfEchoMatcher()
        matcher.beginWrite(path: "a.md")
        XCTAssertTrue(matcher.suppress(path: "a.md", current: externalSameSize, now: 9))
        matcher.endWrite(path: "a.md")
        matcher.record(path: "a.md", fingerprint: own, now: 10)
        XCTAssertTrue(matcher.suppress(path: "a.md", current: ownAfterMetadataChange, now: 10.5))
        XCTAssertTrue(matcher.suppress(path: "a.md", current: ownAfterMetadataChange, now: 11))
        XCTAssertFalse(matcher.suppress(path: "a.md", current: externalSameSize, now: 11.5))
        matcher.record(path: "a.md", fingerprint: own, now: 20)
        XCTAssertFalse(matcher.suppress(path: "a.md", current: own, now: 24))
    }
}
