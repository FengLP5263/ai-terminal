#!/usr/bin/env python3
"""Build or verify an application-local Windows installer; never publish it.

Examples:
    python scripts/installer.py build --path output/packages/app.zip
    python scripts/installer.py build --allow-dirty --acceptance
    python scripts/installer.py verify output/installers/build/setup.json
    python scripts/installer.py smoke output/installers/test/setup.json

Python 3.11+, Windows x64 and the pinned NSIS toolchain for build/smoke.
verify is portable and checks the EXE digest; it never executes the installer.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import uuid
import zipfile

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'scripts/installer'
spec = importlib.util.spec_from_file_location('project_package', ROOT / 'scripts/package.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


def tree_identity(root):
    digest = hashlib.sha256()
    for file in sorted(root.rglob('*')):
        if file.is_file():
            if file.is_symlink():
                raise ValueError('toolchain/source symlinks are unsupported')
            digest.update(file.relative_to(root).as_posix().encode('utf-8') + b'\0')
            digest.update(file.read_bytes() + b'\0')
    return digest.hexdigest()


def compiler():
    record = json.loads((SOURCE / 'TOOLCHAIN.json').read_text(encoding='utf-8'))
    configured = os.environ.get('AI_TERMINAL_NSIS')
    path = Path(configured or shutil.which('makensis.exe') or
                ROOT / '.tools/nsis' / ('nsis-' + record['version']) / 'makensis.exe').resolve()
    if not path.is_file():
        raise ValueError('NSIS is missing; see scripts/README.md and set AI_TERMINAL_NSIS')
    if tree_identity(path.parent) != record['tree_sha256']:
        raise ValueError('NSIS toolchain differs from the pinned official archive')
    version = subprocess.check_output([str(path), '/VERSION'], text=True, encoding='utf-8').strip()
    if version != 'v' + record['version']:
        raise ValueError('unexpected NSIS compiler version: ' + version)
    return path, record


def nsis_string(value):
    value = str(value)
    if any(c in value for c in '\0\r\n'):
        raise ValueError('NSIS strings cannot contain control separators')
    return value.replace('$', '$$').replace('"', '$\\"')


def windows_members(names):
    seen = set()
    for name in names:
        parts = PurePosixPath(name).parts
        if not parts or name.startswith('/') or '\\' in name or PurePosixPath(name).as_posix() != name:
            raise ValueError('unsafe Windows member: ' + name)
        for part in parts:
            if (part in ('.', '..') or part[-1:] in ('.', ' ') or
                any(ord(c) < 32 or c in '<>:"|?*' for c in part) or
                re.fullmatch(r'(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?', part)):
                raise ValueError('unsafe Windows member: ' + name)
        if name.casefold() in seen:
            raise ValueError('case-insensitive member collision: ' + name)
        seen.add(name.casefold())
    return sorted(names)


def write_includes(stage, files, config):
    names = windows_members(files)
    directories = {parent.as_posix() for name in names for parent in PurePosixPath(name).parents
                   if parent != PurePosixPath('.')}
    core = [name for name in ['ai-terminal.exe','ai-terminal-hook.exe','conpty.dll','OpenConsole.exe'] if name in names]
    checks = ['!insertmacro CheckFile "' + name + '"' for name in core]
    checks += ['!insertmacro CheckDirectory "$INSTDIR\\' + nsis_string(d.replace('/', '\\')) + '"'
              for d in sorted(directories, key=lambda x: (x.count('/'), x))]
    checks += ['!insertmacro CheckFile "' + nsis_string(n.replace('/', '\\')) + '"' for n in names if n not in core]
    checks += ['!insertmacro CheckFile ".ai-terminal-installed"']
    installs = []
    for name in names:
        path = PurePosixPath(name)
        parent = '' if path.parent == PurePosixPath('.') else '\\' + str(path.parent).replace('/', '\\')
        installs += ['SetOutPath "$INSTDIR' + nsis_string(parent) + '"',
                     'ClearErrors', 'File "' + nsis_string(stage / 'payload' / name) + '"',
                     'IfErrors failed_install']
    deletes = ['Delete "$INSTDIR\\' + nsis_string(n.replace('/', '\\')) + '"' for n in names]
    deletes += ['RMDir "$INSTDIR\\' + nsis_string(d.replace('/', '\\')) + '"'
                for d in sorted(directories, key=lambda x: (x.count('/'), x), reverse=True)]
    for name, lines in [('check-files.nsh', checks), ('install-files.nsh', installs),
                        ('uninstall-files.nsh', deletes)]:
        (stage / name).write_text('\n'.join(lines) + '\n', encoding='utf-8-sig')
    (stage / 'build-config.nsh').write_text('\n'.join(
        '!define ' + key + ' "' + nsis_string(value) + '"' for key, value in config.items()) + '\n',
        encoding='utf-8-sig')


def build(path=None, allow_dirty=False, acceptance=False, acceptance_id=None):
    if os.name != 'nt':
        raise ValueError('native installer build requires Windows')
    installer_dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT))
    if installer_dirty and not allow_dirty:
        raise ValueError('dirty installer source requires --allow-dirty for local acceptance')
    tool, toolchain = compiler()
    if package.sha(SOURCE / 'NSIS-LICENSE.txt') != toolchain['license_sha256']:
        raise ValueError('NSIS original license identity mismatch')
    if acceptance_id and (not acceptance or not re.fullmatch(r'[a-z0-9-]{6,64}', acceptance_id)):
        raise ValueError('acceptance ID requires --acceptance and a simple unique ID')
    if path is None:
        path = package.build(allow_dirty)
    path = Path(path).resolve()
    package.verify_archive(path)
    with zipfile.ZipFile(path) as archive:
        manifest = json.loads(archive.read('manifest.json'))
        names = windows_members(archive.namelist())
        if manifest.get('dirty') and not allow_dirty:
            raise ValueError('dirty preview payload requires --allow-dirty')
        if manifest.get('platform') != 'windows-x64' or manifest.get('format_version') != 2:
            raise ValueError('installer requires a complete Windows x64 format-2 package')
        version = manifest['version']
        match = re.fullmatch(r'(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?', version)
        if not match or any(int(x) > 65535 for x in match.groups()):
            raise ValueError('invalid installer version')
        stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:6]
        label = 'ai-terminal-v' + version + '-windows-x64-' + ('acceptance-' if acceptance else 'setup-') + stamp
        stage = ROOT / 'output/installers' / label
        stage.mkdir(parents=True, exist_ok=False)
        payload = stage / 'payload'
        payload.mkdir()
        for name in names:
            destination = payload / name
            if not destination.resolve().is_relative_to(payload.resolve()):
                raise ValueError('payload destination escapes stage')
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(archive.read(name))
    shutil.copy2(SOURCE / 'NSIS-LICENSE.txt', payload / 'NSIS-LICENSE.txt')
    files = {name: {'bytes': (payload / name).stat().st_size, 'sha256': package.sha(payload / name)}
             for name in windows_members(names + ['NSIS-LICENSE.txt'])}
    acceptance_id = acceptance_id or stamp.lower()
    product_key = 'AI-Terminal-Acceptance-' + acceptance_id if acceptance else 'AI-Terminal'
    folder = 'AI Terminal Acceptance ' + acceptance_id if acceptance else 'AI Terminal'
    uninstall_key = 'Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\' + product_key
    exe = stage / (label + '.exe')
    config = {'PRODUCT_NAME': folder, 'PRODUCT_FOLDER': folder, 'PRODUCT_KEY': product_key,
              'UNINSTALL_KEY': uninstall_key, 'OUTPUT_EXE': exe, 'APP_VERSION': version,
              'NUMERIC_VERSION': '.'.join(match.groups()) + '.0', 'BUILD_ID': stamp,
              'APP_ICON': ROOT / 'assets/branding/ai-terminal.ico',
              'APP_LICENSE': payload / 'LICENSE.txt',
              'INSTALLED_KIB': str((sum(x['bytes'] for x in files.values()) + 1023) // 1024)}
    if acceptance:
        config['ACCEPTANCE'] = '1'
    write_includes(stage, files, config)
    (stage / 'setup.nsi').write_text((SOURCE / 'setup.nsi').read_text(encoding='utf-8-sig'), encoding='utf-8-sig')
    env = os.environ.copy()
    env.pop('NSISDIR', None)
    with (stage / 'compile.log').open('wb') as log:
        result = subprocess.run([str(tool), '/NOCONFIG', '/V3', str(stage / 'setup.nsi')],
                                cwd=stage, env=env, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        print((stage / 'compile.log').read_text(encoding='utf-8', errors='replace')[-3000:], file=sys.stderr)
        raise ValueError('NSIS compilation failed; see ' + str(stage / 'compile.log'))
    digest = package.sha(exe)
    checksum = exe.with_suffix('.sha256')
    checksum.write_text(digest + '  ' + exe.name + '\n', encoding='ascii')
    record = {'format_version': 1, 'version': version, 'platform': 'windows-x64',
              'acceptance': acceptance, 'product_key': product_key, 'product_folder': folder,
              'uninstall_key': uninstall_key, 'registry_scope': 'HKCU (64-bit view)',
              'artifact': {'name': exe.name, 'bytes': exe.stat().st_size, 'sha256': digest},
              'payload_archive': {'name': path.name, 'sha256': package.sha(path)},
              'payload_git_commit': manifest.get('git_commit'), 'payload_dirty': manifest.get('dirty'),
              'payload_source_sha256': manifest.get('source_sha256'),
              'installer_source_sha256': tree_identity(SOURCE),
              'builder_sha256': package.sha(Path(__file__)), 'compiler': toolchain,
              'signed': False, 'published': False, 'files': files,
              'installer_dirty': installer_dirty,
              'data_directory': '%LOCALAPPDATA%/ai-terminal (retained on uninstall)'}
    metadata = exe.with_suffix('.json')
    metadata.write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(verify(metadata), ensure_ascii=False, indent=2))
    print('Installer:', exe)
    print('Manifest:', metadata)
    return metadata


def verify(path):
    path = Path(path).resolve()
    record = json.loads(path.read_text(encoding='utf-8'))
    if record.get('format_version') != 1 or record.get('platform') != 'windows-x64':
        raise ValueError('unsupported installer metadata')
    artifact = record['artifact']
    name = artifact['name']
    if Path(name).name != name or '/' in name or '\\' in name or ':' in name:
        raise ValueError('unsafe installer artifact name')
    windows_members([name])
    if not name.lower().endswith('.exe'):
        raise ValueError('installer artifact must be an EXE')
    exe = path.parent / name
    digest = package.sha(exe)
    if exe.stat().st_size != artifact['bytes'] or digest != artifact['sha256']:
        raise ValueError('installer checksum mismatch')
    if exe.with_suffix('.sha256').read_text(encoding='ascii') != digest + '  ' + name + '\n':
        raise ValueError('installer checksum sidecar mismatch')
    return {'ok': True, 'version': record['version'], 'artifact': name, 'sha256': digest,
            'bytes': exe.stat().st_size, 'acceptance': record['acceptance'],
            'scope': 'EXE digest and sidecar only; no execution or signature verification'}


def smoke(path, previous=None):
    """Execute only acceptance builds, with unique registry/shortcut names and owned dirs."""
    if os.name != 'nt':
        raise ValueError('installer smoke requires Windows')
    import ctypes
    from ctypes import wintypes
    import time
    import winreg
    path = Path(path).resolve()
    verify(path)
    record = json.loads(path.read_text(encoding='utf-8'))
    key = record['uninstall_key']
    if not record['acceptance'] or not record['product_key'].startswith('AI-Terminal-Acceptance-'):
        raise ValueError('smoke refuses production installers')
    expected_key = 'Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\' + record['product_key']
    if key != expected_key or not re.fullmatch(r'AI-Terminal-Acceptance-[a-z0-9-]{6,64}', record['product_key']):
        raise ValueError('unsafe acceptance registry key')
    if not path.is_relative_to((ROOT / 'output').resolve()):
        raise ValueError('native acceptance artifacts must be under output/')
    def registry_version():
        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key, 0, winreg.KEY_READ | winreg.KEY_WOW64_64KEY) as handle:
                return winreg.QueryValueEx(handle, 'DisplayVersion')[0]
        except FileNotFoundError:
            return None
    if registry_version() is not None:
        raise ValueError('acceptance registry key is already in use')
    old_path = Path(previous).resolve() if previous else path
    verify(old_path)
    old = json.loads(old_path.read_text(encoding='utf-8'))
    if not old['acceptance'] or old['product_key'] != record['product_key']:
        raise ValueError('previous installer must use the same isolated acceptance ID')
    case = ROOT / 'output' / ('installer-smoke-' + uuid.uuid4().hex[:10] + '-验收')
    case.mkdir()
    target = case / 'Program Files 测试'
    data = case / 'retained-data'
    data.mkdir()
    fixture_data = b'{"sessions":[],"active":null,"ui":{"terminal_font":"hack"}}\n'
    (data / 'sessions.json').write_bytes(fixture_data)
    log = []
    def execute(exe, args, expected=0, raw=False):
        command = ('"' + str(exe) + '" ' + args) if raw else [str(exe), *args]
        if not raw and args and args[-1].startswith('/D='):
            # NSIS consumes the unquoted remainder after /D=, including spaces.
            # CreateProcess receives this directly; no command shell is involved.
            command = '"' + str(exe) + '" ' + subprocess.list2cmdline(args[:-1]) + ' ' + args[-1]
        result = subprocess.run(command, cwd=case, capture_output=True, timeout=40,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        log.append({'exe':Path(exe).name,'exit_code':result.returncode})
        if result.returncode != expected:
            raise ValueError('installer acceptance exit mismatch: ' + str(result.returncode))
    def assert_files(state):
        for name, value in state['files'].items():
            file = target / name
            if not file.is_file() or package.sha(file) != value['sha256']:
                raise ValueError('installed file mismatch: ' + name)
        if (data / 'sessions.json').read_bytes() != fixture_data:
            raise ValueError('acceptance data was changed')
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR,wintypes.DWORD,wintypes.DWORD,ctypes.c_void_p,wintypes.DWORD,wintypes.DWORD,wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    def lock_program():
        handle = kernel.CreateFileW(str(target / 'ai-terminal.exe'), 0x80000000, 1, None, 3, 0x80, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        return handle
    exe = path.parent / record['artifact']['name']
    checks = {}
    try:
        execute(old_path.parent / old['artifact']['name'], ['/S', '/D=' + str(target)])
        assert_files(old)
        if registry_version() != old['version']:
            raise ValueError('installed version not registered')
        checks['fresh_install'] = True
        (target / 'user-notes.txt').write_bytes(b'keep this unrelated user file')
        handle = lock_program()
        try:
            execute(exe, ['/S', '/D=' + str(target)], expected=3)
        finally:
            kernel.CloseHandle(handle)
        assert_files(old)
        checks['busy_upgrade_rejected_without_overwrite'] = True
        # An ordinary upgrade must reuse the recorded custom installation path.
        execute(exe, ['/S'])
        assert_files(record)
        if registry_version() != record['version']:
            raise ValueError('upgrade did not register new version')
        checks['upgrade'] = True
        checks['upgrade_reuses_registered_directory'] = True
        smoke_dir = case / 'installed-smoke'
        execute(target / 'ai-terminal.exe', ['--smoke-test', '--data-dir', str(smoke_dir)])
        result = json.loads((smoke_dir / 'smoke.json').read_text(encoding='utf-8'))
        if not result['ok'] or not result.get('capture_after_fixture_ready') or not (smoke_dir / 'smoke.png').is_file():
            raise ValueError('installed program smoke did not capture ready fixture')
        checks['installed_six_pty_smoke'] = True
        kernel.OpenProcess.argtypes = [wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE,wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        for session in result['sessions']:
            handle = kernel.OpenProcess(0x100000, False, session['pty_pid'])
            if handle:
                try:
                    if kernel.WaitForSingleObject(handle, 3000) != 0:
                        raise ValueError('owned smoke PTY did not exit')
                finally:
                    kernel.CloseHandle(handle)
        checks['owned_smoke_ptys_exited'] = True
        # Also test a real running manager, rather than attributing a file-handle fixture to it.
        # Each benchmark owns an eight-second lifetime. Use a fresh one per operation
        # so a natural benchmark exit cannot be mistaken for a missing busy-file guard.
        for operation in ['upgrade', 'uninstall']:
            live_dir = case / ('running-manager-' + operation)
            live = subprocess.Popen([str(target / 'ai-terminal.exe'), '--idle-benchmark', '--data-dir', str(live_dir)],
                                    cwd=case, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                    creationflags=subprocess.CREATE_NO_WINDOW)
            try:
                deadline = time.monotonic() + 12
                while not (live_dir / 'ready.json').exists():
                    if live.poll() is not None or time.monotonic() > deadline:
                        raise ValueError('owned running-manager fixture did not become ready')
                    time.sleep(0.1)
                if operation == 'upgrade':
                    execute(exe, ['/S', '/D=' + str(target)], expected=3)
                else:
                    execute(target / 'uninstall.exe', '/S _?=' + str(target), expected=3, raw=True)
                if live.poll() is not None:
                    raise ValueError('busy operation did not preserve the running manager')
                live.wait(timeout=20)
                if live.returncode != 0:
                    raise ValueError('owned running-manager fixture failed')
            finally:
                if live.poll() is None:
                    live.kill()
                    live.wait()
        checks['actual_running_manager_kept_during_upgrade_and_uninstall'] = True
        handle = lock_program()
        try:
            execute(target / 'uninstall.exe', '/S _?=' + str(target), expected=3, raw=True)
        finally:
            kernel.CloseHandle(handle)
        assert_files(record)
        checks['busy_uninstall_rejected_without_deletion'] = True
        execute(target / 'uninstall.exe', ['/S'])
        deadline = time.monotonic() + 30
        while registry_version() is not None or (target / 'uninstall.exe').exists():
            if time.monotonic() > deadline:
                raise ValueError('uninstaller did not finish')
            time.sleep(0.1)
        if any((target / name).exists() for name in record['files']):
            raise ValueError('managed program files remain after uninstall')
        if (target / 'user-notes.txt').read_bytes() != b'keep this unrelated user file':
            raise ValueError('uninstaller removed an unrelated file')
        if (data / 'sessions.json').read_bytes() != fixture_data:
            raise ValueError('uninstaller changed session data')
        checks['uninstall_keeps_data_and_unrelated_files'] = True
        checks['isolated_registry_removed'] = registry_version() is None
        report = {'ok':True,'checks':checks,'actions':log,'directory':str(case),
                  'version':record['version'],'previous_version':old['version'],
                  'fixture_events':True,'model_requests':False}
        (case / 'result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        return report
    except Exception:
        (case / 'result.json').write_text(json.dumps({'ok':False,'checks':checks,'actions':log},indent=2)+'\n',encoding='utf-8')
        # Retain failed artifacts and registration for diagnosis; never clean by name.
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    actions = parser.add_subparsers(dest='action', required=True)
    builder = actions.add_parser('build')
    builder.add_argument('--path', type=Path)
    builder.add_argument('--allow-dirty', action='store_true')
    builder.add_argument('--acceptance', action='store_true')
    builder.add_argument('--acceptance-id')
    verifier = actions.add_parser('verify')
    verifier.add_argument('path', type=Path)
    native = actions.add_parser('smoke')
    native.add_argument('path', type=Path)
    native.add_argument('--previous', type=Path)
    args = parser.parse_args()
    try:
        if args.action == 'build':
            build(args.path, args.allow_dirty, args.acceptance, args.acceptance_id)
        elif args.action == 'smoke':
            print(json.dumps(smoke(args.path, args.previous), ensure_ascii=False, indent=2))
        else:
            print(json.dumps(verify(args.path), ensure_ascii=False, indent=2))
        return 0
    except (ValueError, OSError, KeyError, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        print('Installer failed: ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
