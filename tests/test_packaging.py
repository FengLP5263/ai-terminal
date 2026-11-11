import hashlib
import importlib.util
import json
import os
import shutil
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
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

    def test_packaged_icon_sources_resolve_their_original_license_texts(self):
        with tempfile.TemporaryDirectory() as folder:
            packaging.icon_notices(Path(folder))
            bundled=Path(folder)/'agent-icons'
            source=packaging.ROOT/'assets/icons'
            self.assertEqual((source/'SOURCE.json').read_bytes(),(bundled/'SOURCE.json').read_bytes())
            records=json.loads((bundled/'SOURCE.json').read_text(encoding='utf-8'))['files']
            self.assertEqual(len(records),6)
            for record in records:
                license_file=record.get('license_file')
                if license_file:
                    self.assertEqual((source/license_file).read_bytes(),(bundled/license_file).read_bytes())
                else:
                    self.assertTrue(record['source'].startswith('https://'))
                    self.assertIn('no standalone license',record['license'])

    def test_outline_icons_preserve_original_license_and_reject_changed_svg(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'root';out=Path(folder)/'package';out.mkdir()
            shutil.copytree(packaging.ROOT/'assets/file-icons',root/'assets/file-icons')
            metadata=packaging.file_icon_notices(out,root)
            self.assertEqual(metadata['bundle'],'Tabler Icons')
            self.assertEqual((out/'Tabler-Icons-LICENSE.txt').read_bytes(),(root/'assets/file-icons/LICENSE.txt').read_bytes())
            self.assertEqual((out/'FILE-ICONS-SOURCE.json').read_bytes(),(root/'assets/file-icons/SOURCE.json').read_bytes())
            icon=root/'assets/file-icons/brand-python.svg'
            icon.write_bytes(icon.read_bytes()+b'tampered')
            with self.assertRaisesRegex(ValueError,'file icon identity mismatch'):
                packaging.file_icon_notices(out,root)

    def test_declared_file_icons_require_notices_while_old_archives_remain_valid(self):
        with tempfile.TemporaryDirectory() as folder:
            old=Path(folder)/'old.zip';new=Path(folder)/'new.zip';self.fixture(old)
            self.assertTrue(packaging.verify_archive(old)['ok'])
            with zipfile.ZipFile(old) as source,zipfile.ZipFile(new,'w') as archive:
                for name in source.namelist():
                    data=source.read(name)
                    if name=='manifest.json':
                        manifest=json.loads(data);manifest['file_icons']={'bundle':'Tabler Icons'};data=json.dumps(manifest).encode()
                    archive.writestr(name,data)
            with self.assertRaisesRegex(ValueError,'file icon license/provenance incomplete'):
                packaging.verify_archive(new)

    def test_source_identity_covers_window_backend_build_sources_and_glow_shaders(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            names=['CHANGELOG.md','CHANGELOG.en.md','build.rs','Cargo.toml','Cargo.lock','vendor/egui_term/Cargo.toml','vendor/alacritty_terminal/Cargo.toml','vendor/vte/Cargo.toml','vendor/egui-winit/Cargo.toml','vendor/winit/Cargo.toml','vendor/winit/build.rs','vendor/winit/src/platform_impl/windows/event_loop.rs','vendor/egui_glow/Cargo.toml','vendor/egui_glow/src/painter.rs','vendor/egui_glow/src/shader/fragment.glsl','vendor/alacritty_terminal/src/term/mod.rs','vendor/vte/src/ansi.rs','assets/file-icons/brand-python.svg']
            for name in names:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'original')
            for name in ['src','vendor/egui_term/src','vendor/egui-winit/src','assets/fonts','assets/icons','assets/branding']:(root/name).mkdir(parents=True,exist_ok=True)
            original=packaging.source_identity(root)
            for name in names[:2]+names[-6:]:
                path=root/name;path.write_bytes(b'changed')
                self.assertNotEqual(original,packaging.source_identity(root),name)
                path.write_bytes(b'original')

    def test_path_traversal_member_is_rejected_without_extraction(self):
        with tempfile.TemporaryDirectory() as folder:
            nested=Path(folder)/'nested';nested.mkdir();path=nested/'unsafe.zip';self.fixture(path,unsafe=True)
            with self.assertRaisesRegex(ValueError,'unsafe archive member'):packaging.verify_archive(path)
            self.assertFalse((Path(folder)/'outside.txt').exists())

    def test_font_assets_are_licensed_and_tampering_blocks_packaging(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'root';destination=Path(folder)/'package';destination.mkdir()
            shutil.copytree(packaging.ROOT/'assets/fonts',root/'assets/fonts')
            metadata=packaging.font_notices(destination,root)
            self.assertEqual(metadata['default_latin'],'JetBrains Mono')
            self.assertEqual((destination/'Cascadia-LICENSE.txt').read_bytes(),(root/'assets/fonts/LICENSE-Cascadia.txt').read_bytes())
            self.assertEqual((destination/'FONT-SOURCE.json').read_bytes(),(root/'assets/fonts/SOURCE.json').read_bytes())
            self.assertIn('Codicons-LICENSE.txt',metadata['licenses'])
            self.assertEqual((destination/'Codicons-LICENSE.txt').read_bytes(),(root/'assets/fonts/LICENSE-Codicons.txt').read_bytes())
            self.assertIn('JetBrainsMono-LICENSE.txt',metadata['licenses'])
            self.assertEqual((destination/'JetBrainsMono-LICENSE.txt').read_bytes(),(root/'assets/fonts/LICENSE-JetBrainsMono.txt').read_bytes())
            font=root/'assets/fonts/CascadiaMono-Regular.ttf'
            font.write_bytes(font.read_bytes()+b'tampered')
            with self.assertRaisesRegex(ValueError,'font asset identity mismatch'):
                packaging.font_notices(destination,root)

    def test_declared_fonts_require_provenance_and_license_while_old_archives_remain_valid(self):
        with tempfile.TemporaryDirectory() as folder:
            old=Path(folder)/'old.zip';self.fixture(old)
            self.assertTrue(packaging.verify_archive(old)['ok'])
            new=Path(folder)/'new.zip'
            with zipfile.ZipFile(old) as source,zipfile.ZipFile(new,'w') as archive:
                for name in source.namelist():
                    data=source.read(name)
                    if name=='manifest.json':
                        manifest=json.loads(data);manifest['fonts']={'default_latin':'Cascadia Mono'};data=json.dumps(manifest).encode()
                    archive.writestr(name,data)
            with self.assertRaisesRegex(ValueError,'font license/provenance incomplete'):
                packaging.verify_archive(new)

    def test_each_declared_font_license_is_required_in_the_archive(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'fonts.zip';self.fixture(path)
            with zipfile.ZipFile(path) as archive:
                files={name:archive.read(name) for name in archive.namelist() if name!='manifest.json'}
            files['Cascadia-LICENSE.txt']=b'original license fixture'
            files['FONT-SOURCE.json']=b'{}'
            manifest={'version':'test','fonts':{'licenses':['Cascadia-LICENSE.txt','JetBrainsMono-LICENSE.txt']},
                      'files':{name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in files.items()}}
            with zipfile.ZipFile(path,'w') as archive:
                for name,data in files.items():archive.writestr(name,data)
                archive.writestr('manifest.json',json.dumps(manifest))
            with self.assertRaisesRegex(ValueError,'declared font license missing'):
                packaging.verify_archive(path)

    def test_new_package_requires_native_console_companions_and_licenses(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'incomplete.zip';self.fixture(path)
            with zipfile.ZipFile(path) as original:files={n:original.read(n) for n in original.namelist() if n!='manifest.json'}
            records={name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in files.items()}
            with zipfile.ZipFile(path,'w') as archive:
                for name,data in files.items():archive.writestr(name,data)
                archive.writestr('manifest.json',json.dumps({'format_version':2,'version':'test','files':records}))
            with self.assertRaisesRegex(ValueError,'native console runtime/license incomplete'):packaging.verify_archive(path)
            for name in packaging.NATIVE_REQUIRED:files[name]=b'native fixture'
            with zipfile.ZipFile(path,'w') as archive:
                for name,data in files.items():archive.writestr(name,data)
                archive.writestr('manifest.json',json.dumps({'format_version':2,'version':'test','files':{name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in files.items()}}))
            self.assertTrue(packaging.verify_archive(path)['ok'])

    def test_native_assets_affect_source_identity_and_are_verified_before_copying(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)/'repo';out=Path(folder)/'package';out.mkdir()
            for name in ['src','vendor/egui_term/src','vendor/egui-winit/src','assets/fonts','assets/icons','assets/branding','assets/windows/conpty/x64']:(root/name).mkdir(parents=True,exist_ok=True)
            for name in ['CHANGELOG.md','CHANGELOG.en.md','build.rs','Cargo.toml','Cargo.lock','vendor/egui_term/Cargo.toml','vendor/alacritty_terminal/Cargo.toml','vendor/vte/Cargo.toml','vendor/egui-winit/Cargo.toml','vendor/egui_glow/Cargo.toml','vendor/winit/Cargo.toml','vendor/winit/build.rs']:
                p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b'fixture')
            native=root/'assets/windows/conpty';files={}
            for name in ['x64/conpty.dll','x64/OpenConsole.exe','LICENSE.txt']:
                (native/name).write_bytes(b'fixture');files[name]={'bytes':7,'sha256':hashlib.sha256(b'fixture').hexdigest()}
            (native/'SOURCE.json').write_text(json.dumps({'package':'test','version':'test','architecture':'x86_64','files':files}),encoding='utf-8')
            original=packaging.source_identity(root)
            packaging.native_console_assets(out,root)
            self.assertEqual(b'fixture',(out/'conpty.dll').read_bytes())
            (native/'x64/conpty.dll').write_bytes(b'tampered')
            self.assertNotEqual(original,packaging.source_identity(root))
            with self.assertRaisesRegex(ValueError,'native console identity mismatch'):packaging.native_console_assets(out,root)

    def test_syntax_notices_match_the_locked_bundle_and_reject_tampering(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/'repo';out=Path(directory)/'package';out.mkdir()
            base=root/'assets/syntaxes';base.mkdir(parents=True)
            license_data=b'[{"text":"Original license text"}]'
            (base/'LICENSES.json').write_bytes(license_data)
            record={'bundle':'two-face','version':'1.0.0','cargo_archive_sha256':'archive','files':[{'name':'LICENSES.json','bytes':len(license_data),'sha256':hashlib.sha256(license_data).hexdigest()}]}
            (base/'SOURCE.json').write_text(json.dumps(record),encoding='utf-8')
            lock='[[package]]\nname = "two-face"\nversion = "1.0.0"\nchecksum = "archive"\n'
            (root/'Cargo.lock').write_text(lock,encoding='utf-8')
            result=packaging.syntax_notices(out,root)
            self.assertEqual(result['version'],'1.0.0')
            self.assertEqual((out/'Syntax-LICENSES.json').read_bytes(),license_data)
            (root/'Cargo.lock').write_text(lock.replace('1.0.0','1.0.1'),encoding='utf-8')
            with self.assertRaisesRegex(ValueError,'syntax bundle identity mismatch'):packaging.syntax_sources(root)
            (root/'Cargo.lock').write_text(lock,encoding='utf-8')
            (base/'LICENSES.json').write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError,'syntax license identity mismatch'):packaging.syntax_sources(root)

    def test_native_oniguruma_license_is_copied_with_the_rust_wrapper_license(self):
        with tempfile.TemporaryDirectory() as directory:
            base=Path(directory)/'onig_sys';base.mkdir();(base/'oniguruma').mkdir()
            (base/'LICENSE.md').write_bytes(b'Rust wrapper license')
            (base/'oniguruma/COPYING').write_bytes(b'Original native C license')
            out=Path(directory)/'package';out.mkdir()
            metadata={'packages':[{'name':'onig_sys','version':'test','manifest_path':str(base/'Cargo.toml'),'license':'MIT','repository':'https://example.invalid'}]}
            with patch.object(packaging,'command',return_value=SimpleNamespace(stdout=json.dumps(metadata))):packaging.dependencies(['cargo'],out)
            self.assertEqual((out/'licenses/onig_sys-test/COPYING').read_bytes(),b'Original native C license')
            self.assertEqual((out/'licenses/onig_sys-test/LICENSE.md').read_bytes(),b'Rust wrapper license')

    def test_declared_syntax_bundle_requires_the_embedded_grammar_notices(self):
        with tempfile.TemporaryDirectory() as directory:
            original=Path(directory)/'original.zip';self.fixture(original)
            changed=Path(directory)/'missing-notices.zip'
            with zipfile.ZipFile(original) as source,zipfile.ZipFile(changed,'w') as archive:
                for name in source.namelist():
                    data=source.read(name)
                    if name=='manifest.json':
                        manifest=json.loads(data);manifest['syntaxes']={'bundle':'two-face'};data=json.dumps(manifest).encode()
                    archive.writestr(name,data)
            with self.assertRaisesRegex(ValueError,'syntax license/provenance incomplete'):packaging.verify_archive(changed)

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
