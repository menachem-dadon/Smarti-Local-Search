param([switch]$SkipTests)
$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $taskRoot
$python=Join-Path $taskRoot '.venv/Scripts/python.exe'
if(!(Test-Path -LiteralPath $python)){python -m venv .venv}
& $python -m pip install --timeout 90 -r inference-host/requirements.transitive.lock
if($LASTEXITCODE){throw 'Python dependencies failed'}
& $PSScriptRoot/fetch-model.ps1
& $PSScriptRoot/fetch-ffmpeg.ps1
& $PSScriptRoot/prepare-runtime.ps1
npm --prefix desktop ci --legacy-peer-deps
if($LASTEXITCODE){throw 'Frontend dependencies failed'}
node scripts/generate-icon.cjs
& $python scripts/generate_icon.py
if(!$SkipTests){& $PSScriptRoot/test.ps1}
& $python scripts/collect_notices.py
& $python -m PyInstaller --noconfirm --onedir --name smarti-local-search-inference --distpath artifacts/sidecar-dist --workpath artifacts/pyinstaller --specpath inference-host/packaging --collect-all litert_lm --collect-all pypdfium2 --collect-all numpy --collect-all PIL --collect-all openvino --hidden-import docx --hidden-import pptx --hidden-import openpyxl --hidden-import odf.opendocument --hidden-import psutil inference-host/src/host.py
if($LASTEXITCODE){throw 'Inference host packaging failed'}
$source=Join-Path $taskRoot 'artifacts/sidecar-dist/smarti-local-search-inference'
$destination=Join-Path $taskRoot 'resources/inference'
$destination=[IO.Path]::GetFullPath($destination)
if($destination -ne [IO.Path]::GetFullPath((Join-Path $taskRoot 'resources/inference'))){throw 'Unsafe bundled runtime directory'}
if(Test-Path -LiteralPath $destination){Remove-Item -LiteralPath $destination -Recurse -Force}
New-Item -ItemType Directory -Force -Path $destination | Out-Null
Get-ChildItem -LiteralPath $source | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse -Force}
& $python scripts/verify_resources.py
if($LASTEXITCODE){throw 'Bundled runtime verification failed'}
& $python scripts/source_manifest.py capture
if($LASTEXITCODE){throw 'Source manifest capture failed'}
npm --prefix desktop run tauri -- build --bundles nsis
if($LASTEXITCODE){throw 'Release build failed'}
& $PSScriptRoot/verify-release.ps1
& $PSScriptRoot/install-smoke.ps1
