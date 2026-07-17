param(
  [switch]$Force
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo is required. Install Rust from https://rustup.rs/"
  exit 1
}

$root = Split-Path -Parent $PSScriptRoot

$cargoArgs = @("install")
if ($Force) {
  $cargoArgs += "--force"
}

Write-Host "Installing skepac..."
& cargo @cargoArgs --path (Join-Path $root "skepac")

Write-Host "Done. Ensure `$env:USERPROFILE\.cargo\bin` is on PATH."
