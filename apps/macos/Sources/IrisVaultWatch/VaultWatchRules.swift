import CryptoKit
import Foundation

/// Pure event decisions shared by the FSEvents adapter and its tests.
public enum VaultWatchRules {
    public static let allPaths = "*"

    public static func relativePath(root: URL, eventPath: String) -> String? {
        let rootPath = root.standardizedFileURL.path
        let path = URL(fileURLWithPath: eventPath).standardizedFileURL.path
        guard path.hasPrefix(rootPath + "/") else { return nil }
        let relative = String(path.dropFirst(rootPath.count + 1))
        guard relative != ".git", !relative.hasPrefix(".git/") else { return nil }
        guard relative != ".iris/cache.sqlite", !relative.hasPrefix(".iris/cache.sqlite") else { return nil }
        guard relative.hasSuffix(".md") || relative == ".iris/computed.yaml" else { return nil }
        return relative
    }
}

public struct FileFingerprint: Equatable {
    public let modified: TimeInterval
    public let size: Int
    public let digest: String

    public init(modified: TimeInterval, size: Int, digest: String) {
        self.modified = modified
        self.size = size
        self.digest = digest
    }

    public static func read(_ url: URL) -> FileFingerprint? {
        guard let data = try? Data(contentsOf: url),
              let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
              let modified = attributes[.modificationDate] as? Date else { return nil }
        let digest = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        return FileFingerprint(modified: modified.timeIntervalSince1970, size: data.count, digest: digest)
    }

    /// The content hash also catches same-size, same-mtime external writes.
    public func hasSameContent(as other: FileFingerprint) -> Bool {
        size == other.size && digest == other.digest
    }
}

public struct WatchBurst {
    private var paths = Set<String>()
    public var isEmpty: Bool { paths.isEmpty }

    public init() {}

    public mutating func insert(_ path: String) { paths.insert(path) }

    public mutating func drain(limit: Int = 50) -> [String] {
        defer { paths.removeAll() }
        return paths.count > limit ? [VaultWatchRules.allPaths] : paths.sorted()
    }
}

public struct SelfEchoMatcher {
    private var own: [String: (fingerprint: FileFingerprint, time: TimeInterval)] = [:]
    private var writing = Set<String>()

    public init() {}

    public mutating func beginWrite(path: String) { writing.insert(path) }

    public mutating func endWrite(path: String) { writing.remove(path) }

    public mutating func record(path: String, fingerprint: FileFingerprint, now: TimeInterval) {
        own[path] = (fingerprint, now)
    }

    public mutating func suppress(path: String, current: FileFingerprint?, now: TimeInterval) -> Bool {
        if writing.contains(path) { return true }
        guard let saved = own[path] else { return false }
        guard now - saved.time <= 3, let current else {
            own.removeValue(forKey: path)
            return false
        }
        if saved.fingerprint.hasSameContent(as: current) { return true }
        own.removeValue(forKey: path)
        return false
    }
}
