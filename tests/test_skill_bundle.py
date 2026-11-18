import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('skill_bundle', Path(__file__).resolve().parents[1] / 'scripts/skill_bundle.py')
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class SkillBundle(unittest.TestCase):
    def test_actual_skill_round_trip_and_content_tamper(self):
        with tempfile.TemporaryDirectory() as directory:
            result = bundle.build(directory)
            with zipfile.ZipFile(result['path']) as archive:
                version = json.loads(archive.read('ai-terminal/version.json'))
                manifest = json.loads(archive.read('bundle-manifest.json'))
                self.assertEqual(version['version'], result['version'])
                self.assertEqual(version['minimum_cli_version'], manifest['minimum_cli_version'])
                self.assertIn('software_version', manifest)
            self.assertTrue(bundle.verify(result['path'])['ok'])
            bad = Path(directory) / 'tampered.zip'
            with zipfile.ZipFile(result['path']) as source, zipfile.ZipFile(bad, 'w') as target:
                for name in source.namelist():
                    data = source.read(name)
                    if name == 'ai-terminal/SKILL.md':
                        data += b'changed'
                    target.writestr(name, data)
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                bundle.verify(bad)

    def test_declared_skill_version_cannot_differ_even_with_valid_checksums(self):
        with tempfile.TemporaryDirectory() as directory:
            good = bundle.build(directory)
            bad = Path(directory) / 'wrong-version.zip'
            with zipfile.ZipFile(good['path']) as source:
                files = {name: source.read(name) for name in source.namelist()}
            manifest = json.loads(files['bundle-manifest.json'])
            manifest['version'] = '99.0.0'
            files['bundle-manifest.json'] = json.dumps(manifest).encode()
            with zipfile.ZipFile(bad, 'w') as target:
                for name, data in files.items():
                    target.writestr(name, data)
            with self.assertRaisesRegex(ValueError, 'version metadata differs'):
                bundle.verify(bad)

    def test_old_released_skill_archives_remain_verifiable(self):
        with tempfile.TemporaryDirectory() as directory:
            good = bundle.build(directory)
            old = Path(directory) / 'legacy.zip'
            with zipfile.ZipFile(good['path']) as source:
                files = {name: source.read(name) for name in source.namelist() if name != 'ai-terminal/version.json'}
            manifest = json.loads(files['bundle-manifest.json'])
            manifest['schema_version'] = 1
            manifest.pop('minimum_cli_version')
            manifest['files'].pop('ai-terminal/version.json')
            files['bundle-manifest.json'] = json.dumps(manifest).encode()
            with zipfile.ZipFile(old, 'w') as target:
                for name, data in files.items():
                    target.writestr(name, data)
            self.assertTrue(bundle.verify(old)['ok'])

    def test_unsafe_member_is_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as directory:
            good = bundle.build(directory)
            bad = Path(directory) / 'unsafe.zip'
            with zipfile.ZipFile(good['path']) as source, zipfile.ZipFile(bad, 'w') as target:
                for name in source.namelist():
                    target.writestr(name, source.read(name))
                target.writestr('../outside', b'never extracted')
            with self.assertRaisesRegex(ValueError, 'Unsafe archive member'):
                bundle.verify(bad)
            self.assertFalse((Path(directory).parent / 'outside').exists())

    def test_declared_management_api_cannot_omit_cli_and_skill(self):
        import sys
        sys.path.insert(0, str(bundle.ROOT / 'scripts'))
        import package
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'missing.zip'
            files = {name: b'fixture' for name in ['ai-terminal.exe', 'ai-terminal-hook.exe', 'LICENSE.txt', 'Hack-LICENSE.txt', 'egui-term-LICENSE.txt']}
            manifest = {'version': 'test', 'management_cli': {'schema_version': 1},
                        'files': {name: {'bytes': len(data), 'sha256': bundle.digest(data)} for name, data in files.items()}}
            with zipfile.ZipFile(path, 'w') as archive:
                for name, data in files.items():
                    archive.writestr(name, data)
                archive.writestr('manifest.json', json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'management CLI/skill incomplete'):
                package.verify_archive(path)

    def test_software_bundle_cannot_misstate_skill_compatibility(self):
        import sys
        sys.path.insert(0, str(bundle.ROOT / 'scripts'))
        import package
        with tempfile.TemporaryDirectory() as directory:
            names = ['ai-terminal.exe', 'ai-terminal-hook.exe', 'ai-terminal-cli.exe', 'LICENSE.txt',
                     'Hack-LICENSE.txt', 'egui-term-LICENSE.txt', 'skills/ai-terminal/SKILL.md',
                     'docs/cli.md', 'docs/skills.md']
            files = {name: b'fixture' for name in names}
            data = (bundle.ROOT / 'skills/ai-terminal/version.json').read_bytes()
            files['skills/ai-terminal/version.json'] = data
            version = bundle.metadata(data)
            manifest = {'version': 'test', 'management_cli': {
                'schema_version': 1, 'skill_version': version['version'],
                'minimum_cli_version': version['minimum_cli_version']},
                'files': {name: {'bytes': len(value), 'sha256': bundle.digest(value)} for name, value in files.items()}}
            for name, declared in [('good', version['minimum_cli_version']), ('bad', '99.0.0')]:
                manifest['management_cli']['minimum_cli_version'] = declared
                path = Path(directory) / (name + '.zip')
                with zipfile.ZipFile(path, 'w') as archive:
                    for member, value in files.items():
                        archive.writestr(member, value)
                    archive.writestr('manifest.json', json.dumps(manifest))
                if name == 'good':
                    self.assertTrue(package.verify_archive(path)['ok'])
                else:
                    with self.assertRaisesRegex(ValueError, 'skill version mismatch'):
                        package.verify_archive(path)
