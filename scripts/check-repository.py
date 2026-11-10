#!/usr/bin/env python3
"""检查管理层、提交候选文件与字体身份；不读取忽略的真实配置。

常用命令：
    python scripts/check-repository.py           # 检查仓库候选内容
    python scripts/check-repository.py --help    # 查看帮助

要求 Python 3.11+ 和 Git；从脚本定位仓库。只读检查，不运行模型、不启动 GUI。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import check_bilingual
import package as project_package

ROOT=Path(__file__).resolve().parent.parent
NATIVE_EXECUTABLE='assets/windows/conpty/x64/OpenConsole.exe'
REQUIRED=['AGENTS.md','CLAUDE.md','README.md','CHANGELOG.md','Cargo.toml','Cargo.lock','build.rs','rust-toolchain.toml','.gitignore','.gitattributes','THIRD_PARTY_NOTICES.md','scripts/README.md','scripts/windows/project.ps1','scripts/posix/project.sh','scripts/package.py','scripts/installer.py','scripts/installer/setup.nsi','scripts/installer/TOOLCHAIN.json','scripts/installer/NSIS-LICENSE.txt']
REQUIRED += ['.agents/architecture/'+name+'.md' for name in ['overview','runtime','data-and-assets','integrations']]
REQUIRED += ['.agents/skills/ai-terminal-'+name+'/SKILL.md' for name in ['development','testing','packaging','git-workflow']]

def check():
    errors=[]
    for item in REQUIRED:
        if not (ROOT/item).is_file():errors.append('missing: '+item)
    try:project_package.release_notes(tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version'])
    except (ValueError,OSError,KeyError) as error:errors.append('release notes: '+str(error))
    try:project_package.syntax_sources()
    except (ValueError,OSError,KeyError,StopIteration) as error:errors.append('syntax sources: '+str(error))
    result=subprocess.run(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT,capture_output=True,check=True)
    paths=sorted(set(result.stdout.decode('utf-8').split('\0'))-{''})
    bilingual_errors,bilingual_pairs=check_bilingual.check(paths)
    errors.extend(bilingual_errors)
    if not bilingual_errors: errors.extend(check_bilingual.verify_snapshots(bilingual_pairs))
    blocked={'target','output','.tools','.runtime','runtime','.local','reference','.git','__pycache__'}
    secret=re.compile(r'(?:sk-(?:proj-)?[A-Za-z0-9_-]{30,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)')
    personal=re.compile(r'[A-Za-z]:[\\/]Users[\\/](?!Public(?:[\\/]|$)|<|\$|%)[A-Za-z0-9_.-]+[\\/]')
    text_suffix={'.rs','.md','.toml','.json','.py','.ps1','.sh','.yml','.yaml','.txt','.nsi','.nsh'}
    for name in paths:
        path=ROOT/name
        if not path.is_file():continue
        if path.is_symlink() or not path.resolve().is_relative_to(ROOT):errors.append('external/symlink: '+name);continue
        if any(part in blocked for part in Path(name).parts) or (path.suffix.lower() in {'.exe','.zip','.7z','.db','.sqlite','.pfx','.p12','.pem','.key','.dump'} and name!=NATIVE_EXECUTABLE):
            errors.append('generated/sensitive file: '+name)
        if path.name.startswith('.env') and not path.name.endswith('.example'):errors.append('real environment: '+name)
        if path.suffix in text_suffix or path.name in {'.gitignore','.gitattributes','Cargo.lock'}:
            try:text=path.read_text(encoding='utf-8-sig')
            except UnicodeError:errors.append('not UTF-8: '+name);continue
            if secret.search(text):errors.append('potential credential material: '+name)
            if personal.search(text):errors.append('personal absolute path: '+name)
            if path.suffix=='.ps1' and any(ord(c)>127 for c in text) and not path.read_bytes().startswith(b'\xef\xbb\xbf'):
                errors.append('PowerShell Chinese text needs UTF-8 BOM: '+name)
            # Published crate READMEs can reference files omitted upstream.
            # Validate project-authored vendor provenance, preserve original docs.
            if path.suffix=='.md' and (not name.startswith('vendor/') or path.name.startswith('UPSTREAM')):
                for target in re.findall(r'\]\(([^)]+)\)',text):
                    target=target.strip('<>').split('#',1)[0]
                    if not target or '://' in target or target.startswith(('/', '${', '%')):continue
                    if not (path.parent/target).exists():errors.append('broken document link: '+name+' -> '+target)
    source=json.loads((ROOT/'assets/fonts/SOURCE.json').read_text(encoding='utf-8'))
    installer_toolchain=json.loads((ROOT/'scripts/installer/TOOLCHAIN.json').read_text(encoding='utf-8'))
    if hashlib.sha256((ROOT/'scripts/installer/NSIS-LICENSE.txt').read_bytes()).hexdigest()!=installer_toolchain['license_sha256']:errors.append('installer license identity mismatch')
    for name,expected in source['files'].items():
        data=(ROOT/'assets/fonts'/name).read_bytes()
        if len(data)!=expected['bytes'] or hashlib.sha256(data).hexdigest()!=expected['sha256']:errors.append('font identity mismatch: '+name)
    icon_source=json.loads((ROOT/'assets/icons/SOURCE.json').read_text(encoding='utf-8'))
    for expected in icon_source['files']:
        data=(ROOT/'assets/icons'/expected['name']).read_bytes()
        if len(data)!=expected['bytes'] or hashlib.sha256(data).hexdigest()!=expected['sha256']:errors.append('icon identity mismatch: '+expected['name'])
    branding=json.loads((ROOT/'assets/branding/SOURCE.json').read_text(encoding='utf-8'))
    for name,expected in branding['files'].items():
        data=(ROOT/'assets/branding'/name).read_bytes()
        if len(data)!=expected['bytes'] or hashlib.sha256(data).hexdigest()!=expected['sha256']:errors.append('branding identity mismatch: '+name)
    if (ROOT/'assets/branding/ai-terminal.svg').read_bytes() != (ROOT/branding['origin']).read_bytes():errors.append('approved B artwork mismatch')
    native=json.loads((ROOT/'assets/windows/conpty/SOURCE.json').read_text(encoding='utf-8'))
    if set(native['files'])!={'x64/conpty.dll','x64/OpenConsole.exe','LICENSE.txt'}:errors.append('unexpected native console asset set')
    for name,expected in native['files'].items():
        data=(ROOT/'assets/windows/conpty'/name).read_bytes()
        if len(data)!=expected['bytes'] or hashlib.sha256(data).hexdigest()!=expected['sha256']:errors.append('native console identity mismatch: '+name)
    ignore=(ROOT/'.gitignore').read_text(encoding='utf-8')
    for rule in ['/target/','/output/','/.tools/','.env','*.zip','*.pem','sessions.json']:
        if rule not in ignore.splitlines():errors.append('missing ignore: '+rule)
    for name in ['dev.ps1','smoke.ps1','measure-idle.ps1','verify-claude.py']:
        if not (ROOT/'scripts'/name).read_text(encoding='utf-8-sig').startswith(('<#','"""','#')):errors.append('missing usage header: '+name)
    return paths,errors,bilingual_pairs

def main():
    argparse.ArgumentParser(description=__doc__,formatter_class=argparse.RawDescriptionHelpFormatter).parse_args()
    paths,errors,bilingual_pairs=check()
    report={'ok':not errors,'candidate_files':len(paths),'font_identities_checked':len(json.loads((ROOT/'assets/fonts/SOURCE.json').read_text(encoding='utf-8'))['files']),'icon_identities_checked':len(json.loads((ROOT/'assets/icons/SOURCE.json').read_text(encoding='utf-8'))['files']),'branding_identities_checked':3,'native_console_identities_checked':3,'bilingual_document_pairs_checked':len(bilingual_pairs),'errors':errors}
    print(json.dumps(report,ensure_ascii=False,indent=2))
    return 1 if errors else 0

if __name__=='__main__':sys.exit(main())
