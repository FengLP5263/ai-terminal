#!/bin/sh
# ai-terminal 成对 POSIX 入口；原生 GUI 当前仅支持 Windows。
#
# 常用检查：
#    sh scripts/posix/project.sh governance                                       # 可移植静态治理检查
#    sh scripts/posix/project.sh verify-package -Path output/packages/demo.zip    # 跨平台只读 ZIP 校验
#    sh scripts/posix/project.sh check                                            # Git Bash 委托 Windows；macOS/Linux 明确不支持原生检查
#    sh scripts/posix/project.sh test                                             # Git Bash 委托 Windows 原生回归
#    sh scripts/posix/project.sh package                                          # Git Bash 委托本地 Windows 打包
#    sh scripts/posix/project.sh installer -AllowDirty                             # Git Bash 委托 Windows 安装版构建
#    sh scripts/posix/project.sh verify-installer -Path output/installers/setup.json # 跨平台只读 EXE 摘要校验

# 参数：动作与 project.ps1 相同；Windows 参数原样传递（如 -Release、-AllowDirty）。
# verify-package 使用 -Path；路径相对仓库根目录。默认 governance；--help 只读帮助。
# 要求：Python 3.11+。Git Bash 原生动作需要 powershell.exe；不安装依赖、不上传。
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
action=${1:-governance}
if [ "$#" -gt 0 ]; then shift; fi
run_python() {
    if [ -n "${AI_TERMINAL_PYTHON:-}" ]; then "$AI_TERMINAL_PYTHON" -X utf8 "$@"
    elif [ "$(uname -s | cut -c1-5)" = MINGW ] && command -v python >/dev/null 2>&1; then python -X utf8 "$@"
    elif command -v python3 >/dev/null 2>&1; then python3 -X utf8 "$@"
    elif command -v python >/dev/null 2>&1; then python -X utf8 "$@"
    else echo 'Python 3.11+ required; set AI_TERMINAL_PYTHON.' >&2; return 2; fi
}
cd "$repo_root"
case "$action" in
    --help|-h) sed -n '2,/^set -eu/p' "$0" | sed '$d'; exit 0 ;;
    governance) [ "$#" -eq 0 ] || { echo 'governance takes no arguments' >&2; exit 2; };run_python scripts/check-repository.py;exit $? ;;
    verify-package) [ "$#" -eq 2 ] && [ "$1" = '-Path' ] || { echo 'verify-package -Path ZIP required' >&2;exit 2; };run_python scripts/package.py verify "$2";exit $? ;;
    verify-installer) [ "$#" -eq 2 ] && [ "$1" = '-Path' ] || { echo 'verify-installer -Path JSON required' >&2;exit 2; };run_python scripts/installer.py verify "$2";exit $? ;;
esac
case "$action" in format-check|check|test|ci|build|run|release|smoke|measure-idle|package|installer|installer-smoke|verify-claude) : ;;
    *) echo "Unknown action: $action" >&2;exit 2;; esac
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        command -v powershell.exe >/dev/null 2>&1 || { echo 'Windows PowerShell required' >&2;exit 2; }
        entry=$(cygpath -w "$repo_root/scripts/windows/project.ps1")
        exec powershell.exe -NoProfile -File "$entry" -Action "$action" "$@"
        ;;
    *)
        if [ "$action" = format-check ] && [ "$#" -eq 0 ]; then exec cargo fmt --all -- --check; fi
        echo "Native action '$action' is unsupported on macOS/Linux: this project currently requires Windows ConPTY and its GPU build. Use Windows project.ps1; governance, verify-package and verify-installer are available here." >&2
        exit 2
        ;;
esac
