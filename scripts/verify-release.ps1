$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $taskRoot
& ./.venv/Scripts/python.exe scripts/verify_resources.py
if($LASTEXITCODE){throw 'Resource validation failed'}
& ./.venv/Scripts/python.exe scripts/source_manifest.py check
if($LASTEXITCODE){throw 'The release no longer matches its source files'}
$exe=Join-Path $taskRoot 'target/release/smarti-local-search.exe'
$installer=Get-ChildItem -LiteralPath 'target/release/bundle/nsis' -Filter '*.exe' | Select-Object -First 1
if(!(Test-Path -LiteralPath $exe) -or !$installer){throw 'Release EXE or installer missing'}
New-Item -ItemType Directory -Force -Path 'artifacts/release' | Out-Null
$files=@($exe,$installer.FullName)
$checksums=$files | ForEach-Object { $hash=Get-FileHash -LiteralPath $_ -Algorithm SHA256; "$($hash.Hash.ToLower())  $([IO.Path]::GetFileName($_))" }
$checksums | Set-Content -LiteralPath 'artifacts/release/SHA256SUMS.txt' -Encoding ascii
Copy-Item -LiteralPath $exe,$installer.FullName -Destination 'artifacts/release' -Force
$report=@{version='0.1.0';model_bytes=(Get-Item 'resources/models/embeddinggemma-2-740m.litertlm').Length;installer_bytes=$installer.Length;exe=$exe;installer=$installer.FullName;verified_resources=$true;installed_smoke_test=$false}
$report | ConvertTo-Json | Set-Content -LiteralPath 'artifacts/release/verification.json' -Encoding utf8
Write-Output $report
