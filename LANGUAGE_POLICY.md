# Language Specification and Compatibility Policy

This document defines how Skepa language semantics are specified and how future changes must be evaluated.

It exists to keep language behavior from drifting according to implementation accident, test inertia, or local intuition.

## 1. Spec Authority

The authoritative documents are:

- [`DOCS.md`](./DOCS.md): source-language syntax, typing, module rules, builtin surface, and user-visible semantics
- [`RUNTIME.md`](./RUNTIME.md): internal runtime, ABI, handle, and FFI contract

The implementation in `skeplib`, `skepart`, and `skepac` is expected to follow those documents.

If code and docs disagree:

- for source-language behavior, `DOCS.md` wins
- for runtime/ABI behavior, `RUNTIME.md` wins
- the implementation is considered buggy or incomplete until it is brought back into agreement, unless the docs are intentionally changed in the same change set

Tests are enforcement tools, not the primary specification.

## 2. Stability Tiers

Skepa language surface is split into three tiers.

### Stable

Stable surface includes:

- core language syntax and typing described in `DOCS.md`
- module/import/export rules described in `DOCS.md`
- stable builtin packages explicitly listed as stable in `DOCS.md`

Stable surface should not change incompatibly without an explicit breaking-change decision.

### Experimental

Experimental surface includes language or builtin areas explicitly marked experimental in `DOCS.md`.

Current example:

- `task`

Experimental surface may change more freely while semantics are still being settled, but the docs must still be updated whenever the behavior changes.

### Narrow Special-Purpose

Some surfaces are supported but intentionally constrained to a smaller contract than the rest of the language.

Current example:

- `ffi`

These surfaces should be extended conservatively. New capability should be added only when the exact contract can be documented precisely.

## 3. Compatibility Rules

### Source-Compatible Changes

The following are source-compatible when they do not change existing program meaning:

- adding new valid programs
- improving diagnostics without changing accepted/rejected semantics
- clarifying docs without changing behavior
- tightening internal implementation details that are outside the documented contract
- adding new stable builtin functionality without altering existing signatures or meanings

### Breaking Changes

A change is breaking if it changes the meaning, validity, or observable behavior of a program that was valid under the documented stable contract.

Examples:

- changing type-checking rules for stable language constructs
- changing import/export resolution behavior
- changing operator precedence or associativity rules
- changing the return type or semantics of a stable builtin
- changing runtime error vs `Option`/`Result` behavior for a stable operation

Breaking changes require:

1. an explicit docs update
2. a compatibility note or migration note
3. commit marking consistent with repository contribution rules

## 4. Runtime Error vs Typed Failure Rules

Changes must preserve the documented boundary between:

- typed failure represented as `Option` or `Result`
- runtime failure represented as runtime errors

A change that moves an operation from one category to the other is a semantic change and must be treated as such.

## 5. Spec Change Process

For any semantic change to stable or experimental surface:

1. update the relevant spec document first or in the same change
2. update tests to enforce the documented behavior
3. update user-facing docs if the workflow or visible surface changed

No semantic change should land based only on “that is how the implementation currently behaves.”

## 6. Review Rule

When evaluating a new language/runtime change, reviewers should ask:

1. what document defines the current intended behavior?
2. is the change stable-surface, experimental-surface, or narrow-special-purpose?
3. is the change compatible or breaking?
4. which tests prove the documented behavior?

If those questions cannot be answered, the change is not ready to merge.
