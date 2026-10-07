$ErrorActionPreference='Stop'
Set-Location -LiteralPath (Split-Path $PSScriptRoot -Parent)
cargo run --release -p smarti-search-core --example benchmark -- artifacts/performance
if($LASTEXITCODE){throw '100k ANN/FTS benchmark failed'}
