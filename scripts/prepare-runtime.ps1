$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$visualStudio=& $vswhere -latest -products '*' -property installationPath
if(!$visualStudio){throw 'MSVC Build Tools installation not found'}
$version=Get-ChildItem -LiteralPath (Join-Path $visualStudio 'VC/Redist/MSVC') -Directory|Where-Object {$_.Name -match '^\d+\.\d+\.\d+$'}|Sort-Object {[version]$_.Name} -Descending|Select-Object -First 1
$source=Join-Path $version.FullName 'x64/Microsoft.VC143.CRT'
if(!(Test-Path -LiteralPath $source)){throw 'Licensed x64 MSVC runtime directory missing'}
$destination=Join-Path $taskRoot 'resources/native'
New-Item -ItemType Directory -Force -Path $destination|Out-Null
$files=Get-ChildItem -LiteralPath $source -Filter '*.dll'
foreach($file in $files){Copy-Item -LiteralPath $file.FullName -Destination $destination -Force}
$manifest=@{source='Visual Studio 2022 Build Tools redist/x64/Microsoft.VC143.CRT';version=$version.Name;files=@($files|ForEach-Object {@{name=$_.Name;size=$_.Length;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower()}})}
$manifest|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $destination 'manifest.json') -Encoding utf8
Invoke-WebRequest -Uri 'https://aka.ms/VCRedistLicense' -OutFile (Join-Path $taskRoot 'resources/licenses/Microsoft-VC-Runtime-License.html')
Write-Output "Bundled application-local MSVC runtime $($version.Name)"
