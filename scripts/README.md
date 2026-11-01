# 开发与交付脚本

统一入口：Windows `scripts/windows/project.ps1 <action>`；POSIX `sh scripts/posix/project.sh <action>`。
从脚本位置定位仓库，不要求调用者当前目录。相对制品路径统一相对仓库根目录。
PowerShell 可用 Get-Help 查看完整命令头；POSIX 使用 --help。Python 要求 3.11+，可用 AI_TERMINAL_PYTHON 指定解释器。

| 动作 | Windows | POSIX 对应 | 目的 | 验证状态 |
| --- | --- | --- | --- | --- |
| governance | project.ps1 governance | project.sh governance | 文件、来源、隐私、文档和治理检查 | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| format-check | project.ps1 format-check | project.sh format-check | cargo fmt，无源码改写 | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| check | project.ps1 check | project.sh check | fmt + Clippy -D warnings | Windows；POSIX 原生受限 |
| test | project.ps1 test | project.sh test | Rust workspace + Python 包校验 | Windows；POSIX 原生受限 |
| ci | project.ps1 ci | project.sh ci | governance + check + test | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| build | project.ps1 build | project.sh build | debug 构建 | Windows 既有实现；新入口静态检查 |
| run | project.ps1 run | project.sh run | 构建并打开交互窗口 | Windows 既有实现；本轮不重复开窗 |
| release | project.ps1 release | project.sh release | release 构建 | 2026-10-30 package 调用通过 |
| smoke | project.ps1 smoke -Release | project.sh smoke -Release | 隔离真实 PTY、合成状态、GPU 截图和回收 | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| measure-idle | project.ps1 measure-idle -Release | project.sh measure-idle -Release | 管理器短窗口 CPU/内存测量 | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| package | project.ps1 package | project.sh package | release + ZIP + manifest + 回读校验 | 2026-10-30 -AllowDirty 构建与回读通过 |
| verify-package | project.ps1 verify-package -Path ZIP | project.sh verify-package -Path ZIP | 跨平台只读校验，不解压／执行 | 2026-10-30 Windows 通过；Git Bash 相关分支通过 |
| verify-claude | project.ps1 verify-claude -AllowModelRequest | project.sh verify-claude -AllowModelRequest | 明确授权后一次可能计费的模型核查 | 仅帮助／拒绝未授权路径；不重跑模型 |

POSIX 静态治理和包校验原生执行 Python；format-check 可调用已配置 Rust。
Git Bash 原生动作委托同仓库的 Windows 入口。macOS/Linux 对 GUI 构建／原生测试／模型 helper 动作明确返回非零，
没有隐藏跳过或假装可用。Git Bash 检查不是 macOS/Linux 运行验收，后者仍待对应平台验证。

Windows 构建由 `build.rs` 编译程序图标资源；需 Windows SDK 的 rc.exe，自动从 PATH／SDK 定位，也可用 RC 指定编译器路径。Windows 与 Git Bash 入口均沿用 Cargo 构建，无需独立的图片工具；制品源码身份包括 build.rs 与 assets/branding。

## 参数与影响

- Action 默认 governance。Release 仅切换 smoke/measure 的 debug/release 目录，测试前需已构建。
- Label 只允许字母、数字、连字符；Profile 为真实模型 helper 的 debug/release，默认 release。
- package 默认拒绝脏工作区；-AllowDirty 仅本地验收并在 manifest 标记，不能冒充干净发行。
- 模型核查必须带 -AllowModelRequest；还必须有用户明确授权，不被 ci/test 调用。
- .tools/env.ps1 和 .tools/cargo-network.toml 是可选且忽略的本机配置，不含在包和提交中。
- 错误返回非零；默认检查不升级依赖、不改用户数据、不操作远端。新包在 output/packages/ 唯一目录，不覆盖运行中的预览。
- 旧 scripts/dev.ps1、smoke.ps1、measure-idle.ps1、verify-claude.py 保留为兼容实现；使用同一权限和失败边界。

## 当前 CI 边界

project.ps1 ci 是厂商无关入口；源码托管于 GitHub，尚未配置云 runner，不把本机结果当作 GitHub Actions 检查。
常规 CI 不需要 GPU、模型账号或代理权限；真实 GPU smoke 和视觉验收在 Windows 交互环境单独执行。

2026-10-30 实跑：Windows ci（36 Rust + 3 Python）、原生 smoke、空闲测量、release package；
Git Bash governance/format-check。verify-package 与失败路径以最终验收记录为准；macOS/Linux runtime 未验证。
