import json
import urllib.request
from pathlib import Path

out = Path(__file__).resolve().parent / 'vendor'
out.mkdir(exist_ok=True)
def read(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read()

for name in ('litert-lm-api', 'litert-lm-api-nightly', 'msgpack', 'pyinstaller', 'pypdfium2'):
    try:
        info = json.loads(read(f'https://pypi.org/pypi/{name}/json'))
        print(name, info['info']['version'], [(f['filename'], f['url']) for f in info['urls'] if 'win_amd64' in f['filename'] or f['filename'].endswith('py3-none-any.whl')][:3], flush=True)
    except Exception as e:
        print(name, str(e), flush=True)
info = json.loads(read('https://huggingface.co/api/models/litert-community/embeddinggemma-2-740m-litert-lm?blobs=true'))
(out / 'model-info.json').write_text(json.dumps(info, indent=2), encoding='utf-8')
print('model', info['sha'], [s for s in info['siblings'] if s['rfilename']=='embeddinggemma-2-740m.litertlm'], flush=True)
for name in ('embedding_engine.py', '_messages.py', 'interfaces.py'):
    (out / name).write_bytes(read(f'https://raw.githubusercontent.com/google-ai-edge/LiteRT-LM/main/python/litert_lm/{name}'))
(out / 'model-card.md').write_bytes(read(f'https://huggingface.co/litert-community/embeddinggemma-2-740m-litert-lm/raw/{info["sha"]}/README.md'))
print('sources saved', flush=True)
