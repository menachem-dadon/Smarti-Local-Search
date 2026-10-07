"""Build-time only; exact immutable revision and SHA-256, atomic installation."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request
import time

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / 'resources/models/model-manifest.json').read_text())
target = root / 'resources/models' / manifest['file_name']
def digest(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()
if target.exists() and target.stat().st_size == manifest['size'] and digest(target) == manifest['sha256']:
    print('Model already verified', flush=True)
else:
    url = f'https://huggingface.co/{manifest["model_id"]}/resolve/{manifest["source_revision"]}/{manifest["file_name"]}'
    temp = target.with_suffix('.download')
    print('Fetching pinned model', flush=True)
    for attempt in range(4):
        offset = temp.stat().st_size if temp.exists() else 0
        request = urllib.request.Request(url, headers={'Range': f'bytes={offset}-'} if offset else {})
        try:
            with urllib.request.urlopen(request, timeout=600) as source:
                with temp.open('ab' if source.status == 206 else 'wb') as sink:
                    while data := source.read(1024 * 1024): sink.write(data)
            break
        except (TimeoutError, OSError) as error:
            print(f'Download interrupted, retry {attempt + 1}: {type(error).__name__}', flush=True)
            if attempt == 3: raise
            time.sleep(2)
    if temp.stat().st_size != manifest['size'] or digest(temp) != manifest['sha256']:
        temp.unlink(missing_ok=True)
        raise SystemExit('Model integrity verification FAILED')
    os.replace(temp, target)
    print(f'Model verified: {target.stat().st_size} bytes', flush=True)
