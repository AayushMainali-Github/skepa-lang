# Skepa Language

Skepa is a small compiled language implemented in Rust with:

- `skepac`: checker/compiler/disassembler
- `skeparun`: source and bytecode runner

## Install (All Available Ways)

Prerequisite: Rust + Cargo (`cargo --version`)

### 1) From Local Repo (clone + installer scripts)

```powershell
./scripts/install.ps1
```

or

```bash
./scripts/install.sh
```

### 2) From Local Repo (manual Cargo install)

```bash
cargo install --path skepac
cargo install --path skeparun
```

### 3) No Clone via GitHub (Cargo git install)

```bash
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skepac
cargo install --git https://github.com/AayushMainali-Github/skepa-lang skeparun
```

### 4) No Rust/Cargo: prebuilt binaries from GitHub Releases

- Download the latest release assets:
  - `skepa-windows-x64.zip`
  - `skepa-linux-x64.tar.gz`
  - `skepa-macos-x64.tar.gz`
- Extract and add binaries to your `PATH`.
- Run `skepac` and `skeparun` directly.

## Automatic Prebuilt Binary Releases

- Workflow: `.github/workflows/release.yml`
- Trigger: push a version tag like `v0.1.3`
- It builds `skepac` + `skeparun` for Windows/Linux/macOS and uploads assets to the GitHub Release automatically.

Create a release tag:

```bash
git tag v0.1.3
git push origin v0.1.3
```

## Run

Check source:

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

For full language and runtime reference, see `DOCS.md`.
