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
