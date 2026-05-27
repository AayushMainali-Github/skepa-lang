# Skepa Language

Skepa is a statically typed compiled language implemented in Rust.

Tools:
- `skepac`: check, run, and build native artifacts

## Install

### 1) Prebuilt binaries (no Rust)

Download from GitHub Releases:
- Windows: `skepa-windows-x64.zip`
- Linux: `skepa-linux-x64.tar.gz`
- macOS: `skepa-macos-x64.tar.gz`

Extract and add binaries to `PATH`.

### 2) Install from GitHub with Cargo

```bash
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skepac
```

### 3) Build/install locally

Windows (PowerShell):
```powershell
./scripts/install.ps1
```

Linux/macOS (bash):
```bash
./scripts/install.sh
```

Manual:
```bash
cargo install --path skepac
```

## Run

```bash
skepac check app.sk
skepac run app.sk
skepac build-native app.sk app.exe
skepac build-obj app.sk app.obj
skepac build-llvm-ir app.sk app.ll
```

`build-obj` and `build-native` keep local cache metadata, compiled object artifacts, and reusable linked native outputs under `.skepac-cache/`, so unchanged builds can skip recompilation, relink from a cached object, or restore a missing executable from the cached linked artifact.

Set `SKEPAC_TIMINGS=1` to print per-phase timing lines for `build-obj` and `build-native` when you want to inspect cache hits, codegen cost, and link cost locally.

Set `SKEPA_CODEGEN_TIMINGS=1` to print lower-level backend stage timings from `skeplib` itself, including LLVM IR emit, `llvm-as`, `clang` object codegen, and native link phases.

## Project Layout

Skepa projects are file-system based. The CLI takes an explicit entry file, usually `main.sk`.

Recommended small-project layout:

```text
my_app/
  main.sk
  lib.sk
  utils/
    math.sk
```

Example:

```sk
// utils/math.sk
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
```

```sk
// main.sk
from utils.math import add;

fn main() -> Int {
  return add(20, 22);
}
```

Run it with:

```bash
skepac check main.sk
skepac run main.sk
```

## Examples

Shipped sample apps live under `examples/`.

- `examples/hello/main.sk`
  - minimal single-file program
- `examples/inventory/main.sk`
  - multi-file project with structs, `Vec`, imports, and user-facing output

Try them with:

```bash
skepac check examples/hello/main.sk
skepac run examples/inventory/main.sk
```

Folder namespaces map directly to import paths:

- `utils/math.sk` -> `utils.math`
- `string/case.sk` -> `string.case`
- `a/mod.sk` -> `a.mod`

## User Test Workflow

There is no `skepac test` command yet.

Current recommended workflow for user programs:

1. keep small executable checks in `.sk` files with a `main() -> Int`
2. return `0` for success and non-zero for failure
3. run them with `skepac run <entry.sk>`
4. use `skepac check <entry.sk>` in fast validation loops before native runs

For multi-file projects, point `skepac` at the entry file for the specific executable check you want to run.

For full language/module reference, see `DOCS.md`.

For the internal runtime and FFI ABI contract, see `RUNTIME.md`.

For language specification authority and compatibility rules, see `LANGUAGE_POLICY.md`.

For current support targets, release expectations, and production-hardening status, see `PRODUCTION.md`.

## Contributing

See [`.github/CONTRIBUTING.md`](./.github/CONTRIBUTING.md) for contribution workflow and commit message guidance.

See [`TESTING.md`](./TESTING.md) for testing expectations and validation commands.
