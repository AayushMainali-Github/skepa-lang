# Production Hardening

This document defines the current production-readiness envelope for Skepa and the minimum process for shipping releases.

## Support Envelope

Normal CI coverage:
- Linux: `ubuntu-latest`
- macOS: `macos-latest`
- Windows: `windows-latest`

Release artifacts are built for:
- Linux x64
- macOS x64
- Windows x64

Toolchain assumptions:
- stable Rust toolchain
- LLVM/Clang toolchain available on the host
- native build and smoke-test coverage exercised through `skepac`

What is covered continuously:
- formatting
- clippy
- full workspace tests
- native codegen smoke tests
- CLI native build/run smoke tests

What is not yet guaranteed:
- non-x64 release artifacts
- long-term backwards compatibility for experimental surfaces
- sanitizer-clean or leak-check-clean builds on every CI run
- performance stability without separate benchmark review

## Release Process

Release tags use the form `v<version>`.

Before cutting a release:
1. ensure normal CI is green on all supported platforms
2. run:
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --workspace`
3. confirm `DOCS.md`, `RUNTIME.md`, and `LANGUAGE_POLICY.md` match the shipped behavior
4. confirm `README.md` install and run commands still match the CLI
5. review changes for breaking behavior and document migration notes when required

Release automation:
- `.github/workflows/release.yml` builds tagged releases on Linux, macOS, and Windows
- each release job builds `skepac` and `skepart`
- each release job runs a native smoke test before packaging
- packaged artifacts are uploaded to the GitHub release

## Reproducibility Expectations

Current baseline:
- the repo is locked by `Cargo.lock`
- release packaging uses the repo-root workflow definitions
- install scripts point at the workspace `skepac` crate directly

Maintainers should treat these as release regressions:
- a tagged build cannot be reproduced from the tag with the documented toolchain
- packaged binaries differ in CLI behavior from CI smoke-tested binaries
- a release changes stable language/runtime behavior without matching spec updates

## Hardening Backlog

Phase 7 is not fully closed until the repo also has:
- fuzzing infrastructure
- broader sample apps outside the test corpus
- stronger resource-regression tooling
