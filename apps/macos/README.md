# Iris — macOS (SwiftUI)

Native macOS shell, per ADR-031 (no webview). Consumes `iris-core` directly through the real UniFFI `FfiEngine` surface — no backend logic lives here, only UI and the SwiftUI ↔ `FfiEngine` wiring.

## Build & run

```bash
cd apps/macos && swift build
swift run Iris
```

No prior `cargo build` needed — `IrisCoreFFI.xcframework` (checked in) already carries a built `iris-core` release dylib plus its C header. **After any `iris-core` change**, rebuild the xcframework (below) before `swift build` will pick it up; there's no automatic dependency from Swift's build graph back to the Rust source.

## Rebuilding the XCFramework

Whenever `iris-core`'s Rust source changes (not just the FFI surface — any change needs a fresh dylib):

```bash
# 1. Build the release dylib
cd iris-core && cargo build --release --lib

# 2. Fix its install name so it's relocatable (Rust's default is an absolute
#    build-machine path, which breaks the moment this leaves this checkout)
#    and ad-hoc sign it (unsigned dylibs won't load into a signed app).
cp ../target/release/libiris_core.dylib /tmp/libiris_core.dylib
install_name_tool -id @rpath/libiris_core.dylib /tmp/libiris_core.dylib
codesign --sign - --force /tmp/libiris_core.dylib

# 3. Regenerate the Swift bindings + C header from that same dylib.
cargo run --release --bin uniffi-bindgen -- generate \
  --library /tmp/libiris_core.dylib --language swift --out-dir /tmp/iris-swift-gen
cp /tmp/iris-swift-gen/iris_core.swift ../apps/macos/Sources/IrisCore/

# 4. Stage headers (the generated modulemap must be named exactly
#    `module.modulemap` for Clang to find it) and rebuild the xcframework.
mkdir -p /tmp/iris-xcframework-headers
cp /tmp/iris-swift-gen/iris_coreFFI.h /tmp/iris-xcframework-headers/
cp /tmp/iris-swift-gen/iris_coreFFI.modulemap /tmp/iris-xcframework-headers/module.modulemap
rm -rf ../apps/macos/IrisCoreFFI.xcframework
xcodebuild -create-xcframework \
  -library /tmp/libiris_core.dylib -headers /tmp/iris-xcframework-headers \
  -output ../apps/macos/IrisCoreFFI.xcframework
```

`Sources/IrisCore/iris_core.swift` is a **generated file — never hand-edit it.** The xcframework's own header/modulemap (inside `IrisCoreFFI.xcframework/macos-arm64/Headers/`) are likewise generated, not hand-maintained.

**Ceiling, flagged:** the xcframework currently has only a `macos-arm64` slice (this dev machine's architecture) — a universal/Intel build needs step 1 run on (or cross-compiled for) x86_64 and merged in via a second `-library`/`-headers` pair on the same `-create-xcframework` invocation. Tracked as part of ADR-029's five-target-triple pipeline, not a regression introduced here.

## Structure

- `IrisCoreFFI.xcframework/` — checked-in binary artifact: the release `iris-core` dylib (relocatable, ad-hoc signed) plus its UniFFI-generated C header, wrapped as a `.binaryTarget` in `Package.swift`. Replaces the old dev-loop hack of linking directly against a freshly-`cargo build`-ed debug dylib by absolute path.
- `Sources/IrisCore/` — the generated Swift bindings (`FfiEngine`, `FfiNode`, `FfiRecurrence`, etc.) — the actual Swift-friendly API.
- `Sources/Iris/` — the real app: `IrisApp.swift` (entry point), `Theme.swift` (mirrors `design/tokens.md` exactly — colors/spacing/type as Swift values), `Screens/` (one file per screen from `design/navigation.md`).

## Design reference

Every screen here should match `design/DESIGN_RULES.md` + `design/tokens.md` + `design/components.md` + `design/navigation.md` + `design/screen-flow.md`, and the corresponding `design/canvas/*.dc.html` / `design/screens/*.png` for that screen. See `design/screen-flow.md` §5 for which screens are this builder's ("Claude Code, Core Workflows") vs. Codex's ("App Surfaces & Domains").
