param([switch]$IncludeResources)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
# Never touch user indexes. Every recursive target is checked inside this checkout.
foreach($relative in @('target','desktop/dist','artifacts/pyinstaller','artifacts/sidecar-dist')){
    $target=[IO.Path]::GetFullPath((Join-Path $taskRoot $relative))
    if(!$target.StartsWith($taskRoot+'\')){throw 'Unsafe clean target'}
    if(Test-Path -LiteralPath $target){Remove-Item -LiteralPath $target -Recurse -Force}
}
if($IncludeResources){foreach($relative in @('resources/inference','resources/ffmpeg/bin')){
    $target=[IO.Path]::GetFullPath((Join-Path $taskRoot $relative))
    if(!$target.StartsWith($taskRoot+'\')){throw 'Unsafe clean target'}
    if(Test-Path -LiteralPath $target){Remove-Item -LiteralPath $target -Recurse -Force}
}}
