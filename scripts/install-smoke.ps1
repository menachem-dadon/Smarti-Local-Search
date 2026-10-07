param([string]$Installer,[switch]$KeepInstalled,[string]$TestName)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
Set-Location -LiteralPath $taskRoot
if(!$Installer){$Installer=(Get-ChildItem target/release/bundle/nsis -Filter '*.exe'|Select-Object -First 1).FullName}
if(!$TestName){$TestName='installed-smoke-'+(Get-Date -Format 'yyyyMMdd-HHmmss')}
if($TestName -notmatch '^[a-z0-9-]+$'){throw 'TestName must contain only lowercase letters, digits and hyphens'}
$testBase=[IO.Path]::GetFullPath((Join-Path $taskRoot ('artifacts/'+$TestName)))
if(!$testBase.StartsWith($taskRoot+'\')){throw 'Unsafe test location'}
$installPath=Join-Path $testBase 'application'
if(Test-Path -LiteralPath $installPath){throw 'Smoke installation directory already exists; inspect or choose a new test location before retrying'}
New-Item -ItemType Directory -Path $testBase -Force|Out-Null
# NSIS /D must be the last argument. The path is trusted and constructed above.
$setup=Start-Process -FilePath $Installer -ArgumentList "/S /D=$installPath" -WindowStyle Hidden -Wait -PassThru
if($setup.ExitCode -ne 0){throw "Installer failed: $($setup.ExitCode)"}
$app=Join-Path $installPath 'smarti-local-search.exe'
if(!(Test-Path -LiteralPath $app)){throw 'Installed executable missing'}
$previousData=$env:SMARTI_SEARCH_DATA_DIR
try {
    $env:SMARTI_SEARCH_DATA_DIR=Join-Path $testBase 'private-index'
    $report=Join-Path $testBase 'native-report.json';$corpus=Join-Path $taskRoot 'tests/fixtures/corpus'
    $arguments=@(('"--smoke-test='+$report+'"'),('"--smoke-root='+$corpus+'"'))
    $process=Start-Process -FilePath $app -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if(!$process.WaitForExit(360000)){$process.Kill();throw 'Installed app smoke test timed out'}
    $result=Get-Content -LiteralPath $report -Raw|ConvertFrom-Json
    if(!$result.passed -or $process.ExitCode -ne 0){throw "Installed app validation failed: $($result.error)"}
    $resources=Join-Path $installPath 'resources'
    & ./.venv/Scripts/python.exe tests/inference_smoke.py --host (Join-Path $resources 'inference/smarti-local-search-inference.exe') --resources $resources --cache (Join-Path $testBase 'inference-fallback') --accelerator npu
    if($LASTEXITCODE){throw 'Installed inference/fallback validation failed'}
    $verification=Get-Content artifacts/release/verification.json -Raw|ConvertFrom-Json
    if(!$KeepInstalled){
        $uninstaller=Get-ChildItem -LiteralPath $installPath -Filter '*uninstall*.exe'|Select-Object -First 1
        if(!$uninstaller){throw 'Uninstaller missing'}
        $uninstall=Start-Process -FilePath $uninstaller.FullName -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
        if($uninstall.ExitCode -ne 0){throw 'Uninstall failed'}
        $deadline=[DateTime]::UtcNow.AddSeconds(30)
        while((Test-Path -LiteralPath $app) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 250}
        if(Test-Path -LiteralPath $app){throw 'Uninstall left the installed application executable'}
        if(!(Test-Path -LiteralPath (Join-Path $env:SMARTI_SEARCH_DATA_DIR 'data/metadata.sqlite'))){throw 'Uninstall removed private data'}
    }
    $verification.installed_smoke_test=$true
    $verification|Add-Member -NotePropertyName uninstall_smoke_test -NotePropertyValue (!$KeepInstalled) -Force
    $verification|ConvertTo-Json|Set-Content artifacts/release/verification.json -Encoding utf8
    Copy-Item -LiteralPath $report -Destination 'artifacts/release/native-smoke.json' -Force
    Copy-Item -LiteralPath (Join-Path $testBase 'inference-fallback/health.json') -Destination 'artifacts/release/inference-fallback.json' -Force
    Write-Output 'Installed native app, bundled real inference, PDF preview and uninstall smoke PASS'
} finally {$env:SMARTI_SEARCH_DATA_DIR=$previousData}
