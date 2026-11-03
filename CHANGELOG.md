# 更新日志

简体中文 | [English](CHANGELOG.en.md)

每次发版在这里记录用户可见的新增、改进、修复和兼容性变化；不适用的分类可省略。中英文同步审阅，GitHub Release 使用同一版本条目，ZIP 随附两份日志。历史条目依据已有 GitHub 发行说明整理，日期采用 UTC；未单独发行的开发版本不编造发布日期。

## [未发布]

尚无下一版本的已确认变更。

## [0.4.3] - 2026-11-03

### 新增

- 建立中英文更新日志入口，补齐已发布版本概览；打包检查当前版本条目，避免新包缺少更新说明。

### 改进

- 在 CMD、PowerShell、Git Bash 之间进入或退出嵌套 Shell 时，会话头像跟随当前 Shell 切换。
- 侧栏滚动条保持细窄并贴近分割线，悬停或滚动时只改变明暗；调整面板宽度与拖动滚动条互不抢占。

### 修复

- Git Bash 正确加载系统和用户启动配置，保留提示符、别名与补全；支持便携安装和安装记录，缺失时可明确改用 PowerShell／CMD。
- Codex／Claude Code 中可用普通鼠标左键拖选文字，再按 Ctrl+C 复制；没有选区时仍可用 Ctrl+C 中断，单击和应用内滚轮继续可用。
- 修复 Windows 10 旧控制台丢失部分全屏和鼠标控制信息的问题，随包提供固定版本、微软签名的原生控制台组件。
- 系统使用浅色模式时，原生标题栏仍保持深色，并在主题通知后维持该设置。[#13](https://github.com/FengLP5263/ai-terminal/issues/13)
- 标准 npm 启动脚本残留、目标缺失时，继续查找有效的 Codex／Claude 入口；自定义脚本保留原优先级，不修改全局 PATH。[#11](https://github.com/FengLP5263/ai-terminal/issues/11)

### 升级与验证范围

- Windows x64（Windows 10 1809／build 17763 或更高）ZIP 必须完整解压，`ai-terminal.exe`、`ai-terminal-hook.exe`、`conpty.dll` 和 `OpenConsole.exe` 放在同一目录；运行不需要 Rust、Python 或 Electron。
- Windows 10 22H2 与本机 Windows 环境经过原生检查和用户预览试用；补充了 125%、150%、151.13608% 缩放、标题栏、拖选、侧栏与浮窗回归。
- 高缩放输入区的原始触发序列仍未稳定复现，现有回归及用户试用通过；保留 [#12](https://github.com/FengLP5263/ai-terminal/issues/12) 跟踪，不把合成响应测试当作真实 Agent 多轮验收。详见[验收记录](docs/windows10-compatibility.md)。

## [0.4.2] - 2026-11-02

### 改进

- 复用 Windows OpenGL 图形缓冲区，减少长会话绘制与输入的重复开销。
- 后台保存并回收本程序拥有的会话，改善多会话退出时的界面响应。

### 修复

- 修复部分设备长时间运行时的内存增长、输入迟滞，以及输入法失焦后组合状态残留的问题。
- 调整设置和重命名弹窗布局，补充图形补丁来源和许可证记录。

## [0.4.1] - 2026-11-02

### 改进

- 改善不同缩放比例显示器之间移动窗口时的布局，更新双语首页、界面预览和上手说明。

### 修复

- 查看会话后清除未读完成标记；减少 Codex／Claude 自动审批等场景的“需要处理”误报。
- 为原生标题栏设置深色主题；Windows 10 的进一步修复见 0.4.3。

## [0.4.0] - 2026-11-01

### 新增

- 设置面板支持跟随系统语言或手动切换简体中文／English。
- 可选置顶会话浮窗，以头像和状态导航会话；一次最多显示 10 个，其余可拖动浏览。

### 改进

- 浮窗跟随全局缩放、在当前显示器居中，并适配不同 DPI；统一正式程序图标。
- 支持生命周期通知和实际退出状态，不根据输出静默时间猜测任务完成。

## [0.3.7] - 2026-11-01（预览版）

### 修复

- 恢复 Codex 原生全屏 TUI、启动界面、底部输入区和任务通知；改善 Claude 应用内滚动。
- 修复 Codex／Claude 原生图片粘贴和继承旧受管启动器的问题，补充依赖与许可证清单。

## [0.3.4] - 2026-11-01（预览版）

### 新增

- 新建会话直接使用 Shell，支持 PowerShell、CMD、Git Bash 和最近工作目录；按启动命令识别 Agent 头像。
- 会话颜色、分隔线、彩色表情名称及 B「会话叠窗」程序图标。

### 修复

- 修复滚轮切换输入历史和跨设备查找公开 CLI 入口的问题。

## [0.2.1] - 2026-10-30（预览版）

### 新增

- 首个公开预览：多 Agent 会话、状态提示、拖动排序、重命名、可收起侧栏与全窗口缩放。
- Windows 原生文字、中文和彩色表情，以及会话元数据和界面偏好保存。

[0.4.3]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.3
[0.4.2]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.2
[0.4.1]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.1
[0.4.0]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.0
[0.3.7]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.3.7
[0.3.4]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.3.4
[0.2.1]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.2.1
