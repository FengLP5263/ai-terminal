---
name: ai-terminal-development
description: 在 ai-terminal 仓库进行 Windows 原生终端的本地开发、构建、输入渲染与状态修复，不包含远端发布。
---

# 本地开发

简体中文 | [English](SKILL.en.md)

先读根 AGENTS.md 与架构记录。Windows x64、Rust stable MSVC、C++ Build Tools/SDK；治理和打包需要 Python 3.11+。
工具链优先使用 PATH，可选加载忽略的 .tools/env.ps1；不能把个人工具路径写进源码或文档。

```powershell
.\scripts\windows\project.ps1 governance
.\scripts\windows\project.ps1 build
.\scripts\windows\project.ps1 run
```

POSIX 对应 `sh scripts/posix/project.sh <action>`。Git Bash 可委托 Windows；macOS/Linux 原生动作明确不支持。
默认渲染 OpenGL，替代 DX12 仅用于明确的 A/B 验证，不混用旧二进制结果。
保留源码布局、独立 helper、字体来源和会话级 hook 语义。正常退出只管理自身会话；不杀用户已有 CLI。
Git Bash 加载系统／用户登录配置后再追加会话集成，保留提示符与 PROMPT_COMMAND；不修改用户文件。发现 Git 时支持 PATH、安装记录和环境目录，缺失时显式提供可用 Shell 的回退；测试使用临时 HOME。
修改字体／输入／渲染时做真实截图与操作验收，测试计数不能证明观感或全部输入兼容。
完成后走 testing 技能；架构／配置／命令变化同步 .agents/architecture 与 scripts/README.md，vendor 补丁更新 UPSTREAM.md。

所有可见调试窗口、浮窗和调试示例 EXE 均使用 assets/branding 中用户选定的 B 图标；不要使用框架默认或早期图标。更新探针时验证窗口图标与 EXE 资源，保留 hook helper 不带 GUI 图标。

原生控制台固定资产见 assets/windows/conpty。build.rs 自动为 Cargo 实际输出目录准备 conpty.dll／OpenConsole.exe；不要用单独两个 ai-terminal EXE 代表完整开发制品。修改原生标题栏只针对自身窗口，不修改系统主题。标准 npm 启动器可做有界字面目标检查，自定义包装器保持不透明，不运行 CLI 健康探测或在非零退出后自动重试。
