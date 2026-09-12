// swift-tools-version:5.9
import Foundation
import PackageDescription

// ponytail: linking directly against iris-core's freshly-`cargo build`-ed
// debug dylib is a dev-loop shortcut, not a distributable build — before
// packaging a real .app, replace this with an XCFramework (`cargo build
// --release` + `xcodebuild -create-xcframework`) so the binary isn't tied
// to a `cargo build` having already been run in this checkout.
// Resolved relative to this file (repo-root/apps/macos/Package.swift), so
// it works from any checkout/worktree, not just the one it was written in.
let rustTargetDir = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent() // Package.swift -> apps/macos/
    .deletingLastPathComponent() // -> apps/
    .deletingLastPathComponent() // -> repo root
    .appendingPathComponent("target/debug")
    .path

let package = Package(
    name: "Iris",
    platforms: [.macOS(.v14)],
    targets: [
        // Wraps iris-core's UniFFI-generated C header so Swift can see it.
        .systemLibrary(
            name: "iris_coreFFI",
            path: "Sources/iris_coreFFI"
        ),
        // The UniFFI-generated Swift bindings (FfiEngine, FfiNode, etc.) —
        // regenerate via `cargo run --bin uniffi-bindgen` in iris-core
        // whenever the Rust FFI surface changes; never hand-edit this file.
        .target(
            name: "IrisCore",
            dependencies: ["iris_coreFFI"],
            path: "Sources/IrisCore",
            linkerSettings: [
                .unsafeFlags([
                    "-L", rustTargetDir,
                    "-liris_core",
                    "-Xlinker", "-rpath", "-Xlinker", rustTargetDir,
                ])
            ]
        ),
        .executableTarget(
            name: "Iris",
            dependencies: ["IrisCore"],
            path: "Sources/Iris"
        ),
    ]
)
