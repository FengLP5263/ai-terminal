---
name: ai-terminal-development
description: 在 ai-terminal 仓库进行 Windows 原生终端的本地开发、构建、输入渲染与状态修复，不包含远端发布。
---

# 本地开发

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
修改字体／输入／渲染时做真实截图与操作验收，测试计数不能证明观感或全部输入兼容。
完成后走 testing 技能；架构／配置／命令变化同步 .agents/architecture 与 scripts/README.md，vendor 补丁更新 UPSTREAM.md。
