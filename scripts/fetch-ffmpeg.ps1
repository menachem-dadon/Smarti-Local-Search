$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
$manifestPath=Join-Path $taskRoot 'resources/ffmpeg/manifest.json'
New-Item -ItemType Directory -Force -Path (Split-Path $manifestPath -Parent),(Join-Path $taskRoot 'artifacts') | Out-Null
if(Test-Path -LiteralPath $manifestPath){
    $manifest=Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
}else{
    $releases=Invoke-RestMethod -Uri 'https://api.github.com/repos/BtbN/FFmpeg-Builds/releases?per_page=8'
    $release=$releases | Where-Object {$_.tag_name -ne 'latest'} | Select-Object -First 1
    $asset=$release.assets | Where-Object {$_.name -match '^ffmpeg-n8\.1.*win64-lgpl-shared.*\.zip$'} | Select-Object -First 1
    if(!$asset){$asset=$release.assets | Where-Object {$_.name -match '^ffmpeg-master.*win64-lgpl-shared.*\.zip$'} | Select-Object -First 1}
    if(!$asset){throw 'No LGPL Windows FFmpeg asset found'}
    $checksums=$release.assets | Where-Object {$_.name -eq 'checksums.sha256'} | Select-Object -First 1
    $lines=(Invoke-WebRequest -Uri $checksums.browser_download_url).Content
    if($lines -is [byte[]]){$lines=[Text.Encoding]::UTF8.GetString($lines)}
    $checksum=($lines -split "`n" | Where-Object {$_ -match [regex]::Escape($asset.name)} | Select-Object -First 1).Split(' ')[0]
    if($checksum -notmatch '^[0-9a-f]{64}$'){throw 'FFmpeg checksum unavailable'}
    $manifest=@{source='BtbN/FFmpeg-Builds';revision=$release.tag_name;filename=$asset.name;url=$asset.browser_download_url;sha256=$checksum;license='LGPL';size=$asset.size}
    $manifest | ConvertTo-Json | Set-Content -LiteralPath $manifestPath -Encoding utf8
}
$zip=Join-Path $taskRoot ('artifacts/'+$manifest.filename)
if(!(Test-Path -LiteralPath $zip) -or (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower() -ne $manifest.sha256){Invoke-WebRequest -Uri $manifest.url -OutFile $zip}
if((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower() -ne $manifest.sha256){throw 'FFmpeg SHA-256 mismatch'}
$expanded=Join-Path $taskRoot 'artifacts/ffmpeg-expanded'
Expand-Archive -LiteralPath $zip -DestinationPath $expanded -Force
$binary=Get-ChildItem -LiteralPath $expanded -Filter ffmpeg.exe -Recurse | Select-Object -First 1
$destination=Join-Path $taskRoot 'resources/ffmpeg/bin'
New-Item -ItemType Directory -Force -Path $destination|Out-Null
Get-ChildItem -LiteralPath $binary.Directory.FullName | ForEach-Object {Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse -Force}
$license=Get-ChildItem -LiteralPath $expanded -Filter LICENSE* -Recurse | Select-Object -First 1
if($license){Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $taskRoot 'resources/licenses/FFmpeg-LICENSE.txt') -Force}
Write-Output "FFmpeg verified and bundled: $($manifest.revision)"
