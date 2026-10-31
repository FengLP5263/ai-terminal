# ai-terminal

一个方便管理多个 AI 编程会话的 Windows 原生终端。把不同项目的 Codex、Claude Code 和 PowerShell 会话放在一个窗口里，快速切换，并查看哪些 Agent 正在工作、等待处理或已完成。

A lightweight Windows-native terminal workspace for managing multiple AI coding agents.

**当前开发版本：0.2.2 · Windows x64 · MIT**

已发布的预览版及 Windows ZIP 见 [GitHub Releases](https://github.com/FengLP5263/ai-terminal/releases)。

[报告问题](https://github.com/FengLP5263/ai-terminal/issues) · [参与贡献](CONTRIBUTING.md) · [许可证](LICENSE)

## 核心功能

- **集中管理会话**：选择项目目录和 Agent，保留各 CLI 的真实终端、认证与权限操作。
- **会话状态提示**：转圈表示工作中，黄点表示需要处理，绿点表示本轮完成未读；后台会话也会更新。
- **整理工作区**：拖动排序、重命名、调整侧栏宽度，或收起为 Agent 头像栏。
- **整体缩放**：Ctrl＋滚轮或 Ctrl＋加减缩放整个界面，Ctrl＋0 恢复 100%；终端随可用区域重排。
- **原生文字显示**：内置 Hack 字体，Windows 原生文字绘制，支持中文、ANSI 颜色与系统彩色表情。
- **按需恢复**：保存会话列表、顺序、界面偏好和 Agent 会话 ID，重开后由你选择继续会话。

项目追求简洁、实用与性能。使用 Rust、egui、Alacritty 和 Windows ConPTY，不内嵌浏览器运行时。

## 从源码运行

如需直接使用，可下载 [Releases](https://github.com/FengLP5263/ai-terminal/releases) 中的 Windows ZIP，完整解压后运行 `ai-terminal.exe`，无需安装 Rust 或 Python。以下步骤适用于从源码构建。

准备环境：

- Windows x64。
- Rust stable 的 MSVC 工具链，以及 Visual Studio C++ Build Tools 与 Windows SDK。已验证 Rust 1.99.1，最低声明版本见 Cargo.toml。
- Python 3.11+，用于项目检查和打包。
- 预先安装并配置好要使用的 Codex CLI、Claude Code 或 PowerShell。ai-terminal 沿用 CLI 的登录与模型设置，无需再填写 API Key。

在 PowerShell 中执行：

```powershell
git clone https://github.com/FengLP5263/ai-terminal.git
cd ai-terminal
rustup component add rustfmt clippy
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`ci` 包括治理检查、格式、Clippy、Rust 与 Python 回归，不调用模型 API。
完整命令、参数和平台边界见[脚本目录](scripts/README.md)。macOS/Linux 原生 GUI 尚未支持。

## 使用

1. 点击左上角＋或按 Ctrl＋Shift＋N，选择项目目录、Agent 和可选名称。
2. 在左侧切换会话；拖动调整顺序，悬停铅笔或右键编辑名称，右键也可关闭会话。
3. 信任、登录和权限在原 CLI 中处理。Codex 首次需要在 `/hooks` 中审查并启用状态 hook；未连接时界面会提示。
4. 点击侧栏的 ‹ 收起为头像栏，› 展开；拖动侧栏右边缘调宽度。
5. 窗口内 Ctrl＋滚轮缩放整个界面（50%–250%），Ctrl＋0 或标题栏百分比按钮恢复 100%；普通滚轮保留终端历史滚动。
6. 重开后点“继续会话”恢复 Agent 历史；不会自动恢复旧进程或原始终端屏幕。

### 已安装 CLI 却提示找不到？

0.2.1 对 npm 安装布局有过强假设，会漏掉自定义安装目录。0.2.2 改为查找公开的 `.exe`、`.cmd` 和 `.bat` 启动器，并在每次启动会话时读取进程 PATH 及 Windows 保存的用户／系统 PATH；不再依赖包内部目录。

自定义安装目录若不在任何 PATH 中，可将启动器所在目录加入 Windows 用户 PATH，或从能够运行该 CLI 的终端启动 ai-terminal。仅在 WSL 或 PowerShell 别名／函数中可用的命令不等于已安装可直接启动的 Windows CLI。

默认数据位于 `%LOCALAPPDATA%/ai-terminal`，只保存会话元数据、界面偏好及临时 hook 配置。
ai-terminal 不保存提示词、对话正文或 API Key；各 CLI 自己的数据管理规则仍适用。
退出会回收本程序拥有的会话进程，不会按进程名清理其他终端。关闭活跃任务前请先保存工作。

## 本地打包

```powershell
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 package
```

`package` 要求干净的 Git 工作区，在 `output/packages/` 生成带 manifest、文件哈希和许可证的 ZIP，并回读校验；不会自动上传或发布。
两个 EXE 必须放在同一目录。原生 smoke 使用隔离数据和三个真实 PowerShell 终端，不操作你的现有会话。

## 欢迎一起完善

欢迎使用、报告问题、分享建议，也欢迎通过 Pull Request 一起完善。输入与渲染兼容性、状态可靠性、性能、无障碍和文档都是有价值的贡献方向。

提交前请阅读[贡献指南](CONTRIBUTING.md)。遇到问题请附上系统、CLI 版本、复现步骤和截图，隐藏密钥与私人对话。

## 架构与验证

- [协作规范](AGENTS.md) · [架构总览](.agents/architecture/overview.md)
- [运行与配置](.agents/architecture/runtime.md) · [数据与资产](.agents/architecture/data-and-assets.md)
- [集成边界](.agents/architecture/integrations.md) · [第三方来源](THIRD_PARTY_NOTICES.md)
- [0.2.1 原生文字与整体缩放验证](docs/visual-quality-0.2.1.md) · [治理记录](docs/governance.md)

状态来自 Agent 生命周期 hook 和进程退出；CLI 升级后可能需要重新验证 hook 兼容性。绿点表示本轮结束，不表示项目验收通过。
实际 Agent TUI、合成事件与真实模型回合分别记录；真实模型验证需要显式授权，不属于默认 CI。
物理输入法全部组合、图片粘贴、长期多会话和 Claude fullscreen renderer 仍有待验证范围。彩色表情受系统字体版本限制。

## 许可证

项目代码使用 [MIT License](LICENSE)。字体、图标和其他第三方组件保留各自许可证，详见[第三方来源](THIRD_PARTY_NOTICES.md)。
