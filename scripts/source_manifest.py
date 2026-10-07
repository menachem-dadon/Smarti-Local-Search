"""Guard release packaging against edits made while the compiler was running."""
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'artifacts/release/source-manifest.json'
DIRECTORIES = ('crates/search-core/src', 'crates/search-core/tests', 'desktop/src',
               'desktop/src-tauri/src', 'desktop/src-tauri/capabilities', 'desktop/src-tauri/icons',
               'inference-host/src', 'inference-host/tests', 'scripts', 'tests')
FILES = ('Cargo.toml', 'Cargo.lock', 'package.json', 'desktop/package.json', 'desktop/package-lock.json',
         'desktop/vite.config.ts', 'desktop/tsconfig.json', 'desktop/src-tauri/Cargo.toml',
         'desktop/src-tauri/build.rs', 'desktop/src-tauri/tauri.conf.json',
         'desktop/src-tauri/installer-hooks.nsh', 'desktop/src-tauri/Hebrew.nsh',
         'inference-host/requirements.lock', 'inference-host/requirements.transitive.lock',
         'resources/models/model-manifest.json', 'resources/ffmpeg/manifest.json',
         'resources/native/manifest.json')

def snapshot():
    paths = {ROOT / name for name in FILES}
    for directory in DIRECTORIES:
        paths.update(path for path in (ROOT / directory).rglob('*')
                     if path.is_file() and '__pycache__' not in path.parts)
    return {path.relative_to(ROOT).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(paths)}

if __name__ == '__main__':
    current = snapshot()
    if sys.argv[1:] == ['capture']:
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        MANIFEST.write_text(json.dumps(current, indent=2), encoding='utf-8')
        print(f'Captured {len(current)} source/configuration hashes before compilation')
    elif sys.argv[1:] == ['check']:
        expected = json.loads(MANIFEST.read_text(encoding='utf-8'))
        changed = [name for name in expected.keys() | current.keys()
                   if expected.get(name) != current.get(name)]
        if changed:
            raise SystemExit('Source changed during build; rebuild before release: ' + ', '.join(changed))
        print(f'Release sources match all {len(current)} captured hashes')
    else:
        raise SystemExit('Usage: source_manifest.py capture|check')
