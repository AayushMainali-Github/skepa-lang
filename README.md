# Skepa Language

Skepa is a compiled language implemented in Rust.

Tooling:
- `skepac`: check, build bytecode, disassemble
- `skeparun`: run source and bytecode

## Install

Prerequisite for Cargo-based install: Rust + Cargo (`cargo --version`).

### 1) Local repo installer scripts

```powershell
./scripts/install.ps1
```

```bash
./scripts/install.sh
```

### 2) Local repo manual Cargo install

```bash
cargo install --path skepac
cargo install --path skeparun
```

### 3) GitHub install without cloning

```bash
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skepac
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skeparun
```

### 4) Prebuilt binaries from GitHub Releases

Download latest assets and add binaries to `PATH`:
- `skepa-windows-x64.zip`
- `skepa-linux-x64.tar.gz`
- `skepa-macos-x64.tar.gz`

## Run

Check:

```bash
skepac check app.sk
```

Build bytecode:

```bash
skepac build app.sk app.skbc
```

Run source:

```bash
skeparun run app.sk
```

Run bytecode:

```bash
skeparun run-bc app.skbc
```

Disassemble:

```bash
skepac disasm app.sk
skepac disasm app.skbc
```

Bundled example:

```bash
skeparun run examples/master.sk
```

## Builtin Packages

```sk
import io;
import str;
import arr;
```

- `io`: print/input/format builtins
- `str`: `len`, `contains`, `startsWith`, `endsWith`, `trim`, `toLower`, `toUpper`, `indexOf`, `lastIndexOf`, `slice`, `replace`, `repeat`, `isEmpty`
- `arr`: `len`, `isEmpty`, `contains`, `indexOf`, `count`, `first`, `last`, `reverse`, `sum`, `join`

Runtime notes:
- `arr.first` / `arr.last` on empty arrays -> `E-VM-INDEX-OOB`
- `str.repeat` with negative count -> `E-VM-INDEX-OOB`
- `str.repeat` output larger than 1,000,000 bytes -> `E-VM-INDEX-OOB`


For full language and runtime reference, see `DOCS.md`.
