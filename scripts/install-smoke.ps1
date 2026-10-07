param([string]$Installer,[switch]$KeepInstalled,[string]$TestName='release-upgrade',[switch]$WhileBundling)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
Set-Location -LiteralPath $taskRoot
$config=Get-Content desktop/src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
if(!$Installer){$Installer=Join-Path $taskRoot ("artifacts/release/$($config.productName)_$($config.version)_x64-setup.exe")}
if(!$WhileBundling -and !(Test-Path -LiteralPath $Installer)){throw 'Canonical release installer missing'}
if($WhileBundling -and (Select-String -LiteralPath target/release/nsis/x64/installer.nsi -Pattern ('^!define VERSION "'+[regex]::Escape($config.version)+'"$')).Count -ne 1){throw 'The current release NSIS recipe is not ready'}
if($TestName -notmatch '^[a-z0-9-]+$'){throw 'Invalid test name'}
$testBase=Join-Path $taskRoot ('artifacts/installed-qa-'+$TestName+'-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
$installPath=Join-Path $testBase 'application'
$qaProduct='Smarti Local Search QA '+$TestName
$registryPath='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\'+$qaProduct
if(Test-Path -LiteralPath $registryPath){throw 'Previous QA installation exists; inspect it before retrying'}
New-Item -ItemType Directory -Path $testBase -Force | Out-Null
$personalBase=Join-Path $env:LOCALAPPDATA 'Smarti Local Search'
$personalProcessIds=@(Get-CimInstance Win32_Process -Filter "Name='smarti-local-search.exe' OR Name='smarti-local-search-inference.exe'" | Where-Object {$_.ExecutablePath -and $_.ExecutablePath.StartsWith($personalBase+'\',[StringComparison]::OrdinalIgnoreCase)} | Select-Object -ExpandProperty ProcessId)
$personalRegistration=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Smarti Local Search' -ErrorAction SilentlyContinue | ConvertTo-Json
$personalExe=Join-Path $env:LOCALAPPDATA 'Smarti Local Search/smarti-local-search.exe'
$personalHash=if(Test-Path -LiteralPath $personalExe){(Get-FileHash -LiteralPath $personalExe).Hash}else{$null}
function Install([string]$Setup,[string]$Arguments){
    $process=Start-Process -FilePath $Setup -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    $deadline=[DateTime]::UtcNow.AddMinutes(6)
    while(!$process.WaitForExit(1000)){if([DateTime]::UtcNow -gt $deadline){throw 'QA installer timed out'}}
    if($process.ExitCode){throw "QA installer failed: $($process.ExitCode)"}
}
function Wait-Report([string]$Path,[System.Diagnostics.Process]$Process){
    $deadline=[DateTime]::UtcNow.AddMinutes(5)
    while(!(Test-Path -LiteralPath $Path)){
        if($Process.HasExited -or [DateTime]::UtcNow -gt $deadline){throw 'Native QA report missing or timed out'}
        Start-Sleep -Milliseconds 250
    }
    $result=Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if(!$result.passed){throw "Native QA failed: $($result.error)"}
    return $result
}
$python=Join-Path $taskRoot '.venv/Scripts/python.exe'
$baseline=Join-Path $taskRoot 'artifacts/upgrade-baseline-0.1.1/qa/nsis-output.exe'
if(!(Test-Path -LiteralPath $baseline)){$baseline=Join-Path $taskRoot 'artifacts/upgrade-baseline/qa/nsis-output.exe'}
$legacy=Test-Path -LiteralPath $baseline
if($legacy -and $TestName -ne 'release-upgrade'){throw 'Baseline installer has a fixed release-upgrade QA identity'}
if($legacy){
    Install $baseline "/S /NS /D=$installPath"
    & $python scripts/upgrade-fixture.py seed $installPath
    if($LASTEXITCODE){throw 'Legacy fixture creation failed'}
    # An orphaned 0.1.0 worker holds the packaged executable/DLLs open.
    $start=[System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName=Join-Path $installPath 'resources/inference/smarti-local-search-inference.exe'
    $start.UseShellExecute=$false; $start.CreateNoWindow=$true
    $start.RedirectStandardInput=$true; $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
    $start.ArgumentList.Add('--resources'); $start.ArgumentList.Add((Join-Path $installPath 'resources'))
    $start.ArgumentList.Add('--cache'); $start.ArgumentList.Add((Join-Path $installPath 'cache'))
    $orphan=[System.Diagnostics.Process]::Start($start)
    Start-Sleep -Seconds 2
    if($orphan.HasExited){throw 'Legacy worker did not stay alive for upgrade test'}
}
$qaDir=Join-Path $testBase 'installer'
& $PSScriptRoot/new-qa-installer.ps1 -SourceNsi target/release/nsis/x64/installer.nsi -OutputDirectory $qaDir -TestName $TestName
$qaSetup=Join-Path $qaDir 'nsis-output.exe'
$database=Join-Path $installPath 'data/metadata.sqlite'
$kept=Join-Path $installPath 'user-kept.txt'
$location=Join-Path $installPath 'location.json'
if($legacy){
    Set-Content -LiteralPath $kept -Value 'Preserved user file'
    Set-Content -LiteralPath $location -Value '{"path":"QA-relocated-index-pointer"}'
}
$dbHash=if($legacy){(Get-FileHash -LiteralPath $database).Hash}else{$null}
if($legacy){Install $qaSetup '/S /NS'}else{Install $qaSetup "/S /NS /D=$installPath"}
if($legacy){
    if(!$orphan.WaitForExit(10000)){throw 'Upgrade left the legacy inference worker running'}
    $orphan.Dispose()
    if((Get-FileHash -LiteralPath $database).Hash -ne $dbHash){throw 'Installer modified the legacy database'}
    if(!(Test-Path -LiteralPath $kept) -or (Get-Content -LiteralPath $location -Raw).Trim() -ne '{"path":"QA-relocated-index-pointer"}'){throw 'Upgrade removed user files or the index location pointer'}
}
$app=Join-Path $installPath 'smarti-local-search.exe'
if((Get-FileHash -LiteralPath $app).Hash -ne (Get-FileHash target/release/smarti-local-search.exe).Hash){throw 'Installed QA app differs from the release payload'}
$registered=Get-ItemProperty -LiteralPath $registryPath
if($registered.DisplayVersion -ne $config.version){throw 'Upgrade registration has the wrong version'}
$previousData=$env:SMARTI_SEARCH_DATA_DIR
try {
    $env:SMARTI_SEARCH_DATA_DIR=$installPath
    $probe=Join-Path $testBase 'upgrade-probe.json'
    $running=Start-Process -FilePath $app -ArgumentList ('"--upgrade-probe='+$probe+'"') -WindowStyle Hidden -PassThru
    $null=Wait-Report $probe $running
    if($legacy){
        & $python scripts/upgrade-fixture.py check $installPath
        if($LASTEXITCODE){throw 'Legacy index migration failed'}
    }
    $workers=@(Get-CimInstance Win32_Process -Filter "Name='smarti-local-search-inference.exe'" | Where-Object {$_.ExecutablePath -like ($installPath+'\*')} | Select-Object -ExpandProperty ProcessId)
    if(!$workers.Count){throw 'Packaged worker missing during running-app upgrade test'}
    $obsolete=Join-Path $installPath 'resources/inference/_internal/obsolete-version.dll'
    Set-Content -LiteralPath $obsolete -Value 'QA stale runtime file'
    # A conflicting /D must not create a second copy; the saved path wins.
    $otherPath=Join-Path $testBase 'conflicting-directory'
    Install $qaSetup "/S /NS /D=$otherPath"
    if(!$running.WaitForExit(10000)){throw 'Reinstallation left the previous app running'}
    foreach($workerId in $workers){if(Get-Process -Id $workerId -ErrorAction SilentlyContinue){throw 'Reinstallation left an inference worker running'}}
    if((Test-Path -LiteralPath (Join-Path $otherPath 'smarti-local-search.exe')) -or (Test-Path -LiteralPath $obsolete)){throw 'Upgrade left a duplicate installation or stale runtime file'}
    if((Test-Path -LiteralPath $otherPath) -and @(Get-ChildItem -LiteralPath $otherPath -Force).Count){throw 'Conflicting install directory contains an unexpected payload'}
    Set-ItemProperty -LiteralPath $registryPath -Name DisplayVersion -Value '99.0.0'
    $downgrade=Start-Process -FilePath $qaSetup -ArgumentList '/S /NS' -WindowStyle Hidden -PassThru -Wait
    if($downgrade.ExitCode -eq 0){throw 'Silent downgrade was accepted'}
    Set-ItemProperty -LiteralPath $registryPath -Name DisplayVersion -Value $config.version
    $report=Join-Path $testBase 'native-report.json'; $corpus=Join-Path $taskRoot 'tests/fixtures/corpus'
    $process=Start-Process -FilePath $app -ArgumentList @(('"--smoke-test='+$report+'"'),('"--smoke-root='+$corpus+'"')) -WindowStyle Hidden -PassThru
    $null=Wait-Report $report $process
    if(!$process.WaitForExit(10000) -or $process.ExitCode){throw 'Native smoke app did not exit successfully'}
    if($legacy){
        & $python scripts/upgrade-fixture.py check $installPath
        if($LASTEXITCODE){throw 'Native smoke lost preserved legacy settings/index'}
    }
    $resources=Join-Path $installPath 'resources'
    & $python tests/inference_smoke.py --host (Join-Path $resources 'inference/smarti-local-search-inference.exe') --resources $resources --cache (Join-Path $testBase 'inference-fallback') --accelerator npu
    if($LASTEXITCODE){throw 'Installed inference/fallback validation failed'}
    & $python scripts/verify-installed-payload.py capture $installPath artifacts/release/installed-payload.json
    if($LASTEXITCODE){throw 'Installed resource payload verification failed'}
    if(!$KeepInstalled){
        Install (Join-Path $installPath 'uninstall.exe') '/S'
        $deadline=[DateTime]::UtcNow.AddSeconds(30)
        while((Test-Path -LiteralPath $app) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 250}
        if((Test-Path -LiteralPath $app) -or !(Test-Path -LiteralPath $database) -or (Test-Path -LiteralPath $registryPath)){throw 'QA uninstall failed or deleted private data'}
        if($legacy -and (!(Test-Path -LiteralPath $kept) -or !(Test-Path -LiteralPath $location))){throw 'Uninstall removed user files or the index location pointer'}
    }
    if($personalHash -and (Get-FileHash -LiteralPath $personalExe).Hash -ne $personalHash){throw 'QA modified the personal application'}
    $after=Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Smarti Local Search' -ErrorAction SilentlyContinue | ConvertTo-Json
    if($after -ne $personalRegistration){throw 'QA modified the personal uninstall registration'}
    foreach($personalId in $personalProcessIds){if(!(Get-Process -Id $personalId -ErrorAction SilentlyContinue)){throw 'QA stopped an existing personal process'}}
    $verification=[pscustomobject]@{version=$config.version;exe_sha256=(Get-FileHash target/release/smarti-local-search.exe).Hash}
    $verification | Add-Member -NotePropertyName installed_smoke_test -NotePropertyValue $true
    foreach($entry in @{uninstall_smoke_test=(!$KeepInstalled);legacy_upgrade_test=$legacy;running_app_upgrade_test=$true;duplicate_install_prevented=$true;stale_runtime_removed=$true;silent_downgrade_blocked=$true;personal_install_preserved=$true;installer_test_identity=$qaProduct;installer_test_recipe='Generated release NSIS recipe repacked with QA product/registry identity; identical release executable and resources'}.GetEnumerator()){
        $verification | Add-Member -NotePropertyName $entry.Key -NotePropertyValue $entry.Value -Force
    }
    $verification | ConvertTo-Json | Set-Content artifacts/release/installed-verification.json -Encoding utf8
    if(!$WhileBundling){& $PSScriptRoot/verify-release.ps1}
    Copy-Item -LiteralPath $report -Destination artifacts/release/native-smoke.json -Force
    Copy-Item -LiteralPath (Join-Path $testBase 'inference-fallback/health.json') -Destination artifacts/release/inference-fallback.json -Force
    Write-Output 'Native release, legacy/running upgrades, data preservation, downgrade guard and uninstall PASS'
} finally {$env:SMARTI_SEARCH_DATA_DIR=$previousData}
