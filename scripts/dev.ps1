$ErrorActionPreference='Stop'
Set-Location -LiteralPath (Split-Path $PSScriptRoot -Parent)
if(!(Test-Path .venv/Scripts/python.exe)){python -m venv .venv; & ./.venv/Scripts/python.exe -m pip install -r inference-host/requirements.transitive.lock}
npm --prefix desktop ci --legacy-peer-deps
& $PSScriptRoot/fetch-model.ps1
& $PSScriptRoot/fetch-ffmpeg.ps1
npm --prefix desktop run tauri -- dev
