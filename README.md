# Skepa Language

Skepa is a compiled language implemented in Rust.

Tooling:
- `skepac`: check, build bytecode, disassemble
- `skeparun`: run source and bytecode

## Install

### Option 1: Download prebuilt binaries (no Rust required)

From GitHub Releases, download:
- Windows: `skepa-windows-x64.zip`
- Linux: `skepa-linux-x64.tar.gz`
- macOS: `skepa-macos-x64.tar.gz`

Extract and add the binaries to your `PATH`.

### Option 2: Install from GitHub with Cargo

Prerequisite: Rust + Cargo (`cargo --version`).

```bash
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skepac
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skeparun
```

### Option 3: Build/install locally from source

Clone the repo first, then:

Windows (PowerShell):

```powershell
./scripts/install.ps1
```

Linux/macOS (bash):

```bash
./scripts/install.sh
```

Manual local install (all OSes):

```bash
cargo install --path skepac
cargo install --path skeparun
```

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

For full language and runtime reference, see `DOCS.md`.
