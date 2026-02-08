param(
  [switch]$Force
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo is required. Install Rust from https://rustup.rs/"
  exit 1
}

$root = Split-Path -Parent $PSScriptRoot

$args = @("install")
if ($Force) {
  $args += "--force"
}

Write-Host "Installing skepac..."
& cargo @args --path (Join-Path $root "skepac")

Write-Host "Installing skeparun..."
& cargo @args --path (Join-Path $root "skeparun")

Write-Host "Done. Ensure `%USERPROFILE%\.cargo\bin` is on PATH."
