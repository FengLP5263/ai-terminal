#!/usr/bin/env python3
"""Build/verify a standalone AI Terminal Agent Skill ZIP.

Examples:
    python scripts/skill_bundle.py build
    python scripts/skill_bundle.py verify --path output/skills/package.zip
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import re
from pathlib import Path, PurePosixPath
import tomllib
import uuid
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def metadata(data):
    record = json.loads(data)
    for key in ('version', 'minimum_cli_version'):
        if not isinstance(record.get(key), str) or not re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?', record[key]):
            raise ValueError('Invalid skill ' + key)
    if record.get('schema_version') != 1 or record.get('name') != 'ai-terminal' or record.get('control_schema') != 1:
        raise ValueError('Unsupported skill version metadata')
    return record


def digest(data):
    return hashlib.sha256(data).hexdigest()


def members(root=ROOT):
    source = root / 'skills/ai-terminal'
    files = {f'ai-terminal/{p.relative_to(source).as_posix()}': p.read_bytes()
             for p in sorted(source.rglob('*')) if p.is_file() and not p.is_symlink()}
    required = {'ai-terminal/SKILL.md', 'ai-terminal/SKILL.en.md',
                'ai-terminal/agents/openai.yaml', 'ai-terminal/version.json'}
    if not required.issubset(files):
        raise ValueError('Skill entry/metadata missing')
    for name in files:
        if not name.endswith(('.md', '.yaml', '.json')):
            raise ValueError('Unexpected skill resource: ' + name)
    files['LICENSE.txt'] = (root / 'LICENSE').read_bytes()
    metadata(files['ai-terminal/version.json'])
    return files


def verify(path):
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        names = [i.filename for i in infos]
        if len(names) != len(set(n.casefold() for n in names)):
            raise ValueError('Duplicate archive members')
        if len(names) > 128 or sum(i.file_size for i in infos) > 2 * 1024 * 1024:
            raise ValueError('Skill archive exceeds limits')
        for item in infos:
            p = PurePosixPath(item.filename)
            if p.is_absolute() or '..' in p.parts or '\\' in item.filename or ':' in item.filename:
                raise ValueError('Unsafe archive member')
            if (item.external_attr >> 16) & 0o170000 == 0o120000 or item.flag_bits & 1:
                raise ValueError('Encrypted or symbolic-link member')
        manifest = json.loads(archive.read('bundle-manifest.json'))
        expected = set(manifest['files']) | {'bundle-manifest.json'}
        if set(names) != expected or not {'ai-terminal/SKILL.md', 'LICENSE.txt'}.issubset(expected):
            raise ValueError('Skill manifest members differ')
        for name, record in manifest['files'].items():
            data = archive.read(name)
            if len(data) != record['bytes'] or digest(data) != record['sha256']:
                raise ValueError('Skill checksum mismatch: ' + name)
        if manifest.get('schema_version', 1) == 2:
            record = metadata(archive.read('ai-terminal/version.json'))
            if record['version'] != manifest['version'] or record['minimum_cli_version'] != manifest['minimum_cli_version']:
                raise ValueError('Skill version metadata differs from manifest')
        elif manifest.get('schema_version', 1) != 1:
            raise ValueError('Unsupported skill bundle schema')
    return {'ok': True, 'archive': Path(path).name, 'version': manifest['version'],
            'sha256': digest(Path(path).read_bytes()), 'files': len(names)}


def build(output=None, root=ROOT):
    software_version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    files = members(root)
    record = metadata(files['ai-terminal/version.json'])
    version = record['version']
    manifest = {'schema_version': 2, 'version': version, 'minimum_cli_version': record['minimum_cli_version'],
                'software_version': software_version, 'source_repository': record['source_repository'], 'control_schema': 1,
                'files': {n: {'bytes': len(b), 'sha256': digest(b)} for n, b in files.items()}}
    files['bundle-manifest.json'] = (json.dumps(manifest, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    folder = Path(output) if output else root / 'output/skills'
    folder.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    path = folder / f'ai-terminal-skills-v{version}-{stamp}-{uuid.uuid4().hex[:6]}.zip'
    with zipfile.ZipFile(path, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(files.items()):
            archive.writestr(name, data)
    result = verify(path)
    path.with_suffix('.zip.sha256').write_text(f'{result["sha256"]}  {path.name}\n', encoding='utf-8', newline='\n')
    return {**result, 'path': str(path)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['build', 'verify'])
    parser.add_argument('--output')
    parser.add_argument('--path')
    args = parser.parse_args()
    if args.action == 'verify' and not args.path:
        parser.error('verify requires --path')
    result = build(args.output) if args.action == 'build' else verify(args.path)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
