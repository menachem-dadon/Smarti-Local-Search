$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $taskRoot
& ./.venv/Scripts/python.exe scripts/verify_resources.py
if($LASTEXITCODE){throw 'Resource validation failed'}
& ./.venv/Scripts/python.exe scripts/source_manifest.py check
if($LASTEXITCODE){throw 'The release no longer matches its source files'}
$exe=Join-Path $taskRoot 'target/release/smarti-local-search.exe'
$config=Get-Content desktop/src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$installer=Get-Item -LiteralPath (Join-Path $taskRoot ("target/release/bundle/nsis/$($config.productName)_$($config.version)_x64-setup.exe")) -ErrorAction SilentlyContinue
if(!(Test-Path -LiteralPath $exe) -or !$installer){throw 'Release EXE or installer missing'}
New-Item -ItemType Directory -Force -Path 'artifacts/release' | Out-Null
$files=@($exe,$installer.FullName)
$checksums=$files | ForEach-Object { $hash=Get-FileHash -LiteralPath $_ -Algorithm SHA256; "$($hash.Hash.ToLower())  $([IO.Path]::GetFileName($_))" }
$checksums | Set-Content -LiteralPath 'artifacts/release/SHA256SUMS.txt' -Encoding ascii
Copy-Item -LiteralPath $exe,$installer.FullName -Destination 'artifacts/release' -Force
$commit=git rev-parse HEAD
$report=@{version=$config.version;commit=$commit;model_bytes=(Get-Item 'resources/models/embeddinggemma-2-740m.litertlm').Length;installer_bytes=$installer.Length;exe=$exe;installer=$installer.FullName;verified_resources=$true;installed_smoke_test=$false}
$installedReport='artifacts/release/installed-verification.json'
if(Test-Path -LiteralPath $installedReport){
    $installed=Get-Content -LiteralPath $installedReport -Raw | ConvertFrom-Json
    if($installed.version -eq $config.version -and $installed.exe_sha256 -eq (Get-FileHash -LiteralPath $exe).Hash){
        & ./.venv/Scripts/python.exe scripts/verify-installed-payload.py check artifacts/release/installed-payload.json
        if($LASTEXITCODE){throw 'Release differs from installed QA payload'}
        foreach($field in $installed.PSObject.Properties){$report[$field.Name]=$field.Value}
    }
}
$report | ConvertTo-Json | Set-Content -LiteralPath 'artifacts/release/verification.json' -Encoding utf8
Write-Output $report
