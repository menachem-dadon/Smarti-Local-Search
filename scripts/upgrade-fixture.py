"""Create/check a private legacy index for native installer upgrade tests."""
import json
from pathlib import Path
import shutil
import sqlite3
import sys

ROOT = Path(__file__).resolve().parents[1]


def snapshot(base):
    with sqlite3.connect(base / 'data/metadata.sqlite') as db:
        settings = json.loads(db.execute("SELECT value FROM settings WHERE key='app'").fetchone()[0])
        return {
            'settings': {key: settings[key] for key in ('language', 'theme', 'threads', 'shortcut', 'index_path')},
            'custom_excluded': 'qa-private-exclusion' in settings['exclusions'],
            'roots': db.execute('SELECT id,path,exclusions FROM roots ORDER BY id').fetchall(),
            'files': db.execute('SELECT id,path,semantic FROM files ORDER BY id').fetchall(),
            'chunks': db.execute('SELECT count(*) FROM chunks WHERE vector IS NOT NULL').fetchone()[0],
        }


def main():
    mode, target = sys.argv[1:]
    base = Path(target).resolve()
    assert base.is_relative_to(ROOT / 'artifacts'), 'Private fixture must stay inside artifacts'
    expected_path = base.parent / 'legacy-index.json'
    if mode == 'seed':
        candidates = sorted((ROOT / 'artifacts').glob('installed-smoke*/private-index/data/metadata.sqlite'))
        assert candidates, 'A validated 0.1.0 private index is required for upgrade testing'
        source = candidates[-1]
        (base / 'data').mkdir(parents=True, exist_ok=True)
        with sqlite3.connect(source) as old, sqlite3.connect(base / 'data/metadata.sqlite') as new:
            old.backup(new)
            settings = json.loads(new.execute("SELECT value FROM settings WHERE key='app'").fetchone()[0])
            settings.update(language='en', theme='dark', threads=2, shortcut='Ctrl+Alt+Shift+F11', index_path=str(base))
            settings['exclusions'].append('qa-private-exclusion')
            new.execute("UPDATE settings SET value=? WHERE key='app'", (json.dumps(settings),))
            new.execute("INSERT OR REPLACE INTO settings VALUES('index_stopped','1')")
            new.execute("UPDATE roots SET exclusions=?", (json.dumps(['qa-root-exclusion']),))
        shutil.copytree(source.parent / 'vectors', base / 'data/vectors', dirs_exist_ok=True)
        expected_path.write_text(json.dumps(snapshot(base), indent=2), encoding='utf-8')
    elif mode == 'check':
        expected = json.loads(expected_path.read_text(encoding='utf-8'))
        actual = json.loads(json.dumps(snapshot(base)))
        # 0.1.2 intentionally unifies the legacy root exclusions. Their scope
        # and all indexed IDs/vectors/settings still need to survive upgrading.
        with sqlite3.connect(base / 'data/metadata.sqlite') as db:
            settings = json.loads(db.execute("SELECT value FROM settings WHERE key='app'").fetchone()[0])
            for _, path, rules in expected['roots']:
                for rule in json.loads(rules):
                    scoped = path.replace('\\', '/').lower().rstrip('/') + '/**/' + rule
                    assert scoped in settings['exclusions'], 'Legacy scoped exclusion lost'
            expected['roots'] = [[id, path, '[]'] for id, path, _ in expected['roots']]
        assert actual == expected, 'Legacy settings, roots, file IDs or vectors changed during upgrade'
        with sqlite3.connect(base / 'data/metadata.sqlite') as db:
            assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
            assert any(row[1] == 'priority' for row in db.execute('PRAGMA table_info(jobs)'))
        print('Legacy index, custom settings/exclusions and new queue schema PASS')
    else:
        raise ValueError('Use seed or check')


if __name__ == '__main__':
    main()
