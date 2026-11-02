# ai-terminal

简体中文 | [English](README.en.md)

一个用于管理多个 AI 编程会话的 Windows 原生终端。将 Codex、Claude Code 和普通 Shell 集中在一个窗口中，快速切换并查看任务状态。

**版本：0.4.0 · Windows x64 · MIT** · [下载发行版](https://github.com/FengLP5263/ai-terminal/releases) · [文档](docs/README.md) · [参与贡献](CONTRIBUTING.md)

## 功能

- **终端会话**：新建 PowerShell、CMD 或 Git Bash；在终端中输入 `codex` 或 `claude` 启动已安装的 CLI，自动显示对应头像。
- **任务状态**：通过支持的生命周期通知和进程退出显示进行中、等待处理、完成或失败；不会根据输出静默推测完成。绿点表示本轮结束，不代表项目已验收。
- **会话管理**：拖动排序、重命名、设置颜色、调整或收起侧栏；界面整体支持 Ctrl＋滚轮、Ctrl＋加减缩放，Ctrl＋0 恢复。
- **设置与浮窗**：可跟随系统语言或选择简体中文、English；可开启置顶头像栏，最多同时显示 10 个会话，拖动头像区查看更多。点击头像返回原会话，浮窗跟随界面缩放，并在缩放或会话数量变化时水平居中。
- **原生显示**：采用 Rust、egui、Alacritty 和 Windows ConPTY，支持中文、ANSI 颜色和系统彩色 emoji，不包含浏览器运行时。Codex 使用原生 TUI；支持其全屏滚动和底部输入区。

## 安装与使用

从 [GitHub Releases](https://github.com/FengLP5263/ai-terminal/releases) 下载 Windows x64 ZIP，完整解压后运行 `ai-terminal.exe`；请将 `ai-terminal-hook.exe` 保留在同一目录。运行发行包不需要安装 Rust、Python 或 Electron。先自行安装和配置要使用的 Codex CLI 或 Claude Code，程序沿用其登录、模型、信任和权限设置。

1. 点击左上角 **＋** 选择 Shell，或用 Ctrl＋Shift＋N 新建 PowerShell。新会话使用当前或最近使用的有效工作目录；可在 Shell 中用 `cd` 更换。
2. 输入 `codex` 或 `claude` 启动 Agent。切换已运行会话不会重启其终端；重启程序只恢复会话列表，不恢复运行中的进程和屏幕内容。恢复 AI 对话请使用 CLI 自己的恢复命令。
3. 左上角齿轮可设置语言和浮窗。鼠标悬停头像可查看会话信息；拖动浮窗边缘可调整位置，在设置中关闭浮窗。
4. 任务状态来自 CLI 支持的 hook。Codex 通常在首轮任务后才发送通知；如提交任务后仍无状态，可在 CLI 中使用 `/hooks` 自行审查。程序不会替用户信任 hook 或修改用户级 CLI 配置。
5. 普通滚轮查看终端历史；全屏应用启用鼠标协议时由应用处理滚轮，Shift＋滚轮查看本地历史。识别到 Codex／Claude 时，Ctrl＋V 或 Shift＋Insert 可把剪贴板图片交给 CLI 原生处理。

如果已安装 CLI 却找不到，请确认公开的 `.exe`、`.cmd` 或 `.bat` 启动器在当前或 Windows 用户／系统 PATH 中；仅在 WSL 或 PowerShell 别名中可用的命令不属于受支持的 Windows 入口。

默认会话数据位于 `%LOCALAPPDATA%/ai-terminal`，只保存会话元数据、界面偏好及临时 hook 配置；不保存提示词、对话正文或 API Key。退出时回收本程序管理的会话进程；关闭活动任务前请保存工作。

## 从源码开发

需要 Windows x64、Rust stable MSVC 工具链、Visual Studio C++ Build Tools、Windows SDK（含 `rc.exe`）及 Python 3.11+。已验证的工具链与平台边界见[开发脚本](scripts/README.md)。

```powershell
git clone https://github.com/FengLP5263/ai-terminal.git
cd ai-terminal
rustup component add rustfmt clippy
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`ci` 包含治理、格式、Clippy、Rust 和 Python 检查，不调用模型 API。macOS／Linux 原生 GUI 尚不支持；详细命令、打包和验证见[文档目录](docs/README.md)。

## 参与与许可证

欢迎报告问题、改善输入、渲染、状态、性能和文档。提交前请读[贡献指南](CONTRIBUTING.md)和[协作规范](AGENTS.md)，报告中不要包含凭据或私人对话。

项目代码使用 [MIT 许可证](LICENSE)；第三方组件保留各自许可，见[第三方声明](THIRD_PARTY_NOTICES.md)。
