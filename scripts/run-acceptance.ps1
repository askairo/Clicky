param(
  [string]$Filter = "acceptance_"
)

$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
  cargo test --manifest-path src-tauri/Cargo.toml $Filter -- --test-threads=1
}
finally {
  Pop-Location
}
