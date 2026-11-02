"""Check missing translations and language links without network access."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('check_bilingual', ROOT / 'scripts/check_bilingual.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BilingualChecks(unittest.TestCase):
    def test_missing_pair_and_missing_backlink_fail(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'README.md').write_text('[English](README.en.md)', encoding='utf-8')
            old = module.VENDOR_PAIRS
            module.VENDOR_PAIRS = ()
            try:
                errors, _ = module.check({'README.md'}, root)
                self.assertTrue(any('missing bilingual counterpart' in e for e in errors))
                (root / 'README.en.md').write_text('English only', encoding='utf-8')
                errors, _ = module.check({'README.md', 'README.en.md'}, root)
                self.assertTrue(any('missing reciprocal language link' in e for e in errors))
                (root / 'README.en.md').write_text('[中文](README.md)', encoding='utf-8')
                errors, count = module.check({'README.md', 'README.en.md'}, root)
                self.assertEqual([], errors)
                self.assertEqual(1, len(count))
            finally:
                module.VENDOR_PAIRS = old

    def test_changed_translation_requires_reviewed_manifest(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'scripts').mkdir()
            (root / 'README.md').write_text('[English](README.en.md)', encoding='utf-8')
            (root / 'README.en.md').write_text('[中文](README.md)', encoding='utf-8')
            pairs = {('README.md', 'README.en.md')}
            manifest = root / 'scripts/bilingual-manifest.json'
            manifest.write_text(json.dumps(module.digests(pairs, root)), encoding='utf-8')
            self.assertEqual([], module.verify_snapshots(pairs, root))
            (root / 'README.md').write_text('changed [English](README.en.md)', encoding='utf-8')
            self.assertTrue(any('README.md' in e for e in module.verify_snapshots(pairs, root)))

    def test_new_directory_requires_a_pair(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'new-area').mkdir()
            (root / 'new-area/guide.md').write_text('# Guide', encoding='utf-8')
            old = module.VENDOR_PAIRS
            module.VENDOR_PAIRS = ()
            try:
                errors, _ = module.check({'new-area/guide.md'}, root)
                self.assertTrue(any('missing bilingual counterpart' in e for e in errors))
            finally:
                module.VENDOR_PAIRS = old
    def test_missing_vendor_translation_fails(self):
        old = module.VENDOR_PAIRS
        module.VENDOR_PAIRS = (('vendor/a/UPSTREAM.zh.md', 'vendor/a/UPSTREAM.en.md'),)
        try:
            with tempfile.TemporaryDirectory() as folder:
                errors, _ = module.check(set(), Path(folder))
                self.assertTrue(any('missing vendor translation' in e for e in errors))
        finally:
            module.VENDOR_PAIRS = old


if __name__ == '__main__':
    unittest.main()
