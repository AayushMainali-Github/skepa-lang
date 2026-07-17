param(
  [switch]$Force
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo is required. Install Rust from https://rustup.rs/"
  exit 1
}

$root = Split-Path -Parent $PSScriptRoot
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
$binDir = Join-Path $cargoHome "bin"

$cargoArgs = @("install")
if ($Force) {
  $cargoArgs += "--force"
}

Write-Host "Installing skepac..."
& cargo @cargoArgs --path (Join-Path $root "skepac")

Write-Host "Building native runtime library..."
& cargo build --release -p skepart --manifest-path (Join-Path $root "Cargo.toml")

New-Item -ItemType Directory -Force $binDir | Out-Null
$runtimeArtifacts = @(
  "skepart.dll",
  "skepart.dll.lib",
  "libskepart.dll.a",
  "libskepart.a",
  "skepart.lib"
)
$copied = $false
foreach ($name in $runtimeArtifacts) {
  $src = Join-Path $root "target\release\$name"
  if (Test-Path $src) {
    Copy-Item $src (Join-Path $binDir $name) -Force
    $copied = $true
  }
}
if (-not $copied) {
  throw "No skepart runtime artifacts found under target\release after release build"
}

Write-Host "Done. Ensure $binDir is on PATH."
Write-Host "build-native will find skepart beside skepac (or via SKEPA_RUNTIME_DIR)."
