# Testing Guide

This repo relies on layered, native-first tests. New features should add tests at the narrowest useful layer first, then add a cross-layer test if the behavior reaches IR interpretation, native codegen, runtime, or CLI.

## Repo Rule

No new feature should be merged without:
- at least one unit or regression test for the module that changed
- at least one cross-layer test if the behavior reaches runtime or CLI

Cross-layer tests include:
- IR interpreter vs native execution comparisons
- native codegen execution tests
- `skepac` CLI tests
- runtime dispatch tests in `skepart`

## Shared Helper Modules

Prefer shared helpers over ad hoc temp-file, fake-host, or process-running code.

### `skeplib/tests/common.rs`

Use this for compiler/backend tests.

- parsing and sema helpers
  - `parse_ok`
  - `parse_err`
  - `sema_ok`
  - `sema_err`
  - `assert_no_diags`
  - `assert_sema_success`
- fixture helpers
  - `fixtures_dir`
  - `sk_files_in`
- temp project helpers
  - `TempProject`
  - `make_temp_dir`
- IR helpers
  - `compile_ir_ok`
  - `compile_project_ir_ok`
  - `ir_run_ok`
  - `ir_run_err`
- native helpers
  - `native_run_structured`
  - `native_run_project_structured`
  - `native_run_exit_code_ok`
  - `native_run_printed_int_ok`
- LLVM/tool helpers
  - `llvm_tool_available`
  - `require_llvm_tool`
- differential helpers
  - `assert_native_matches_ir_value`
  - `assert_native_matches_ir_error_kind`
- runtime assertion helpers
  - `assert_runtime_error_kind`

### `skepart/tests/common.rs`

Use this for runtime tests.

- `RecordingHost`
  - deterministic fake host implementation
- `RecordingHostBuilder`
  - builder-style configuration for:
    - time
    - random values
    - cwd/platform
    - shell output/status
    - fake files / existence state

### `skepac/tests/common.rs`

Use this for CLI tests.

- temp path helpers
  - `make_temp_dir`
  - `write_temp_file`
- binary helpers
  - `skepac_bin`
- CLI assertions
  - `assert_cli_failure_class`
  - `assert_diag_code_and_message`

## Where Tests Go

### `skeplib/tests`

Use `skeplib/tests` for compiler and backend behavior.

- `frontend/lexer/main.rs`
  - tokenization, spans, recovery
- `frontend/parser/`
  - syntax shape, parser recovery, fixture-driven parser coverage
- `frontend/resolver/`
  - module graph resolution, import/export rules, project filesystem behavior
- `frontend/sema/`
  - typing, semantic rules, builtin signatures, invalid programs
- `frontend/sema_project/`
  - cross-module semantic behavior
- `frontend/ast/main.rs`, `frontend/diagnostic/main.rs`
  - internal data structures and diagnostic helpers
- `ir/lowering/main.rs`, `ir/verify/main.rs`, `ir/interp/main.rs`, `ir/diff/main.rs`
  - IR lowering, verification, interpretation, differential checks
- `ir/opt/`
  - optimization pass behavior, cross-pass stability, semantic preservation
- `ir/pretty/main.rs`
  - textual IR rendering coverage
- `native/codegen/main.rs`
  - LLVM IR emission, object generation, native executable generation
- `native/runtime/main.rs`
  - native executable correctness for single-file programs
- `native/project/`
  - native executable correctness for multi-file projects
- `misc/smoke/main.rs`
  - broad smoke coverage

### `skepart/tests`

Use `skepart/tests` for runtime library behavior.

- `string.rs`
- `array.rs`
- `vec.rs`
- `value.rs`
- `structs.rs`
- `builtins.rs`
- `host.rs`
- `function.rs`
- `ffi.rs`

Runtime-only semantics should be tested here instead of through `skeplib` when possible.

### `skepac/tests`

Use `skepac/tests` for user-facing CLI behavior.

- command success/failure
- exit codes
- artifact creation
- stderr/stdout behavior
- toolchain failure messaging
- native run/build flows
- install-layout/runtime-archive failures

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

Prefer `TempProject` in `skeplib/tests/common.rs` when testing compiler/runtime behavior directly.
Prefer `make_temp_dir` / `write_temp_file` in `skepac/tests/common.rs` for CLI black-box tests.

## Preferred Assertions

Prefer narrow, stable assertions over matching whole stderr blocks.

- diagnostics
  - use `assert_has_diag` / `assert_diag_code_and_message`
- runtime failures
  - assert error kind with `assert_runtime_error_kind`
- CLI failures
  - assert failure class with `assert_cli_failure_class`
- differential runtime behavior
  - use `assert_native_matches_ir_value`
  - use `assert_native_matches_ir_error_kind`

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
3. add IR interpreter coverage if lowering/runtime semantics changed
4. add codegen/native coverage if backend behavior changed
5. add a CLI test if the change is user-visible from `skepac`

## Native-First Differential Pattern

For runtime-managed values and backend-sensitive behavior, prefer this progression:

1. sema acceptance/rejection
2. IR lowering shape check if relevant
3. IR interpreter execution
4. native executable execution
5. IR vs native differential assertion where practical

Use IR interpreter tests for internal semantic validation.
Use native execution tests for end-to-end backend confidence.

## Validation Commands

After every code change, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```
