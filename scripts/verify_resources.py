from pathlib import Path
import json,hashlib,subprocess,sys
root=Path(__file__).resolve().parents[1]
resources=root/'resources'
manifest=json.loads((resources/'models/model-manifest.json').read_text())
model=resources/'models'/manifest['file_name']
assert model.stat().st_size==manifest['size']
with model.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==manifest['sha256']
for file in ('inference/smarti-local-search-inference.exe','ffmpeg/bin/ffmpeg.exe','ffmpeg/bin/ffprobe.exe','licenses/THIRD_PARTY_NOTICES.md'):
    assert (resources/file).is_file(),file
assert list((resources/'inference').rglob('litert-lm.dll'))
assert list((resources/'inference').rglob('pdfium.dll'))
native=json.loads((resources/'native/manifest.json').read_text(encoding='utf-8-sig'))
for file in native['files']:
    path=resources/'native'/file['name'];assert path.stat().st_size==file['size']
    with path.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==file['sha256']
process=subprocess.run([str(resources/'inference/smarti-local-search-inference.exe'),'--help'],capture_output=True,timeout=30)
assert process.returncode==0,process.stderr.decode(errors='replace')
print('Release resources, model integrity and standalone inference host verified')
