# AGENTS Guide

This file defines the working rules for making changes in this repository.

## Architecture

- `skeplib`
  - frontend, sema, IR, optimization, LLVM codegen
- `skepart`
  - native runtime library
- `skepac`
  - user-facing CLI
- `skepabench`
  - benchmark harness

The old bytecode/VM backend is removed. Do not reintroduce bytecode, VM, `.skbc`, or VM-oriented execution paths.

## Change Expectations

For any non-trivial change:
- add or update a narrow regression/unit test
- add a cross-layer test if behavior affects:
  - runtime
  - native codegen
  - CLI

Prefer extending existing shared test helpers instead of creating new ad hoc temp-project, fake-host, or process-running utilities.

See [TESTING.md](C:/Users/ACER/OneDrive/Desktop/coding/maintained/skepa-lang/TESTING.md) for test placement and helper usage.

## Validation

After code changes, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

If a change is codegen- or CLI-heavy, ensure native build/run paths are exercised by tests, not only IR or unit coverage.

## Editing Rules

- Preserve the native-first architecture.
- Prefer shared helpers in:
  - `skeplib/tests/common.rs`
  - `skepart/tests/common.rs`
  - `skepac/tests/common.rs`
- Keep runtime semantics centralized in `skepart` rather than duplicating them in tests or codegen.
- Do not silently weaken diagnostics or test assertions just to make tests pass.

## Commit Messages

Use short conventional commit messages with a scoped area when possible.

Examples:
- `feat(codegen): add native builtin execution coverage`
- `fix(runtime): correct host-backed fs behavior`
- `test(ir): expand differential runtime checks`
- `docs(testing): document shared test helpers`

## Docs

When changing user-visible commands, testing patterns, or architecture boundaries, update the relevant docs:
- `README.md`
- `TESTING.md`
- `AGENTS.md`
- `CONTRIBUTING.md` if contributor workflow changes
