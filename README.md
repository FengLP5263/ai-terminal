<div align="center">

<img src="assets/branding/ai-terminal.png" width="96" alt="AI Terminal 图标">

# AI Terminal

**给多个 AI 会话，一个清爽的工作空间。**

在熟悉的终端里使用 Codex、Claude Code、Qoder CN、Kimi Code、ZCode 和 TRAE CLI，随时查看会话状态，需要时一键返回。

[![GitHub Release](https://img.shields.io/github/v/release/FengLP5263/ai-terminal?label=release)](https://github.com/FengLP5263/ai-terminal/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D4)](#平台支持)
[![License](https://img.shields.io/badge/license-MIT-5c8a66)](LICENSE)

[下载使用](#快速开始) · [功能介绍](#为多会话而设计) · [支持的 Agent](#支持的-agent) · [更新日志](CHANGELOG.md) · [常见问题](#常见问题) · [参与贡献](CONTRIBUTING.md)

简体中文 | [English](README.en.md)

</div>

## 为什么做 AI Terminal？

AI 任务可能运行很久。打开多个窗口后，来回确认哪个会话还在工作、哪个需要处理，很容易打断手头的事。AI Terminal 把终端会话和状态集中起来，让你继续使用自己熟悉的 Shell 和 Agent，需要时再回到对应会话。

它是一个 Windows 原生终端工作区，不内置模型服务，也不要求你改变现有 CLI 的登录、权限或工作习惯。

## 看看界面

![AI Terminal 多会话工作区：侧栏显示多个会话，右侧是终端](assets/readme/workspace.png)

开启浮窗后，可以在其他工作窗口上方查看会话头像和状态，点击头像回到对应会话：

![AI Terminal 浮窗示意](assets/readme/floating.png)

> 截图使用 0.4.8-preview.1 的真实界面、合成会话内容、示例目录和模拟状态，不包含真实用户名、工作目录或私人对话。点击图片可查看原始分辨率。

## 为多会话而设计

| 功能 | 你可以做什么 |
| --- | --- |
| **真正的终端** | 新建 PowerShell、CMD 或 Git Bash，在里面直接运行已安装的命令行工具。 |
| **Agent 会话识别** | 启动 Codex、Claude Code、Qoder CN、Kimi Code、ZCode 或 TRAE CLI 后显示对应头像；其他命令仍可作为普通终端使用。 |
| **任务状态** | 在侧栏和浮窗中查看支持的 Agent 的进行中、需要处理、完成或失败状态。状态来自支持的生命周期通知和进程退出，并非对所有 CLI 的通用推断。 |
| **置顶浮窗** | 只显示会话头像和状态，一次最多显示 10 个；拖动头像区域可查看其余会话，点击头像回到会话。 |
| **整理自己的工作区** | 拖动会话排序、重命名、设置颜色、收起侧栏；使用 Ctrl + 滚轮调整整个界面大小。 |
| **熟悉的终端体验** | 支持中文、ANSI 颜色和系统彩色 emoji；Codex／Claude Code／Qoder CN CLI 可使用剪贴板图片，具体交互由对应 CLI 处理。 |
| **中英双语** | 自动跟随系统语言，也可在设置中手动选择简体中文或 English。 |

## 支持的 Agent

| 图标 | Agent | Windows 启动命令 | 头像与启动／退出识别 | 自动任务状态 |
| --- | --- | --- | --- | --- |
| <img src="assets/readme/agent-codex.png" width="32" height="32" alt="Codex CLI"> | [Codex CLI](https://github.com/openai/codex) | `codex` | 支持 | 原生生命周期；旧版兼容 hook |
| <img src="assets/readme/agent-claude.png" width="32" height="32" alt="Claude Code"> | [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) | `claude` | 支持 | 会话级 hook |
| <img src="assets/readme/agent-qoder-cn.png" width="32" height="32" alt="Qoder CN CLI"> | [Qoder CN CLI](https://docs.qoder.cn/en/cli/installation) | `qodercn` / `qoderclicn` | 支持 | 会话级 hook |
| <img src="assets/readme/agent-kimi.png" width="32" height="32" alt="Kimi Code CLI"> | [Kimi Code CLI](https://www.kimi.com/code/docs/) | `kimi` | 支持 | 原生生命周期（Kimi Code 2.5.0） |
| <img src="assets/readme/agent-zcode.png" width="32" height="32" alt="ZCode"> | [ZCode](https://zcode.z.ai/) | `zcode` | 支持 | 有限会话 hook；提交后状态待确认，明确完成后显示完成 |
| <img src="assets/readme/agent-trae.png" width="32" height="32" alt="TRAE CLI"> | [TRAE CLI](https://docs.trae.cn/cli_about-trae-code-cli-2) | `traecli` | 支持 | 会话级 hook；需在 CLI 内审查信任 |

状态只来自明确生命周期、hook 与进程退出；未收到事件时保持“等待任务通知”，不根据终端文字或静默猜测完成。新增集成在 0.4.8 预览版提供，真实模型回合等待用户验收；稳定版 0.4.7 的三款新增 CLI 仍仅识别头像与启停。

- Kimi：受管 `kimi` 入口为本次进程启用原生 OSC 7501，区分工作、授权／问题／登录等待、完成与失败；忽略子 Agent 记录和消息正文。显式 `PI_PROGRAM_STATUS=0` 保持关闭。
- TRAE：当前企业版 CLI 需要 TRAE 企业旗舰版账号及其 PAT，个人订阅或第三方模型 API Key 不能替代。参见[官方 CLI 说明](https://docs.trae.cn/cli_about-trae-code-cli-2)。本次 `-c` 参数提供生命周期 hook，不改用户／项目配置；启动后在 `/hooks` 中审查信任。显式后续配置仍由 CLI 解释，远端服务不承诺本机 hook 可用。
- ZCode：仅针对社区 zcode-app-cli 3.17.2-44／运行时 0.17.1。支持开始、工具、问题工具与完成；当前 hook 接口缺少可靠的授权等待、任务失败及中断事件，提交后显示“状态待确认”（灰色空心标记），不持续显示进行中；收到明确完成通知才显示完成，明确失败或非正常退出才显示失败。通过 `ZCODE_CLI_SETTINGS_FILE` 选择本次进程的基础设置快照，仅允许无凭据的基础选项；已有 hook、非空自定义列表、MCP／插件定义、未知设置或显式该环境变量时保留原配置并跳过接入。不会打开 provider／凭据文件。快照内的设置修改仅对本次调用有效；原配置有自定义内容时，状态连接说明会提示该边界。

已有 CLI 需退出后重新启动，才能应用本次会话集成。登录、目录与 hook 信任由你在 Agent 内确认。

ZCode 本次实测的是社区发行 [zcode-app-cli](https://github.com/kingsword09/zcode-cli) 3.17.2-44（运行时 0.17.1），不作为 Z.ai 官方独立 CLI 验收。管理器转发已安装的公开 `zcode` 入口，不捆绑或自动安装任何 Agent。图标复用正式界面组件，以深色底衬保证在 README 的浅色／深色页面中均可辨识；品牌来源见[第三方说明](THIRD_PARTY_NOTICES.md)。

## 平台支持

| 平台 | 状态 |
| --- | --- |
| Windows x64 | 支持 |
| macOS / Linux | 暂不支持原生 GUI |

## 快速开始

### 1. 下载并启动

从 [GitHub Releases](https://github.com/FengLP5263/ai-terminal/releases/latest) 下载 Windows x64 ZIP，**完整解压**后运行 `ai-terminal.exe`。0.4.3 起请将 `ai-terminal.exe`、`ai-terminal-hook.exe`、`conpty.dll` 和 `OpenConsole.exe` 保留在同一目录。运行发行包不需要安装 Rust、Python 或 Electron。

### 2. 创建终端，启动 Agent

点击左上角 **＋** 选择 Shell，或按 <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>N</kbd> 新建 PowerShell。在终端中进入项目目录，输入已安装的 CLI 命令（将示例路径替换为自己的路径）：

```powershell
cd 'C:\path\to\your-project'
codex
# 或运行 claude / qodercn / kimi / zcode / traecli
```

请事先自行安装并配置 [Codex CLI](https://github.com/openai/codex)、[Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview)、[Qoder CN CLI](https://docs.qoder.cn/cli/installation)、Kimi Code、ZCode 或 TRAE CLI。AI Terminal 沿用 CLI 自己的登录、模型、信任和权限设置。

### 3. 按自己的方式管理

在左上角齿轮中设置语言和浮窗；悬停会话头像可查看信息，点击即可切换。浮窗可拖动位置，并可在设置中关闭。按住 <kbd>Ctrl</kbd> 滚动鼠标滚轮可缩放整个界面。

在受管终端里交互式启动 `powershell`、`cmd` 或 `bash` 时，头像会跟随当前 Shell；`exit` 逐层恢复。启动 Codex／Claude／Qoder CN／Kimi Code／ZCode／TRAE CLI 时优先显示 Agent 头像，退出后恢复当前 Shell。单次脚本命令和 Agent 内部的工具命令不会因此改变 Shell 头像。

## 常见问题

### 已经安装 CLI，为什么提示找不到命令？

请确认 Windows 可从当前或用户／系统 `PATH` 找到对应的 `.exe`、`.cmd` 或 `.bat` 启动器。仅安装在 WSL 中的程序或 PowerShell 别名不属于受支持的 Windows 入口。

### 没有安装 Git Bash 也能使用吗？

可以直接使用 PowerShell 或 CMD。Git Bash 是可选依赖；选择后若未找到安装，界面会提供切换到可用 Shell 的按钮。便携版 Git 需将 `cmd` 或 `bin` 目录加入 `PATH`。Git Bash 会加载其系统／用户启动配置，保留提示符、别名和补全。

### 为什么 Agent 没有显示状态？

状态依赖 CLI 明确的生命周期通知。Codex 默认使用原生状态并保留共享 agents 总览；旧版可显式运行 `codex --no-daemon` 使用兼容 hook。Claude Code 和 Qoder CN 通过会话级 hook 接入。稳定版 0.4.7 的 Kimi Code、ZCode 与 TRAE CLI 仅识别头像和启停；0.4.8 预览的接入范围见上方表格。未接收到通知时显示“等待任务通知”，ZCode 收到开始事件后显示“状态待确认”。管理器不替你信任 hook，也不修改用户或项目的 hook 配置。

### 重启后可以继续之前的对话吗？

程序恢复的是会话列表和界面设置，不会恢复原先运行的进程或终端屏幕内容。要继续 AI 对话，请使用对应 CLI 自带的会话恢复功能。退出只回收本程序拥有的终端及 helper；Codex 共享后台中的任务可能继续运行，由 Codex 自身管理。

### 怎么查看终端历史和粘贴图片？

普通滚轮用于查看历史；全屏应用接管鼠标滚轮时可用 <kbd>Shift</kbd> + 滚轮查看本地历史。识别到 Codex／Claude Code／Qoder CN／Kimi Code／ZCode／TRAE CLI 时，可通过 <kbd>Ctrl</kbd> + <kbd>V</kbd> 或 <kbd>Shift</kbd> + <kbd>Insert</kbd> 将剪贴板图片交给 CLI 处理。

### 首次运行会在桌面创建快捷方式吗？

使用默认数据目录首次启动时，程序会尝试在桌面创建 `ai-terminal.lnk`，便于关闭后再次打开；只尝试一次，创建失败不影响启动。使用 `--data-dir` 指定非默认数据目录、隔离 smoke 或工作区读取失败时不会创建，手动删除后也不会自动重建。快捷方式只指向当前 `ai-terminal.exe`，不含命令参数或凭据。

### 会保存我的对话或 API Key 吗？

默认数据位于 `%LOCALAPPDATA%/ai-terminal`，保存会话元数据、界面偏好及临时 hook 配置；不保存提示词、对话正文或 API Key。各 CLI 自己的数据和配置由其各自管理。

## 从源码开发

需要 Windows x64、Rust stable MSVC、Visual Studio C++ Build Tools、包含 `rc.exe` 的 Windows SDK 和 Python 3.11+。

```powershell
git clone https://github.com/FengLP5263/ai-terminal.git
cd ai-terminal
rustup component add rustfmt clippy
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`ci` 包含治理、格式、Clippy、Rust 和 Python 检查，不调用模型 API。发行包的源码摘要覆盖本地图形补丁、着色器及窗口后端补丁。开发、测试和打包的详细说明见[文档目录](docs/README.md)及[开发脚本](scripts/README.md)。

## 一起完善

欢迎反馈问题、分享想法、改进终端体验和文档！提交之前，请先阅读[贡献指南](CONTRIBUTING.md)和[协作规范](AGENTS.md)。反馈问题时请勿包含凭据或私人对话。

[提交 Issue](https://github.com/FengLP5263/ai-terminal/issues) · [查看贡献指南](CONTRIBUTING.md) · [浏览文档](docs/README.md)

## 许可证

项目代码使用 [MIT License](LICENSE)。第三方组件保留各自的许可证，详见[第三方声明](THIRD_PARTY_NOTICES.md)。

Agent 品牌图标的来源与许可按文件分别记录，详见[第三方说明](THIRD_PARTY_NOTICES.md)；ZIP 的 agent-icons/ 附带来源清单和可获得的原始许可。

## 0.4.7 更新

新增 Kimi Code、ZCode 和 TRAE CLI 的头像与启停识别、首次桌面快捷方式；改进彩色动画 TUI 的字形缓存并修复旧图集释放。支持范围和六种 Agent 图标见上方表格。完整功能、升级说明和验证范围见[中英文更新日志](CHANGELOG.md)；每次发行的 ZIP 均附带两份日志。
