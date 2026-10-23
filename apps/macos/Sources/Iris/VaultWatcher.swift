import CoreServices
import Foundation
import IrisCore
import IrisVaultWatch

/// FSEvents only discovers paths; a separate FFI engine rebuilds the shared
/// derived cache on this queue before the UI receives the change signal.
final class VaultWatcher {
    private let root: URL
    private let queue = DispatchQueue(label: "iris.vault-watcher")
    private let queueKey = DispatchSpecificKey<Bool>()
    private let onChanges: ([String]) -> Void
    private let onError: (String) -> Void
    private var stream: FSEventStreamRef?
    private var burst = WatchBurst()
    private var selfEcho = SelfEchoMatcher()
    private var flushWork: DispatchWorkItem?
    private var needsFullScan = false

    init?(root: URL, onChanges: @escaping ([String]) -> Void, onError: @escaping (String) -> Void) {
        self.root = root.standardizedFileURL
        self.onChanges = onChanges
        self.onError = onError
        queue.setSpecific(key: queueKey, value: true)
        var context = FSEventStreamContext(
            version: 0, info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil, release: nil, copyDescription: nil
        )
        let flags = FSEventStreamCreateFlags(
            kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagNoDefer |
            kFSEventStreamCreateFlagWatchRoot | kFSEventStreamCreateFlagIgnoreSelf
        )
        guard let stream = FSEventStreamCreate(
            kCFAllocatorDefault,
            { _, info, count, eventPaths, eventFlags, _ in
                guard let info else { return }
                let watcher = Unmanaged<VaultWatcher>.fromOpaque(info).takeUnretainedValue()
                watcher.receive(count: count, paths: eventPaths, flags: eventFlags)
            },
            &context, [self.root.path] as CFArray,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow), 0.3, flags
        ) else { return nil }
        self.stream = stream
        FSEventStreamSetDispatchQueue(stream, queue)
        guard FSEventStreamStart(stream) else {
            FSEventStreamInvalidate(stream)
            FSEventStreamRelease(stream)
            self.stream = nil
            return nil
        }
    }

    func recordOwnSave(path: String, fingerprint: FileFingerprint) {
        queue.async { [weak self] in
            self?.selfEcho.record(path: path, fingerprint: fingerprint, now: ProcessInfo.processInfo.systemUptime)
            self?.selfEcho.endWrite(path: path)
        }
    }

    func beginOwnWrite(path: String) {
        queue.async { [weak self] in self?.selfEcho.beginWrite(path: path) }
    }

    func endOwnWrite(path: String) {
        queue.async { [weak self] in self?.selfEcho.endWrite(path: path) }
    }

    func stop() {
        if DispatchQueue.getSpecific(key: queueKey) != nil {
            stopOnQueue()
        } else {
            // A rebuild may be waiting on SQLite. Closing/switching a vault
            // must not wait for that work on SwiftUI's main thread.
            queue.async { self.stopOnQueue() }
        }
    }

    private func stopOnQueue() {
        flushWork?.cancel()
        flushWork = nil
        if let stream {
            FSEventStreamStop(stream)
            FSEventStreamInvalidate(stream)
            FSEventStreamRelease(stream)
            self.stream = nil
        }
    }

    deinit {
        if DispatchQueue.getSpecific(key: queueKey) != nil {
            stopOnQueue()
        } else {
            queue.sync { stopOnQueue() }
        }
    }

    private func receive(
        count: Int, paths: UnsafeMutableRawPointer, flags: UnsafePointer<FSEventStreamEventFlags>
    ) {
        let names = paths.assumingMemoryBound(to: UnsafePointer<CChar>?.self)
        for index in 0..<count {
            if flags[index] & FSEventStreamEventFlags(
                kFSEventStreamEventFlagMustScanSubDirs | kFSEventStreamEventFlagRootChanged
            ) != 0 {
                needsFullScan = true
                continue
            }
            if flags[index] & FSEventStreamEventFlags(kFSEventStreamEventFlagItemIsDir) != 0 {
                continue
            }
            guard let name = names[index],
                  let path = VaultWatchRules.relativePath(root: root, eventPath: String(cString: name)) else { continue }
            burst.insert(path)
        }
        guard needsFullScan || !burst.isEmpty else { return }
        flushWork?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.flush() }
        flushWork = work
        queue.asyncAfter(deadline: .now() + .milliseconds(300), execute: work)
    }

    private func flush() {
        flushWork = nil
        let pending = burst.drain()
        let candidates = needsFullScan ? [VaultWatchRules.allPaths] : pending
        needsFullScan = false
        guard !candidates.isEmpty else { return }
        let now = ProcessInfo.processInfo.systemUptime
        let changed = candidates.filter { path in
            guard path != VaultWatchRules.allPaths else { return true }
            let current = FileFingerprint.read(root.appendingPathComponent(path))
            return !selfEcho.suppress(path: path, current: current, now: now)
        }
        guard !changed.isEmpty else { return }

        if changed.contains(where: { $0 == VaultWatchRules.allPaths || $0.hasSuffix(".md") }) {
            do {
                try rebuildCache()
            } catch {
                onError("Could not refresh the vault cache: \(error)")
                return
            }
        }
        onChanges(changed)
    }

    private func rebuildCache() throws {
        var lastError: Error?
        for attempt in 0..<2 {
            do {
                let engine = try FfiEngine.open(path: root.path)
                try engine.rebuildCache()
                return
            } catch {
                lastError = error
                if attempt == 0 { Thread.sleep(forTimeInterval: 0.2) }
            }
        }
        throw lastError ?? NSError(domain: "IrisVaultWatcher", code: 1)
    }
}
