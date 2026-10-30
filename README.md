# ai-terminal

精简的 Windows 原生 Agent 终端管理器，当前本地预览版本 **0.2.1**。
左侧按项目管理会话，右侧保留 Codex、Claude Code 或 PowerShell 的真实终端。
转圈表示工作中，黄点表示需要处理，绿点表示本轮完成未读。

## 开发与检查

Windows x64、Rust stable MSVC（已验证 1.99.1，最低版本见 Cargo.toml）、C++ Build Tools/Windows SDK，
以及 Python 3.11+。CLI 预先安装和配置；管理器无需另填 API Key。

```powershell
.\scripts\windows\project.ps1 governance
.\scripts\windows\project.ps1 check
.\scripts\windows\project.ps1 test
.\scripts\windows\project.ps1 run
```

原脚本 scripts/dev.ps1、smoke.ps1 等保留为兼容入口。
完整命令、参数和 Windows/POSIX 支持范围见 [脚本目录](scripts/README.md)。
macOS/Linux 原生 GUI/ConPTY 尚未支持；静态治理和 ZIP 校验可使用 POSIX 入口。

## 使用

1. 左上角＋或 Ctrl+Shift+N，选择项目目录、Agent 和可选名称。
2. 左侧切换，右键重命名／关闭；信任和权限仍在原 CLI 中处理。
3. Codex 首次在 /hooks 中检查并启用状态 hook；未连接时界面会提示。
4. 重开后点“继续会话”恢复 Agent 历史，不自动恢复旧进程／终端屏幕。

默认数据在 %LOCALAPPDATA%/ai-terminal，只保存会话元数据和临时 hook 配置。
退出会回收本程序拥有的会话进程；不会按名称清理其他终端。

## 本地制品

```powershell
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 package
```

package 生成 output/packages/ 下的独立版本目录、ZIP、manifest 和校验结果；不上传、不发布。
已有本机 0.2.1 预览在 output/preview-v0.2.1/ai-terminal.exe，目录不入 Git，可通过正式脚本重新打包。

## 架构与协作

Rust + egui/glow GPU 界面 + Alacritty + Windows ConPTY，不内嵌浏览器。
字体内置 Hack 常规／粗体／斜体；Windows 终端使用 DirectWrite／Direct2D，中文指定微软雅黑 UI，彩色表情使用系统字体。文字按物理像素网格和共同基线绘制。
后台状态消费独立于 UI 绘制，隐藏终端继续排空输出。

- [协作规范](AGENTS.md) · [架构总览](.agents/architecture/overview.md)
- [运行与配置](.agents/architecture/runtime.md) · [数据与资产](.agents/architecture/data-and-assets.md)
- [集成边界](.agents/architecture/integrations.md) · [第三方来源](THIRD_PARTY_NOTICES.md)
- [治理验收](docs/governance.md) · [原始选型](docs/architecture.md)

## 验证边界

常规测试不调用模型。原生 smoke 是真实 PowerShell + 合成状态；真实 Agent TUI 未提交文本与真实模型回合分别记录。
模型验证需显式授权与 AllowModelRequest，不包含在默认 CI。
物理输入法全部组合、图片粘贴、长期多会话、Claude fullscreen renderer 仍有未验证范围。
历史记录：[0.1.0](docs/verification.md)、[0.1.1 输入修复](docs/input-display-0.1.1.md)、
[0.1.2 REA/视觉修复](docs/visual-quality-0.1.2.md)、[0.2.0 侧栏改进](docs/visual-quality-0.2.0.md)、
[0.2.1 全窗口缩放与原生文字](docs/visual-quality-0.2.1.md)。历史性能不能当作新版测量。

### 会话面板与缩放

拖动左侧会话卡片调整顺序；悬停铅笔或右键编辑名称。‹ 收起为 Agent 头像栏，› 展开；拖面板右边缘调宽度。
窗口内 Ctrl＋滚轮缩放整个界面，侧栏、控件与终端同步变化。Ctrl＋加减可缩放，Ctrl＋0 或标题栏百分比按钮恢复 100%。顺序、面板偏好与缩放比例会自动保存。
