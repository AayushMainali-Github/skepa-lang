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

## Operators

Skepa supports:
- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- logical: `!`, `&&`, `||`
- bitwise integer operators: `~`, `&`, `|`, `^`, `<<`, `>>`
- user-defined infix operators in backticks, for example ``a `xoxo` b``

Current bitwise rules:
- bitwise operators are `Int`-only
- shifts require an `Int` right-hand side
- bitwise assignment operators like `&=` and `<<=` are not implemented yet

Current user-defined operator rules:
- declare with `opr name(lhs: T1, rhs: T2) -> R precedence N { ... }`
- binary only
- backtick infix use only
- custom operators must currently be declared before use in the same module
- cross-module imported custom operator usage is not implemented yet

## Migration

Old commands were removed:
- old runtime-runner commands were replaced by `skepac run`
- old backend-specific build/disassembly flows were removed

Use these native-first commands instead:
- `skepac check app.sk`
- `skepac run app.sk`
- `skepac build-native app.sk app.exe`
- `skepac build-llvm-ir app.sk app.ll`

## Examples


For full language/module reference, see `DOCS.md`.

## Contributing

See [`.github/CONTRIBUTING.md`](./.github/CONTRIBUTING.md) for contribution workflow and commit message guidance.

See [`TESTING.md`](./TESTING.md) for testing expectations and validation commands.
