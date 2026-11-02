import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

spec=importlib.util.spec_from_file_location('local_package',Path(__file__).resolve().parents[1]/'scripts/package.py')
packaging=importlib.util.module_from_spec(spec);spec.loader.exec_module(packaging)

class ArchiveVerification(unittest.TestCase):
    def fixture(self,path,tamper=False,unsafe=False):
        files={name:b'fixture bytes' for name in ['ai-terminal.exe','ai-terminal-hook.exe','LICENSE.txt','Hack-LICENSE.txt','egui-term-LICENSE.txt']}
        manifest={'version':'test','files':{name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in files.items()}}
        with zipfile.ZipFile(path,'w') as archive:
            for name,data in files.items():archive.writestr(name,b'changed bytes' if tamper and name=='ai-terminal.exe' else data)
            archive.writestr('manifest.json',json.dumps(manifest))
            if unsafe:archive.writestr('../outside.txt',b'never extracted')

    def test_valid_archive_then_program_tampering_is_detected(self):
        with tempfile.TemporaryDirectory() as folder:
            good=Path(folder)/'good.zip';bad=Path(folder)/'bad.zip'
            self.fixture(good);self.fixture(bad,tamper=True)
            self.assertTrue(packaging.verify_archive(good)['ok'])
            with self.assertRaisesRegex(ValueError,'checksum mismatch'):packaging.verify_archive(bad)

    def test_source_identity_covers_glow_patch_and_shaders(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            names=['build.rs','Cargo.toml','Cargo.lock','vendor/egui_term/Cargo.toml','vendor/egui-winit/Cargo.toml','vendor/egui_glow/Cargo.toml','vendor/egui_glow/src/painter.rs','vendor/egui_glow/src/shader/fragment.glsl']
            for name in names:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'original')
            for name in ['src','vendor/egui_term/src','vendor/egui-winit/src','assets/fonts','assets/icons','assets/branding']:(root/name).mkdir(parents=True,exist_ok=True)
            original=packaging.source_identity(root)
            for name in names[-3:]:
                path=root/name;path.write_bytes(b'changed')
                self.assertNotEqual(original,packaging.source_identity(root),name)
                path.write_bytes(b'original')

    def test_path_traversal_member_is_rejected_without_extraction(self):
        with tempfile.TemporaryDirectory() as folder:
            nested=Path(folder)/'nested';nested.mkdir();path=nested/'unsafe.zip';self.fixture(path,unsafe=True)
            with self.assertRaisesRegex(ValueError,'unsafe archive member'):packaging.verify_archive(path)
            self.assertFalse((Path(folder)/'outside.txt').exists())

    def test_old_license_timestamp_can_be_packaged_and_verified(self):
        with tempfile.TemporaryDirectory() as folder:
            source=Path(folder)/'source';source.mkdir()
            names=['ai-terminal.exe','ai-terminal-hook.exe','LICENSE.txt','Hack-LICENSE.txt','egui-term-LICENSE.txt']
            records={}
            for name in names:
                path=source/name;path.write_bytes(b'fixture bytes')
                records[name]={'bytes':path.stat().st_size,'sha256':packaging.sha(path)}
            os.utime(source/'LICENSE.txt',(0,0))
            (source/'manifest.json').write_text(json.dumps({'version':'test','files':records}),encoding='utf-8')
            archive=Path(folder)/'archive.zip'
            packaging.write_archive(source,archive)
            self.assertTrue(packaging.verify_archive(archive)['ok'])
            with zipfile.ZipFile(archive) as handle:self.assertEqual(handle.getinfo('LICENSE.txt').date_time[0],1980)

if __name__=='__main__':unittest.main()
