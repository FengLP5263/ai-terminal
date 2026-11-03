import importlib.util
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('release_package',Path(__file__).resolve().parents[1]/'scripts/package.py')
packaging=importlib.util.module_from_spec(spec);spec.loader.exec_module(packaging)

class ReleaseNotesContract(unittest.TestCase):
    def write(self,root):
        for name,category in [('CHANGELOG.md','修复'),('CHANGELOG.en.md','Fixed')]:
            (root/name).write_text(f'# History\n\n## [1.2.3] - 2026-11-03\n\n### {category}\n\n- Current fix.\n\n## [1.2.2] - 2026-11-02\n\n### {category}\n\n- Previous fix.\n',encoding='utf-8')

    def test_current_entries_are_selected_without_older_changes(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.write(root)
            entries=packaging.release_notes('1.2.3',root)
            self.assertEqual(set(packaging.CHANGELOGS),set(entries))
            self.assertTrue(all('Current fix.' in value and 'Previous fix.' not in value for value in entries.values()))

    def test_missing_wrong_duplicate_or_mismatched_translation_blocks_release(self):
        for change in ['missing','version','duplicate','date']:
            with self.subTest(change=change),tempfile.TemporaryDirectory() as folder:
                root=Path(folder);self.write(root);p=root/'CHANGELOG.en.md';text=p.read_text(encoding='utf-8')
                if change=='missing':p.unlink()
                elif change=='version':p.write_text(text.replace('[1.2.3]','[1.2.4]'),encoding='utf-8')
                elif change=='duplicate':p.write_text(text+'\n## [1.2.3] - 2026-11-03\n\n### Fixed\n- Duplicate.\n',encoding='utf-8')
                else:p.write_text(text.replace('2026-11-03','2026-11-04'),encoding='utf-8')
                with self.assertRaises((ValueError,OSError)):packaging.release_notes('1.2.3',root)

    def test_empty_current_entry_cannot_borrow_older_release_bullets(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.write(root)
            p=root/'CHANGELOG.md';p.write_text(p.read_text(encoding='utf-8').replace('- Current fix.',''),encoding='utf-8')
            with self.assertRaisesRegex(ValueError,'no categorized changes'):packaging.release_notes('1.2.3',root)

if __name__=='__main__':unittest.main()
