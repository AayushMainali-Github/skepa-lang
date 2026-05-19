# Runtime and ABI Contract

This document defines the internal runtime contract for `skepart`, native codegen, and the C ABI helpers exported by the runtime.

It is not a user-facing language tutorial. For source-language semantics, see [`DOCS.md`](./DOCS.md).

## Runtime Value Ownership

The exported `skp_rt_*` constructors return owned heap pointers.

- `skp_rt_value_from_*` returns an owned `*mut RtValue`
- `skp_rt_string_from_utf8` returns an owned `*mut RtString`
- container/object constructors such as `skp_rt_array_new`, `skp_rt_vec_new`, and `skp_rt_struct_new` return owned heap pointers

The caller that receives an owned pointer is responsible for eventually freeing it with the matching `skp_rt_*_free` helper.

Free helpers:

- `skp_rt_value_free`
- `skp_rt_string_free`
- `skp_rt_array_free`
- `skp_rt_vec_free`
- `skp_rt_map_free`
- `skp_rt_option_free`
- `skp_rt_result_free`
- `skp_rt_struct_free`

Rules:

- owned runtime pointers must be freed at most once
- null pointers are accepted by free helpers and are ignored
- raw-pointer-consuming exports are `unsafe extern "C" fn`; non-null is not enough, the pointer must also be valid for the expected runtime type

### Clone vs Consume

The runtime uses two distinct pointer helpers:

- `clone_value(ptr)` borrows and clones the pointed runtime value
- `take_value(ptr)` consumes ownership of the boxed runtime value and takes it out of the raw pointer

Use `take_value` only when the callee is transferring ownership of the boxed runtime object to the runtime wrapper.

## Linked Extern ABI Surface

The intended public FFI surface is linked `extern("...") fn ...;` declarations in Skepa source.

The runtime deliberately supports only a narrow borrowed ABI layer. The exact supported linked extern shapes are:

- `extern("lib") fn foo() -> Int`
- `extern("lib") fn foo() -> Bool`
- `extern("lib") fn foo() -> Void`
- `extern("lib") fn foo(Int) -> Int`
- `extern("lib") fn foo(Int) -> Bool`
- `extern("lib") fn foo(Int) -> Void`
- `extern("lib") fn foo(String) -> Int`
- `extern("lib") fn foo(String) -> Void`
- `extern("lib") fn foo(Int, Int) -> Int`
- `extern("lib") fn foo(Bytes, Int) -> Int`
- `extern("lib") fn foo(String, String) -> Int`
- `extern("lib") fn foo(String, Int) -> Int`
- `extern("lib") fn foo(Bytes) -> Int`

Internal runtime signature strings name the exact ABI shape used by lowering and verification:

- `->i64`
- `->_Bool`
- `->i32bool`
- `system:->BOOL`
- `i64->i64`
- `i64->_Bool`
- `i64->i32bool`
- `system:i64->BOOL`
- `i64->void`
- `cstr->usize`
- `system:cstr->i32`
- `cstr->void`
- `system:cstr->void`
- `cstr,cstr->i32`
- `system:cstr,cstr->i32`
- `cstr,usize->usize`
- `bytes->usize`
- `i64,i64->i64`
- `bytes,usize->usize`

These strings are runtime implementation details. User code should not call low-level `ffi.call...` helpers directly.

### Borrowing Rules

- `String` arguments are passed as temporary borrowed NUL-terminated pointers
- `Bytes` arguments are passed as temporary borrowed `(ptr, len)` views
- borrowed string and byte views are valid only for the duration of the foreign call
- foreign code must not retain or mutate those borrowed views after the call returns
- no raw pointer ownership transfer API is currently part of the contract
- callbacks are not part of the current contract

Wrong ABI/signature use at the foreign boundary is undefined behavior.

## Handle and Resource Lifetime

Runtime handles represent host-managed resources such as:

- sockets
- listeners
- tasks
- channels
- foreign libraries
- foreign symbols

Rules:

- handles are affine resources: a live handle may be explicitly closed or consumed by an operation such as `task.join`
- aliasing the same logical handle is allowed in source-level values, but close/consume is global for the underlying resource
- using a closed handle is a runtime error
- using the wrong handle kind is a runtime error
- double-close is a runtime error
- `task.join` consumes the task handle
- `ffi.closeSymbol` and `ffi.closeLibrary` reclaim host handle state; symbol/library cache entries are weak and may be pruned after the last strong owner drops

## Panic and Error Mapping

The runtime distinguishes ordinary typed failure from runtime failure.

Typed failure remains in language values:

- `Option`
- `Result`

Runtime failure is carried as `RtError` with `RtErrorKind`.

### FFI Wrapper Panic Mapping

`ffi_try(...)` is the standard wrapper for exported runtime C ABI functions.

Rules:

- it clears the thread-local last error before executing the body
- it catches Rust panics with `catch_unwind`
- a caught panic becomes `RtErrorKind::InvalidArgument`
- string panic payloads are preserved when possible; otherwise the message becomes `runtime ffi panic`

Exports such as `skp_rt_call_builtin` and `skp_rt_call_function` return a unit boxed value on failure and record the last error.

### Last Error and Abort

- `skp_rt_last_error_kind()` exposes the current thread-local coarse error kind
- `skp_rt_abort_if_error()` prints the recorded error to stderr and exits with code `101`
- native LLVM codegen calls `skp_rt_abort_if_error()` after fallible runtime helper boundaries
- the IR interpreter converts `RtErrorKind` into interpreter errors instead of aborting the host process

### Task Panic Mapping

If a spawned native task panics, `task.join` maps that failure to:

- `RtErrorKind::InvalidArgument`
- message: `spawned task panicked`

This keeps panic behavior deterministic at the language boundary instead of exposing Rust panic internals directly.

## Stabilization Invariants

The following invariants should remain true:

- no safe exported Rust function may dereference caller-provided raw pointers
- every supported linked extern source signature must lower to one exact runtime ABI shape
- unsupported ABI shapes must be rejected during semantic checking or IR verification
- runtime handle misuse must fail as runtime error, not silent reuse
- exported FFI wrappers must not unwind across the C ABI boundary
