# Contributing to Skepa

Thanks for your interest in contributing to Skepa.

This repository follows a conventional commit style for commit messages.

## Before You Contribute

Please open an issue before starting work on a bug fix or feature. This gives us a chance to discuss the change, confirm the approach, and avoid duplicated effort.

Pull requests should be tied to an approved issue. If you open a PR without a corresponding issue, you may be asked to open one before the PR can be reviewed.

Create a separate branch for your work instead of committing directly to `main`. A `feature/...` or `fix/...` branch keeps changes isolated and easier to review.

## Commit Message Format

Use this shape:

```text
<type>(<optional scope>): <description>
```

## Rules

- Use the imperative, present tense
- Start the description with a lowercase letter
- Do not end the description with a period

## Types

- `feat`
- `fix`
- `refactor`
- `perf`
- `style`
- `test`
- `docs`
- `build`
- `ops`
- `chore`

## Scopes

Scopes are optional. Use one when it helps identify the area of the change.

## Breaking Changes

If a commit introduces a breaking change, add `!` after the type or scope.

## Testing

New features should include tests at the narrowest useful layer first. If the change affects runtime, codegen, or CLI behavior, add cross-layer coverage as well.

No new feature should be merged without:
- at least one unit or regression test for the module that changed
- at least one cross-layer test if the behavior reaches runtime or CLI

Common test locations:
- `skeplib/tests` for compiler and backend behavior
- `skepart/tests` for runtime library behavior
- `skepac/tests` for user-facing CLI behavior

Preferred test flow for language or runtime changes:
1. add a narrow unit or regression test
2. add semantic acceptance or rejection coverage if relevant
3. add IR or codegen coverage if lowering or codegen changed
4. add a native or CLI test if user-visible runtime behavior changed

See [`TESTING.md`](../TESTING.md) for the full testing guide and where different test types belong.

Before opening a PR, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

## References

When useful, reference issues in the footer.

Reference:

- qoomon's conventional commit guide: <https://gist.github.com/qoomon/5dfcdf8eec66a051ecd85625518cfd13>
