"""Tie isolated installer QA to the exact executable and packaged resources."""
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def source_hashes():
    result = {'smarti-local-search.exe': sha(ROOT / 'target/release/smarti-local-search.exe')}
    for file in sorted((ROOT / 'resources').rglob('*')):
        if file.is_file():
            result['resources/' + file.relative_to(ROOT / 'resources').as_posix()] = sha(file)
    config = json.loads((ROOT / 'desktop/src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
    for source, destination in config['bundle']['resources'].items():
        if destination.endswith('.dll'):
            result[destination] = sha(ROOT / 'desktop/src-tauri' / source)
    return result


if __name__ == '__main__':
    mode, *args = sys.argv[1:]
    if mode == 'capture':
        installed, output = map(Path, args)
        installed = installed.resolve()
        assert installed.is_relative_to(ROOT / 'artifacts'), 'QA must stay under workspace artifacts'
        expected = source_hashes()
        for name, digest in expected.items():
            assert sha(installed / name) == digest, f'Installed release payload mismatch: {name}'
        Path(output).write_text(json.dumps(expected, indent=2), encoding='utf-8')
        print(f'Installed payload matches {len(expected)} executable/resource hashes')
    elif mode == 'check':
        expected = json.loads(Path(args[0]).read_text(encoding='utf-8'))
        assert source_hashes() == expected, 'Release payload changed since installed QA'
        print(f'Release still matches all {len(expected)} installed payload hashes')
    else:
        raise SystemExit('Usage: verify-installed-payload.py capture INSTALL OUTPUT | check OUTPUT')
