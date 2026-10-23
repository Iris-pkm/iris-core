import Foundation
import IrisCore
import XCTest

final class CacheConcurrencyTests: XCTestCase {
    func testSeparateEnginesCanRebuildSharedCacheConcurrently() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("iris-cache-race-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let appEngine = try FfiEngine.`init`(path: root.path)
        let notes = root.appendingPathComponent("notes")
        try FileManager.default.createDirectory(at: notes, withIntermediateDirectories: true)
        for index in 0..<600 {
            let text = "---\nid: \(String(format: "%026d", index))\ntype: note\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\n---\nNote \(index)\n"
            try text.write(to: notes.appendingPathComponent("\(index).md"), atomically: true, encoding: .utf8)
        }
        let secondEngine = try FfiEngine.open(path: root.path)
        let engines = [appEngine, secondEngine]
        let group = DispatchGroup()
        let lock = NSLock()
        var failed: [Int] = []
        for (index, engine) in engines.enumerated() {
            group.enter()
            DispatchQueue.global().async {
                defer { group.leave() }
                do { try engine.rebuildCache() }
                catch {
                    lock.lock(); failed.append(index); lock.unlock()
                }
            }
        }
        group.wait()
        // A busy writer may fail its first attempt; T15 retries on its own
        // queue. Check that the same retry succeeds once the other writer ends.
        for index in failed { try engines[index].rebuildCache() }
        XCTAssertEqual(try appEngine.search(query: "", nodeType: nil, domain: nil, tag: nil).count, 600)
        XCTAssertEqual(try secondEngine.search(query: "", nodeType: nil, domain: nil, tag: nil).count, 600)
    }
}
