$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
python (Join-Path $taskRoot 'scripts/fetch_model.py')
if($LASTEXITCODE){throw 'Pinned model download or verification failed'}
