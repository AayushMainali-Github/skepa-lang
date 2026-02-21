# Skepa Language

Skepa is a statically typed compiled language implemented in Rust.

Tools:
- `skepac`: check/build/disassemble
- `skeparun`: run source or bytecode

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
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skeparun
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
cargo install --path skeparun
```

## Run

```bash
skepac check app.sk
skepac build app.sk app.skbc
skeparun run app.sk
skeparun run-bc app.skbc
skepac disasm app.sk
```

Runtime options:
- `skeparun run --trace app.sk` enables VM trace output.
- `SKEPA_MAX_CALL_DEPTH` can be set to control call depth and must be an integer `>= 1`.

## Quickstart (multi-file project)

```text
myapp/
  main.sk
  utils/
    math.sk
```

`utils/math.sk`:
```sk
fn add(a: Int, b: Int) -> Int { return a + b; }
export { add };
```

`main.sk`:
```sk
from utils.math import add;
fn main() -> Int { return add(20, 22); }
```

Run:
```bash
skeparun run myapp/main.sk
```

## Examples

- `examples/master.sk`
- `examples/master_modules.sk`
- `examples/modules_basic/`
- `examples/modules_folder/`
- `examples/modules_fn_struct/`

For full language/module reference, see `DOCS.md`.
