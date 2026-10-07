from pathlib import Path
import json,subprocess,shutil,sys,importlib.metadata as metadata
root=Path(__file__).resolve().parents[1]
out=root/'resources/licenses';out.mkdir(parents=True,exist_ok=True)
items=[]
python_license=Path(sys.base_prefix)/'LICENSE.txt'
if python_license.is_file():shutil.copyfile(python_license,out/'CPython-LICENSE.txt')
for dist in metadata.distributions():
    name=dist.metadata['Name'];version=dist.version
    license=dist.metadata.get('License-Expression') or dist.metadata.get('License') or 'See bundled license'
    items.append(f'Python: {name} {version} — {license.splitlines()[0]}')
    for file in dist.files or []:
        if ('license' in file.name.lower() or 'notice' in file.name.lower()) and file.suffix.lower() in ('','.txt','.md'):
            source=Path(dist.locate_file(file))
            if source.is_file():
                target=out/'python'/f'{name}-{version}'/file.name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target)
def package_manifests(directory):
    if not directory.is_dir():return
    for entry in directory.iterdir():
        if entry.name.startswith('.'):continue
        if entry.name.startswith('@'):
            for scoped in entry.iterdir():
                if (scoped/'package.json').is_file():
                    yield scoped/'package.json'
                    yield from package_manifests(scoped/'node_modules')
        elif (entry/'package.json').is_file():
            yield entry/'package.json'
            yield from package_manifests(entry/'node_modules')
for package in package_manifests(root/'desktop/node_modules'):
    if 'node_modules' not in package.parts:continue
    try:info=json.loads(package.read_text(encoding='utf-8'))
    except (ValueError,UnicodeError):continue
    if not info.get('name'):continue
    items.append(f'npm: {info["name"]} {info.get("version", "")} — {info.get("license", "See bundled license")}')
    for source in package.parent.glob('*'):
        if source.is_file() and any(t in source.name.lower() for t in ('license','notice')):
            target=out/'npm'/info['name'].replace('/','_')/source.name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target)
data=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],cwd=root))
for package in data['packages']:
    if package.get('source') is None:continue
    items.append(f'Rust: {package["name"]} {package["version"]} — {package.get("license", "See bundled license")}')
    for source in Path(package['manifest_path']).parent.glob('*'):
        if source.is_file() and any(t in source.name.lower() for t in ('license','notice')):
            target=out/'rust'/f'{package["name"]}-{package["version"]}'/source.name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,target)
notice=root/'THIRD_PARTY_NOTICES.md'
text=notice.read_text(encoding='utf-8')+'\n\n## Dependency inventory\n\n'+'\n'.join('- '+s for s in sorted(set(items)))+'\n'
(out/'THIRD_PARTY_NOTICES.md').write_text(text,encoding='utf-8')
print('Collected third-party licenses and dependency inventory')
