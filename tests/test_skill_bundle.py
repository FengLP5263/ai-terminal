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
