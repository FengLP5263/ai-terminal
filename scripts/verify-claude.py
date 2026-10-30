"""显式真实 Claude 核查：沿用用户接入，禁用工具，单次预算上限 0.15 美元。

常用命令：
    python scripts/verify-claude.py --help                                     # 只读帮助，不调用模型
    python scripts/verify-claude.py --profile release --allow-model-request    # 经明确授权后执行真实模型回合

参数：--profile debug/release（默认 release），--cli 可选路径，必须显式 --allow-model-request。
从脚本定位仓库；helper 与 CLI 必须已安装。证据写 output/，不记录密钥或正文转录。
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import threading
import time
import uuid

parser=argparse.ArgumentParser()
parser.add_argument('--profile',choices=['debug','release'],default='release')
parser.add_argument('--cli',type=Path,help='optional installed Claude executable')
parser.add_argument('--allow-model-request',action='store_true',help='explicitly allow one potentially billed model request')
args=parser.parse_args()
if not args.allow_model_request:parser.error('explicit --allow-model-request is required; this command can incur model charges')
if os.name!='nt':parser.error('this native helper verification currently requires Windows')
root=Path(__file__).resolve().parent.parent
helper=root/'target'/args.profile/'ai-terminal-hook.exe'
candidates=[args.cli] if args.cli else [Path(shutil.which('claude.exe') or '__missing__'),Path.home()/'.local/bin/claude.exe',Path(os.environ.get('APPDATA',''))/'npm/node_modules/@anthropic-ai/claude-code/bin/claude.exe']
cli=next((path for path in candidates if path.is_file()),Path('__missing__'))
if not cli.is_file() or not helper.is_file():
    raise SystemExit('Build ai-terminal and install Claude Code first.')
out=root/'output'/('live-claude-'+time.strftime('%Y%m%d-%H%M%S'))
out.mkdir(parents=True,exist_ok=False)
events=[]
token=uuid.uuid4().hex+uuid.uuid4().hex
listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen();listener.settimeout(.2)
endpoint='127.0.0.1:'+str(listener.getsockname()[1])
finished=threading.Event()
def serve():
    while not finished.is_set():
        try:
            connection,_=listener.accept()
        except socket.timeout:
            continue
        except OSError:
            break
        with connection:
            connection.settimeout(.5)
            try:
                frame=connection.makefile('rb').readline(8193)
                envelope=json.loads(frame)
                if envelope.get('token')==token:
                    event=envelope['event']
                    events.append({key:event.get(key) for key in ['name','session_id','turn_id','local_id','generation']})
                    connection.sendall(b'OK\n')
            except (ValueError,KeyError,OSError):
                pass
threading.Thread(target=serve,daemon=True).start()
settings={'hooks':{name:[{'hooks':[{'type':'command','command':str(helper),'args':['--hook'],'timeout':2}]}] for name in ['SessionStart','UserPromptSubmit','PreToolUse','PermissionRequest','PostToolUse','Stop','SessionEnd']}}
(out/'hooks.json').write_text(json.dumps(settings),encoding='utf-8')
(out/'mcp.json').write_text('{"mcpServers":{}}',encoding='utf-8')
env=os.environ.copy();env.pop('CLAUDECODE',None)
env.update(AI_TERMINAL_ENDPOINT=endpoint,AI_TERMINAL_TOKEN=token,AI_TERMINAL_SESSION='live-verification',AI_TERMINAL_GENERATION='live-generation')
command=[str(cli),'--print','--output-format','stream-json','--verbose','--tools','','--strict-mcp-config','--mcp-config',str(out/'mcp.json'),'--settings',str(out/'hooks.json'),'--setting-sources','user','--effort','low','--max-turns','1','--max-budget-usd','0.15','Reply exactly AI_TERMINAL_LIVE_OK. Do not use tools.']
started=time.monotonic()
try:
    result=subprocess.run(command,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=90)
finally:
    finished.set();listener.close()
texts=[]
for line in result.stdout.decode('utf-8',errors='replace').splitlines():
    try:item=json.loads(line)
    except ValueError:continue
    if item.get('type')=='assistant':
        texts.extend(block.get('text','') for block in item.get('message',{}).get('content',[]) if block.get('type')=='text')
names=[event['name'] for event in events]
ok=result.returncode==0 and any('AI_TERMINAL_LIVE_OK' in text for text in texts) and all(name in names for name in ['SessionStart','UserPromptSubmit','Stop'])
report={'ok':ok,'real_model_call':True,'tools_disabled':True,'user_settings_changed':False,'hook_trust_bypassed':False,'returncode':result.returncode,'elapsed_seconds':round(time.monotonic()-started,3),'events':events,'expected_text_received':any('AI_TERMINAL_LIVE_OK' in text for text in texts)}
if not ok:
    error=result.stderr.decode('utf-8',errors='replace')[:2000]
    error=re.sub(r'sk-[A-Za-z0-9_-]+','[REDACTED]',error)
    error=re.sub(r'[A-Fa-f0-9]{32,}','[REDACTED]',error)
    report['diagnostic']=error
(out/'result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({key:value for key,value in report.items() if key!='events'},ensure_ascii=False,indent=2))
print('Evidence:',out)
raise SystemExit(0 if ok else 1)
