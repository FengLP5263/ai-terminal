#!/usr/bin/env python3
"""构建和验证本地 Windows ZIP；不会上传、发布、打标签或改用户数据。

常用命令：
    python scripts/package.py build                              # 构建干净源码的本地 ZIP
    python scripts/package.py verify output/packages/demo.zip    # 回读校验已有 ZIP

本地验收：
    python scripts/package.py build --allow-dirty    # 明确记录未提交状态

Python 3.11+。build 要求 Windows/Rust/C++ SDK；从仓库定位路径，verify 不执行包内程序。"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
import re
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tomllib
import uuid
import zipfile

ROOT=Path(__file__).resolve().parent.parent
NATIVE_REQUIRED={'conpty.dll','OpenConsole.exe','Microsoft-ConPTY-LICENSE.txt','Microsoft-ConPTY-SOURCE.json'}
CHANGELOGS=('CHANGELOG.md','CHANGELOG.en.md')

def release_notes(version,root=ROOT):
    """Require dated, nonempty current-version entries in both languages."""
    entries={};dates=set()
    for name in CHANGELOGS:
        text=(root/name).read_text(encoding='utf-8-sig')
        heads=list(re.finditer(r'^## \['+re.escape(version)+r'\] - (\d{4}-\d{2}-\d{2})\s*$',text,re.M))
        if len(heads)!=1:raise ValueError(f'{name}: expected exactly one release entry for {version}')
        head=heads[0];date=head.group(1)
        datetime.strptime(date,'%Y-%m-%d');dates.add(date)
        following=re.search(r'^## ',text[head.end():],re.M)
        end=head.end()+following.start() if following else len(text)
        section=text[head.start():end].strip()
        active=False;has_change=False
        categories={'新增','改进','修复','安全','弃用','移除','Added','Changed','Fixed','Security','Deprecated','Removed'}
        for line in section.splitlines():
            if line.startswith('### '):active=line[4:].strip() in categories
            elif active and line.startswith('- ') and line[2:].strip():has_change=True
        if not has_change:raise ValueError(f'{name}: current release has no categorized changes')
        entries[name]=section
    if len(dates)!=1:raise ValueError('bilingual release dates differ')
    return entries

def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def source_identity(root=ROOT):
    inputs=[root/'build.rs',root/'Cargo.toml',root/'Cargo.lock',root/'vendor/egui_term/Cargo.toml',root/'vendor/egui-winit/Cargo.toml',root/'vendor/egui_glow/Cargo.toml']+list((root/'src').rglob('*.rs'))+list((root/'vendor/egui_term/src').rglob('*.rs'))+list((root/'vendor/egui-winit/src').rglob('*.rs'))+[p for p in (root/'vendor/egui_glow/src').rglob('*') if p.is_file()]+list((root/'assets/fonts').iterdir())+list((root/'assets/icons').iterdir())+list((root/'assets/branding').iterdir())+[p for p in (root/'assets/windows/conpty').rglob('*') if p.is_file()]
    inputs += [root/name for name in CHANGELOGS]
    inputs += [root/'vendor/winit/Cargo.toml', root/'vendor/winit/build.rs']
    inputs += [p for p in (root/'vendor/winit/src').rglob('*') if p.is_file()]
    digest=hashlib.sha256()
    for item in sorted(inputs,key=lambda p:p.relative_to(root).as_posix()):
        digest.update(item.relative_to(root).as_posix().encode());digest.update(b'\0');digest.update(item.read_bytes());digest.update(b'\0')
    return digest.hexdigest()

def command(args,check=True):
    return subprocess.run(args,cwd=ROOT,capture_output=True,text=True,encoding='utf-8',errors='replace',check=check)

def verify_archive(path):
    path=Path(path)
    with zipfile.ZipFile(path) as archive:
        infos=archive.infolist();names=[i.filename for i in infos]
        if len(names)!=len(set(names)) or len(names)>10000:raise ValueError('duplicate entries or too many files')
        for info in infos:
            name=info.filename
            if '\\' in name or ':' in name or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts or info.is_dir():
                raise ValueError('unsafe archive member: '+name)
            if info.flag_bits&1:raise ValueError('encrypted members are unsupported')
            if (info.external_attr>>16)&0o170000==0o120000:raise ValueError('symbolic links are unsupported')
        if sum(i.file_size for i in infos)>512*1024*1024:raise ValueError('archive exceeds verification budget')
        if 'manifest.json' not in names:raise ValueError('manifest missing')
        manifest=json.loads(archive.read('manifest.json'))
        expected=set(manifest['files'])|{'manifest.json'}
        if expected!=set(names):raise ValueError('archive and manifest member sets differ')
        if not {'ai-terminal.exe','ai-terminal-hook.exe','LICENSE.txt','Hack-LICENSE.txt','egui-term-LICENSE.txt'}.issubset(expected):
            raise ValueError('program/helper/license incomplete')
        if manifest.get('format_version',1)>=2 and not NATIVE_REQUIRED.issubset(expected):raise ValueError('native console runtime/license incomplete')
        for name,record in manifest['files'].items():
            data=archive.read(name)
            if len(data)!=record['bytes'] or hashlib.sha256(data).hexdigest()!=record['sha256']:raise ValueError('checksum mismatch: '+name)
    return {'ok':True,'archive':path.name,'sha256':sha(path),'files':len(names),'version':manifest['version']}

def native_console_assets(folder,root=ROOT):
    source=root/'assets/windows/conpty'
    record=json.loads((source/'SOURCE.json').read_text(encoding='utf-8'))
    if set(record['files'])!={'x64/conpty.dll','x64/OpenConsole.exe','LICENSE.txt'}:raise ValueError('unexpected native console asset set')
    for name,expected in record['files'].items():
        path=source/name
        if path.stat().st_size!=expected['bytes'] or sha(path)!=expected['sha256']:raise ValueError('native console identity mismatch: '+name)
    for name in ('conpty.dll','OpenConsole.exe'):shutil.copy2(source/'x64'/name,folder/name)
    shutil.copy2(source/'LICENSE.txt',folder/'Microsoft-ConPTY-LICENSE.txt')
    shutil.copy2(source/'SOURCE.json',folder/'Microsoft-ConPTY-SOURCE.json')
    return {'package':record['package'],'version':record['version'],'architecture':record['architecture']}

def write_archive(folder,path):
    # Cargo registry license files may retain a reproducible Unix epoch mtime.
    # Clamp ZIP's legacy date range without changing file bytes or their hashes.
    with zipfile.ZipFile(path,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as archive:
        for item in sorted(Path(folder).rglob('*')):
            if item.is_file():archive.write(item,item.relative_to(folder).as_posix())

def dependencies(cargo,folder):
    metadata=json.loads(command(cargo+['metadata','--locked','--offline','--format-version','1','--filter-platform','x86_64-pc-windows-msvc']).stdout)
    inventory=[]
    for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
        base=Path(package['manifest_path']).parent
        found=set()
        if package.get('license_file'):
            item=base/package['license_file']
            if item.is_file() and item.resolve().is_relative_to(base.resolve()):found.add(item)
        for item in base.iterdir():
            if item.is_file() and item.name.upper().startswith(('LICENSE','COPYING','NOTICE')):found.add(item)
        copied=[]
        # The local workspace root has its own bundled LICENSE and separate font/vendor notices.
        if base!=ROOT:
            for item in sorted(found):
                if item.stat().st_size>1024*1024:continue
                target=folder/'licenses'/f"{package['name']}-{package['version']}"/item.name
                target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(item,target)
                copied.append(target.relative_to(folder).as_posix())
        inventory.append({'name':package['name'],'version':package['version'],'license_expression':package.get('license'),'repository':package.get('repository'),'license_files':copied,'license_text_available':bool(found)})
    (folder/'dependencies.json').write_text(json.dumps({'scope':'Cargo locked Windows-target metadata, not a binary-completeness claim','packages':inventory,'license_text_missing':[p['name']+'@'+p['version'] for p in inventory if not p['license_text_available']]},ensure_ascii=False,indent=2),encoding='utf-8')

def icon_notices(folder,root=ROOT):
    source=root/'assets/icons'
    destination=folder/'agent-icons'
    destination.mkdir()
    shutil.copy2(source/'SOURCE.json',destination/'SOURCE.json')
    for license_file in sorted(source.glob('LICENSE-*')):
        shutil.copy2(license_file,destination/license_file.name)

def build(allow_dirty=False):
    if os.name!='nt':raise ValueError('native Windows packaging is unsupported on this OS; use Windows project.ps1, or verify an existing ZIP here')
    host=command(['rustc','-Vv']).stdout
    if 'host: x86_64-pc-windows-msvc' not in host:raise ValueError('packaging currently requires the x86_64-pc-windows-msvc host toolchain')
    git=command(['git','status','--porcelain=v1','--untracked-files=normal']).stdout
    dirty=bool(git.strip())
    if dirty and not allow_dirty:raise ValueError('worktree is dirty; commit first or explicitly use --allow-dirty for local acceptance')
    version=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    release_notes(version)
    cargo=['cargo']
    local_network=ROOT/'.tools/cargo-network.toml'
    if local_network.exists():cargo+=['--config',str(local_network)]
    # Cargo is the build authority; never copy a potentially stale previous preview.
    result=subprocess.run(cargo+['build','--locked','--release','-j','4'],cwd=ROOT)
    if result.returncode:raise RuntimeError('release build failed')
    label=f"ai-terminal-v{version}-windows-x64-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{uuid.uuid4().hex[:6]}"
    folder=ROOT/'output/packages'/label;folder.mkdir(parents=True,exist_ok=False)
    # Cargo metadata is authoritative, including CARGO_TARGET_DIR / build.target-dir.
    target=Path(json.loads(command(cargo+['metadata','--no-deps','--locked','--offline','--format-version','1']).stdout)['target_directory'])/'release'
    for name in ['ai-terminal.exe','ai-terminal-hook.exe']:shutil.copy2(target/name,folder/name)
    for name in CHANGELOGS:shutil.copy2(ROOT/name,folder/name)
    native=native_console_assets(folder)
    icon_notices(folder)
    for source,destination in [('LICENSE','LICENSE.txt'),('vendor/egui_term/LICENSE','egui-term-LICENSE.txt'),('assets/fonts/LICENSE.md','Hack-LICENSE.txt'),('assets/icons/LICENSE-lobe-icons','Lobe-Icons-LICENSE.txt'),('THIRD_PARTY_NOTICES.md','THIRD_PARTY_NOTICES.md')]:shutil.copy2(ROOT/source,folder/destination)
    shutil.copy2(ROOT/'THIRD_PARTY_NOTICES.en.md',folder/'THIRD_PARTY_NOTICES.en.md')
    (folder/'README.txt').write_text('ai-terminal '+version+'\n\n请完整解压。ai-terminal.exe、ai-terminal-hook.exe、conpty.dll 和 OpenConsole.exe 必须放在同一目录。双击 ai-terminal.exe 默认进入 PowerShell；左上角＋可新建 PowerShell、CMD 或 Git Bash，在终端中输入 codex、claude、qodercn、kimi、zcode 或 traecli。\n会话数据在 %LOCALAPPDATA%/ai-terminal；退出只回收本程序拥有的会话进程。\nCodex 默认使用原生状态与共享 agents 总览；旧版没有原生通知时，可显式运行 codex --no-daemon 并在 /hooks 审查兼容 hook。共享后台任务由 Codex 自身管理。Kimi 使用原生任务状态；TRAE 在 /hooks 审查本次任务 hook；社区 ZCode 使用无凭据基础设置快照接入开始／工具／完成，自定义配置时跳过接入。真实模型回合待用户验收，限制见双语更新日志。版本、源码与原生组件身份见 manifest.json；许可证随包携带。\n\nExtract the complete archive. Keep ai-terminal.exe, ai-terminal-hook.exe, conpty.dll and OpenConsole.exe together. Double-click ai-terminal.exe for PowerShell; use + for PowerShell, CMD or Git Bash, then run codex, claude, qodercn, kimi, zcode or traecli.\nSession data: %LOCALAPPDATA%/ai-terminal. Exit closes only owned sessions. Codex uses native lifecycle and the shared agents overview by default. Older CLIs without native events can run codex --no-daemon and review compatibility hooks in /hooks. Codex owns shared background tasks. Kimi uses native task events. Review TRAE session hooks in /hooks. Community ZCode uses a credential-free basic settings snapshot for start/tools/completion and skips injection for custom configuration. Real model-turn acceptance remains with the user; see the bilingual changelog for limitations. See manifest.json and the included licenses for source/runtime identity.\n',encoding='utf-8')
    dependencies(cargo,folder)
    head=command(['git','rev-parse','--verify','HEAD'],check=False)
    manifest={'format_version':2,'version':version,'platform':'windows-x64','git_commit':head.stdout.strip() if head.returncode==0 else None,'dirty':dirty,'rustc':command(['rustc','-V']).stdout.strip(),'source_sha256':source_identity(),'source_scope':'Cargo manifests/lock, build.rs, production Rust, local egui_glow sources/shaders and winit build sources/assets, font/icon/branding/native-console assets','native_console':native,'build':'cargo build --locked --release','published':False,'files':{}}
    manifest['source_scope']+=', bilingual changelog'
    for item in sorted(folder.rglob('*')):
        if item.is_file():manifest['files'][item.relative_to(folder).as_posix()]={'bytes':item.stat().st_size,'sha256':sha(item)}
    (folder/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
    archive=folder.parent/(folder.name+'.zip')
    partial=archive.with_name(archive.name+'.partial')
    write_archive(folder,partial)
    verify_archive(partial)
    os.replace(partial,archive)
    result=verify_archive(archive)
    archive.with_suffix('.sha256').write_text(result['sha256']+'  '+archive.name+'\n',encoding='ascii')
    print(json.dumps(result,ensure_ascii=False,indent=2));print('Artifact:',archive)

def main():
    parser=argparse.ArgumentParser(description=__doc__,formatter_class=argparse.RawDescriptionHelpFormatter)
    sub=parser.add_subparsers(dest='action',required=True)
    builder=sub.add_parser('build');builder.add_argument('--allow-dirty',action='store_true')
    verifier=sub.add_parser('verify');verifier.add_argument('path',type=Path)
    args=parser.parse_args()
    try:
        if args.action=='build':build(args.allow_dirty)
        else:print(json.dumps(verify_archive(args.path if args.path.is_absolute() else ROOT/args.path),ensure_ascii=False,indent=2))
        return 0
    except (ValueError,RuntimeError,OSError,KeyError,zipfile.BadZipFile,subprocess.CalledProcessError) as error:
        print('Packaging failed: '+str(error),file=sys.stderr)
        if isinstance(error,subprocess.CalledProcessError) and error.stderr:print(error.stderr[:1500],file=sys.stderr)
        return 1

if __name__=='__main__':sys.exit(main())
