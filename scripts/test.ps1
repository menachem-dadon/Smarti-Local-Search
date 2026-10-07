$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $taskRoot
function Checked([scriptblock]$Block){& $Block;if($LASTEXITCODE){throw "Validation failed: $Block"}}
Checked {cargo fmt --all --check}
Checked {cargo clippy --workspace --all-targets -- -D warnings}
Checked {cargo test -p smarti-search-core}
Checked {npm --prefix desktop run typecheck}
Checked {npm --prefix desktop test}
Checked {npm --prefix desktop run build}
Checked {cargo test -p smarti-local-search}
Checked {& ./.venv/Scripts/python.exe scripts/generate-fixtures.py}
Checked {& ./.venv/Scripts/python.exe -m unittest discover -s inference-host/tests -v}
Checked {& ./.venv/Scripts/python.exe tests/inference_smoke.py}
Checked {cargo test -p smarti-search-core --test end_to_end -- --ignored --test-threads=1}
Checked {git diff --check}
