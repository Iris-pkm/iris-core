// swift-tools-version:5.9
import PackageDescription

// IrisCoreFFI.xcframework wraps the release iris-core dylib (install name
// fixed to @rpath, ad-hoc signed) plus its UniFFI-generated C header —
// built via `cargo build --release --lib` + `xcodebuild -create-xcframework`
// (see README.md's "Rebuilding the XCFramework" section for the exact
// commands). Distributable: this checkout doesn't need a prior `cargo
// build` to have been run, unlike the old raw -L/-liris_core dev-loop hack.
// Ceiling, flagged: macos-arm64 only (this dev machine's architecture) —
// a universal/x86_64 slice needs building on or cross-compiling for Intel,
// tracked separately as part of ADR-029's five-target-triple pipeline.
let package = Package(
    name: "Iris",
    platforms: [.macOS(.v14)],
    targets: [
        .binaryTarget(
            name: "iris_coreFFI",
            path: "IrisCoreFFI.xcframework"
        ),
        // The UniFFI-generated Swift bindings (FfiEngine, FfiNode, etc.) —
        // regenerate via `cargo run --bin uniffi-bindgen` in iris-core
        // whenever the Rust FFI surface changes; never hand-edit this file.
        .target(
            name: "IrisCore",
            dependencies: ["iris_coreFFI"],
            path: "Sources/IrisCore"
        ),
        .executableTarget(
            name: "Iris",
            dependencies: ["IrisCore"],
            path: "Sources/Iris"
        ),
    ]
)
