param(
    [Parameter(Mandatory)][string]$SourceNsi,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][string]$TestName,
    [string]$BinarySource,
    [string]$Hooks
)
$ErrorActionPreference='Stop'
if($TestName -notmatch '^[a-z0-9-]+$'){throw 'Invalid QA name'}
$taskRoot=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(!$output.StartsWith($taskRoot+'\artifacts\',[StringComparison]::OrdinalIgnoreCase)){throw 'QA output must be under workspace artifacts'}
New-Item -ItemType Directory -Force -Path $output | Out-Null
$source=(Resolve-Path -LiteralPath $SourceNsi).Path
$text=Get-Content -LiteralPath $source -Raw
if(([regex]::Matches($text,'(?m)^!define PRODUCTNAME "Smarti Local Search"\r?$')).Count -ne 1 -or
   ([regex]::Matches($text,'(?m)^!define BUNDLEID "com.smarti.localsearch"\r?$')).Count -ne 1){throw 'Unexpected NSIS identity; refusing QA repack'}
# Repack the generated recipe with an isolated registry/shortcut identity. The
# application and every runtime resource remain the actual release payload.
$text=$text.Replace('!define PRODUCTNAME "Smarti Local Search"',('!define PRODUCTNAME "Smarti Local Search QA '+$TestName+'"'))
$text=$text.Replace('!define BUNDLEID "com.smarti.localsearch"',('!define BUNDLEID "com.smarti.localsearch.qa.'+$TestName+'"'))
if($text.Contains('!define PRODUCTNAME "Smarti Local Search"') -or $text.Contains('!define BUNDLEID "com.smarti.localsearch"')){throw 'QA isolation failed'}
$text=$text.Replace('SetCompressor /SOLID "lzma"','SetCompress off')
if($BinarySource){
    $binary=(Resolve-Path -LiteralPath $BinarySource).Path
    $text=[regex]::Replace($text,'(?m)^!define MAINBINARYSRCPATH .*$',('!define MAINBINARYSRCPATH "'+$binary+'"'))
}
if($Hooks){
    $hookPath=(Resolve-Path -LiteralPath $Hooks).Path
    $text=[regex]::Replace($text,'(?m)^!include ".*installer-hooks\.nsh"\r?$',('!include "'+$hookPath+'"'))
}
foreach($name in @('utils.nsh','FileAssociation.nsh')){
    Copy-Item -LiteralPath (Join-Path (Split-Path $source -Parent) $name) -Destination $output -Force
}
$recipe=Join-Path $output 'installer.nsi'
Set-Content -LiteralPath $recipe -Value $text -Encoding utf8
$compiler=Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe'
Push-Location -LiteralPath $output
try {
    & $compiler /INPUTCHARSET UTF8 /V2 $recipe
    if($LASTEXITCODE){throw 'QA installer compilation failed'}
} finally {Pop-Location}
Write-Output (Join-Path $output 'nsis-output.exe')
