import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('installer', Path(__file__).resolve().parents[1] / 'scripts/installer.py')
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class InstallerValidation(unittest.TestCase):
    def temporary(self):
        root = Path(__file__).resolve().parents[1] / 'output/installer-unit'
        root.mkdir(parents=True, exist_ok=True)
        return tempfile.TemporaryDirectory(dir=root)

    def test_member_validation_rejects_windows_aliases_collisions_and_script_injection(self):
        for members in [['../outside'], ['a\\outside'], ['C:/outside'], ['NUL.txt'],
                        ['name.'], ['a/A', 'a/a'], ['a"\nFileWrite $0 bad'], ['a:b'], ['a//b'], ['a/./b']]:
            with self.assertRaises(ValueError):
                installer.windows_members(members)
        self.assertEqual(installer.nsis_string('a$foo"b'), 'a$$foo$\\"b')
        self.assertEqual(installer.windows_members(['fonts/来源.txt', 'name$dollar']), ['fonts/来源.txt', 'name$dollar'])

    def test_uninstall_is_an_explicit_owned_file_list_and_never_recurses(self):
        with self.temporary() as folder:
            stage = Path(folder)
            installer.write_includes(stage, ['ai-terminal.exe', 'licenses/demo/LICENSE.txt'], {'APP_VERSION': 'test'})
            text = (stage / 'uninstall-files.nsh').read_text(encoding='utf-8-sig')
            self.assertIn('Delete "$INSTDIR\\ai-terminal.exe"', text)
            self.assertIn('RMDir "$INSTDIR\\licenses\\demo"', text)
            self.assertNotIn('/r', text.lower())
            self.assertNotIn('sessions.json', text)
            checks = (stage / 'check-files.nsh').read_text(encoding='utf-8-sig')
            self.assertIn('CheckDirectory "$INSTDIR\\licenses"', checks)
            self.assertIn('CheckFile "ai-terminal.exe"', checks)

    def test_sidecar_verification_detects_changed_program_and_escaping_paths(self):
        with self.temporary() as folder:
            root = Path(folder)
            file = root / 'setup.exe'
            file.write_bytes(b'installer fixture')
            digest = hashlib.sha256(file.read_bytes()).hexdigest()
            file.with_suffix('.sha256').write_text(digest + '  setup.exe\n', encoding='ascii')
            record = {'format_version': 1, 'platform': 'windows-x64', 'version': 'test', 'acceptance': True,
                      'artifact': {'name': file.name, 'bytes': file.stat().st_size, 'sha256': digest}}
            meta = root / 'setup.json'
            meta.write_text(json.dumps(record), encoding='utf-8')
            self.assertTrue(installer.verify(meta)['ok'])
            file.write_bytes(b'changed installer')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                installer.verify(meta)
            record['artifact']['name'] = '../outside.exe'
            meta.write_text(json.dumps(record), encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'unsafe'):
                installer.verify(meta)
