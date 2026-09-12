# Iris — macOS (SwiftUI)

Native macOS shell, per ADR-031 (no webview). Consumes `iris-core` directly through the real UniFFI `FfiEngine` surface — no backend logic lives here, only UI and the SwiftUI ↔ `FfiEngine` wiring.

## Build & run

1. Build the Rust side first (from the repo root):
   ```bash
   cd iris-core && cargo build
   ```
2. Then build/run the app:
   ```bash
   cd apps/macos && swift build
   swift run Iris
   ```

`Package.swift` links against `<repo-root>/target/debug/libiris_core.dylib` — resolved relative to `Package.swift`'s own location, so this works from any checkout or worktree. **You must `cargo build` the Rust side after any `iris-core` change** — `swift build` doesn't know to rebuild the Rust dependency for you.

## Regenerating the Swift bindings

Whenever `iris-core/src/ffi.rs`'s exported surface changes (a new `FfiEngine` method, a new DTO, etc.), regenerate:

```bash
cd iris-core
cargo build --lib
cargo run --bin uniffi-bindgen -- generate --library ../target/debug/libiris_core.dylib --language swift --out-dir /tmp/iris-swift-gen
cp /tmp/iris-swift-gen/iris_core.swift ../apps/macos/Sources/IrisCore/
cp /tmp/iris-swift-gen/iris_coreFFI.h ../apps/macos/Sources/iris_coreFFI/
```

`Sources/IrisCore/iris_core.swift` and `Sources/iris_coreFFI/iris_coreFFI.h` are **generated files — never hand-edit them.** `Sources/iris_coreFFI/module.modulemap` is hand-written (points at the generated header) and doesn't need regenerating.

## Structure

- `Sources/iris_coreFFI/` — a `.systemLibrary` target wrapping the generated C header, so Swift can see the raw FFI functions.
- `Sources/IrisCore/` — the generated Swift bindings (`FfiEngine`, `FfiNode`, `FfiRecurrence`, etc.) — the actual Swift-friendly API.
- `Sources/Iris/` — the real app: `IrisApp.swift` (entry point), `Theme.swift` (mirrors `design/tokens.md` exactly — colors/spacing/type as Swift values), `Screens/` (one file per screen from `design/navigation.md`).

## Design reference

Every screen here should match `design/DESIGN_RULES.md` + `design/tokens.md` + `design/components.md` + `design/navigation.md` + `design/screen-flow.md`, and the corresponding `design/canvas/*.dc.html` / `design/screens/*.png` for that screen. See `design/screen-flow.md` §5 for which screens are this builder's ("Claude Code, Core Workflows") vs. Codex's ("App Surfaces & Domains").

**Known simplification (ponytail):** `Package.swift` links against a debug dylib by absolute-relative path, not a distributable `.xcframework`. Fine for local dev; must change before this ships as an actual `.app` — see the comment in `Package.swift`.
