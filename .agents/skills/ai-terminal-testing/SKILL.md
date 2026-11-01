---
name: ai-terminal-testing
description: 验证 ai-terminal 的离线 Rust 回归、治理脚本、Windows 原生 smoke、DPI 和受控 Agent 联调，区分付费模型请求。
---

# 测试与验收

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 measure-idle -Release
```

POSIX 配对入口见 scripts/README.md，macOS/Linux 只验证支持的静态治理／ZIP 校验。
测试保留现有模块内和 tests/ 布局。常规 test 包括 Rust workspace 与 Python 包校验测试，不调用模型 API。
Codex 配置测试在 CLI 未安装时可能跳过；Windows 的实际安装／真回合必须另行记录。
smoke 使用三个真实 PowerShell PTY 和合成生命周期，隔离 output/ 唯一目录，验证 JSON、PNG、退出和自身 PTY 回收。
异常时只停止本次创建的进程。性能记录应区分 manager 与 Agent 子进程、debug/release、GPU、测量窗口和当前版本。

未提交文字的 TUI 探针仅在已信任目录运行，不自动通过信任／授权；CLI 仍可能执行自有启动行为。
真实模型回合会沿用用户接入并计费，必须明确用户授权并传 `verify-claude -AllowModelRequest`；
预算上限 0.15 美元，禁用工具，不作为默认 CI。当前治理验收不自动运行它。
维护 tests/、scripts/、脚本目录、相关架构与验收记录；失败或未验证项如实保留，不把不同类型证据混称完整兼容。

原生截图／空闲测量应在优化构建结束后执行，避免重型编译干扰启动与 PTY 时间限制。
terminal_probe 与 workspace_probe 必须等待 CLI／样例内容就绪；没有实际内容的空白截图不能通过。

文字改动覆盖彩色 emoji、修饰／连接序列、光标绘制与多档缩放后的终端边界。原生探针必须检查进程退出码，截图完成不等于退出正常；TextRendererSession 在 on_exit 显式释放。

scroll_probe 使用独立 raw-console 本地消息样例验收应用内部滚轮，等待实际翻页内容后截图。覆盖正常／SGR 鼠标、MOUSE_MOTION、Shift 本地历史和 Ctrl 缩放；仍不发送历史方向键。真实 Claude 启动记录 SessionStart 与终端模式，不把启动核查当作模型任务完成验证。


图片／窗口粘贴改动另跑 `cargo test --offline --locked -p egui-winit --lib paste_tests`。该补丁不作为独立 workspace 成员，常规 test 动作必须显式包含此测试。terminal_probe 的 codex-image／claude-image 模式仅在明确的图片粘贴核查中读取现有剪贴板，不改写剪贴板、不发送 Enter；应等待 CLI 完整就绪后检查附件标记及截图。lifecycle_probe 的 offline 模式仅向本机临时 Responses 假服务提交固定样例，明确记录假模型回合与真实 hook 事件，不调用真实推理服务，不接受 hook 信任。


0.4.0 设置／浮窗核查另跑 examples/island_probe.rs 的原生多窗口探针（参数为独立 output 目录）。覆盖中英文截图、主窗口最小化、浮窗可见／置顶、点击恢复及关闭／重开，数据为合成任务状态，不启动 Agent 或模型。tests/settings_floating.rs 覆盖旧设置兼容、偏好保存、实际控件选择、横向滚动后的点击；App 测试保留实际 PTY PID 核查。探针只读取自身窗口属性，不向系统注入输入，不触碰用户窗口。
