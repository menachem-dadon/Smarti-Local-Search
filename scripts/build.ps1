$ErrorActionPreference='Stop'
Set-Location -LiteralPath (Split-Path $PSScriptRoot -Parent)
npm --prefix desktop run build
if($LASTEXITCODE){throw 'Frontend build failed'}
cargo build --release -p smarti-local-search
if($LASTEXITCODE){throw 'Native release build failed'}
