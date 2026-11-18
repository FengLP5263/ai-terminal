# 原生管理 CLI

简体中文 | [English](cli.en.md)

从 0.14.0 开始，ai-terminal-cli.exe 允许人和 Agent 管理运行中的 AI Terminal。它与主程序、hook helper 一同交付；0.13.0 及以前的管理器不提供此接口。完整 Windows 发行包包含 CLI，可另行下载独立 Agent 技能包。

## 快速使用

先打开对应版本的 ai-terminal.exe。在它创建的 Shell 或 Agent 内：

```powershell
ai-terminal-cli instance info --json
ai-terminal-cli session list --json
ai-terminal-cli session rename current --name '主协调' --json
```

创建并显式启动一个成员：

```powershell
$r = ai-terminal-cli session create --name '前端 ✨' --directory $PWD.Path --color blue --json | ConvertFrom-Json
if (-not $r.ok) { throw $r.error.message }
ai-terminal-cli session run $r.result.results[0].id --agent codex --json
```

普通创建不启动进程；session start 启动 Shell，session run 在没有运行终端的会话中启动指定 Agent。管理器重启后不会自动再次启动 Agent。登录、信任和模型任务沿用 CLI 自己的交互；管理器不自动提交提示词或确认授权。

外部调用用完整程序路径，必要时加 --data-dir 指定管理器的元数据目录。该路径不是项目目录。不写用户级 PATH；受管子进程临时获得命令路径和 AI_TERMINAL_CLI。current 始终是调用者会话，而非界面选中项；外部调用先查询完整 UUID。

## 命令与团队编排

- [会话命令参考](../skills/ai-terminal/references/sessions.md)：查询、创建、改名、配色、启动、切换、关闭。
- [分组与排序](../skills/ai-terminal/references/groups.md)：同组代表由全局会话顺序决定。
- [团队计划](../skills/ai-terminal/references/teams.md)：一次创建成员、引用当前主会话、建组、排序，支持 dry-run 和 revision 冲突检查。
- [协议与排错](../skills/ai-terminal/references/protocol.md)：JSON、退出码、请求回执、多实例与限额。
- [安装和使用 Agent Skill](skills.md)。

批量元数据先完整校验，再原子保存并同步界面；失败不留下半套队伍。已有运行终端保持原进程。启动与关闭是单独命令，运行会话关闭需要 --force。解散组保留成员。本版提供显式终端读取／输入；仅在任务需要时调用，不保存正文，不代理模型。

## 原生操作覆盖

0.14.0 将 CLI 扩展到现有工作区的主要原生操作。先 `instance capabilities` 确认 native_controls，再执行：

```powershell
ai-terminal-cli floating set --enabled true --transparency 50
ai-terminal-cli settings set --language zh_cn --scale 114
ai-terminal-cli panel open files
ai-terminal-cli file open --path README.md
```

- [设置与窗口](../skills/ai-terminal/references/settings.md)：全部可见设置、浮窗开关／透明度／边框／缩放／位置，侧栏，窗口状态和截图。
- [文件／Git／媒体](../skills/ai-terminal/references/viewing.md)：惰性文件树、分页阅读、Markdown 渲染、图片缩放、音视频播放及只读 Git。
- [终端交互](../skills/ai-terminal/references/terminal.md)：显式视口读取、文字／按键／本地滚动、停止／重启和未读确认。

这些接口使用现有原生实现；设置先校验再保存，立即生效。文件／Git／媒体异步排队，用 loading／error 和实际结果核实。CLI 不接管系统文件选择器、安装卸载、其他应用或用户的剪贴板，也不提供界面本就没有的 Git 写操作。命令完整列表见 `--help`。

## 从源码验证

```powershell
.\scripts\windows\project.ps1 build
.\target\debug\ai-terminal-cli.exe --help
```

构建生成三个独立入口。开发与测试命令见[脚本目录](../scripts/README.md)，架构见[运行与配置](../.agents/architecture/runtime.md)。
