# Testing Guide

This repo relies on layered tests. New features should add tests at the narrowest useful layer first, then add a cross-layer test if the behavior reaches runtime, codegen, or CLI.

## Repo Rule

No new feature should be merged without:
- at least one unit or regression test for the module that changed
- at least one cross-layer test if the behavior reaches runtime or CLI

Cross-layer tests include:
- IR interpreter vs native execution comparisons
- native codegen execution tests
- `skepac` CLI tests
- runtime dispatch tests in `skepart`

## Where Tests Go

### `skeplib/tests`

Use `skeplib/tests` for compiler and backend behavior.

- `lexer.rs`
  - tokenization, spans, recovery
- `parser.rs`, `parser_cases/*`, `parser_fixtures.rs`
  - syntax shape, parser recovery, fixture-driven parser coverage
- `resolver.rs`, `resolver_cases/*`, `resolver_fixtures.rs`
  - module graph resolution, import/export rules, project filesystem behavior
- `sema.rs`, `sema_cases/*`, `sema_fixtures.rs`
  - typing, semantic rules, builtin signatures, invalid programs
- `sema_project.rs`, `sema_project_fixtures.rs`
  - cross-module semantic behavior
- `ir.rs`
  - IR lowering, verification, interpretation, optimization, differential checks
- `codegen.rs`
  - LLVM IR emission, object generation, native executable generation
- `native_runtime.rs`
  - native executable correctness for single-file programs
- `native_project.rs`, `native_project_fixtures.rs`
  - native executable correctness for multi-file projects
- `diagnostic.rs`, `ast.rs`, `smoke.rs`
  - internal data structures and broad smoke coverage

### `skepart/tests`

Use `skepart/tests` for runtime library behavior.

- value/container semantics
- builtin dispatch
- host integration
- runtime function registry / trampoline behavior
- runtime error behavior

Runtime-only semantics should be tested here instead of through `skeplib` when possible.

### `skepac/tests`

Use `skepac/tests` for user-facing CLI behavior.

- command success/failure
- exit codes
- artifact creation
- stderr/stdout behavior
- toolchain failure messaging
- native run/build flows

### `skepabench`

Benchmark code should have correctness and harness tests for:
- baseline parsing/writing
- workload registration
- compare output shape
- required benchmark case presence

## Which Test Style To Use

### Inline Source Tests

Use inline source strings when:
- the case is small
- the test is focused on one rule
- the source is easier to understand directly in the test

Good for:
- sema regressions
- IR lowering edge cases
- codegen smoke tests

### Fixture Tests

Use fixtures when:
- the source is large enough to hurt readability inline
- you want a reusable valid/invalid corpus
- project directory shape matters

Good for:
- parser valid/invalid examples
- resolver graphs
- sema project tests
- multi-file native project tests

### Temp Project Tests

Create temporary files/directories when:
- the behavior depends on real filesystem layout
- the test needs generated artifacts
- the CLI or native backend should be exercised end-to-end

Good for:
- `skepac` tests
- native executable build/run tests
- project codegen tests

## Expected Cross-Layer Coverage

If a change affects one of these areas, add a cross-layer test:

- builtin behavior
  - sema + runtime/native
- runtime-managed values
  - IR interpreter + native executable
- codegen/runtime ABI
  - native executable test, not just LLVM text validation
- CLI-visible behavior
  - `skepac/tests`

## Preferred Test Flow

For new language/runtime features:
1. add a narrow unit/regression test
2. add a semantic acceptance/rejection test if relevant
3. add IR or codegen coverage if lowering/codegen changed
4. add a native or CLI test if user-visible runtime behavior changed

## Validation Commands

After every code change, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```
