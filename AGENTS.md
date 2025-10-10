# Repository Guidelines

## Project Structure & Modules
- `src/` — Rust WASM lib (`cdylib`) with entry in `src/lib.rs`.
- `memlayout/` — Rust crate for DWARF-driven memory visualization and typed reads.
- `tools/` — C++ utilities: `wasm_debugger`, `dwarf_dump`, `wat_tools` (uses WABT, Wasmtime C-API, LLVM).
- `CMakeLists.txt` — top-level CMake driving C++ tools; `Cargo.toml` files define Rust crates.
- `wabt/`, `third_party/` — vendored or external deps; `web/` reserved for UI assets.
- Build outputs: `target/` (Rust), `build/` (CMake, when used).

## Build, Test, and Development
- Rust (root crate → .wasm): `rustup target add wasm32-unknown-unknown && cargo build --target wasm32-unknown-unknown`.
- Rust (memlayout host crate): `cargo build -p memlayout` and `cargo test -p memlayout`.
- C++ tools: `cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build -j`.
- Run tools: binaries appear under `build/tools/<tool>/` (e.g., `build/tools/wasm_debugger/wasm_debugger`).
- Requirements: WABT (wasm2wat/wat2wasm), Wasmtime C-API, LLVM dev headers.

## Coding Style & Naming
- Rust: run `cargo fmt` and `cargo clippy --all-targets -- -D warnings`. Modules `snake_case`; types `CamelCase`; constants `SCREAMING_SNAKE_CASE`.
- C++: C++20; format with `clang-format` (LLVM style). Types `PascalCase`, functions `lowerCamelCase`, constants `UPPER_SNAKE_CASE`. Enable warnings (`-Wall -Wextra`) in new targets.

## Testing Guidelines
- Rust: unit tests inline; integration tests under `tests/`. Name files `*_test.rs` where practical. Run `cargo test` or `cargo test -p memlayout`.
- C++: prefer `ctest` via CMake. If adding a tool, place tests under `tools/<tool>/tests` and wire with `add_test`.
- Aim for meaningful coverage on parsing, DWARF handling, and memory reads.

## Commit & Pull Request Guidelines
- Commits: imperative subject (“Add wasm trace hook”), concise body explaining why, reference issues (`#123`). Conventional Commits (`feat:`, `fix:`, `chore:`) encouraged.
- PRs: include build/run instructions, linked issue(s), and screenshots/CLI output when relevant. Keep diffs focused; CI must pass.

## Security & Configuration Tips
- Install `wasm32-unknown-unknown` for Rust WASM builds. Ensure WABT and Wasmtime C-API are present. If using Emscripten workflows, source `emsdk_env.sh` before building UI targets.

## Agent-Specific Notes
- Keep changes minimal and scoped; don’t reformat unrelated files. Prefer small, reviewable patches and update docs when adding commands or targets.

