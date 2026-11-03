# 开发与交付脚本

简体中文 | [English](README.en.md)

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

governance、ci 和 package 会检查根中英文更新日志是否含 Cargo 当前版本的唯一日期条目、分类与实际变更，且双语日期相同；package 将两份完整日志装入 ZIP 并计入源码摘要。Windows／POSIX 统一入口及参数不变；新增回归包含缺失翻译、错误／重复版本、日期分歧及空当前条目。

project.ps1 ci 是厂商无关入口；源码托管于 GitHub，尚未配置云 runner，不把本机结果当作 GitHub Actions 检查。
常规 CI 不需要 GPU、模型账号或代理权限；真实 GPU smoke 和视觉验收在 Windows 交互环境单独执行。

2026-10-30 实跑：Windows ci（36 Rust + 3 Python）、原生 smoke、空闲测量、release package；
Git Bash governance/format-check。verify-package 与失败路径以最终验收记录为准；macOS/Linux runtime 未验证。


0.3.7 起 test／ci 显式执行本地 egui-winit 补丁的 paste_tests，覆盖无文本的粘贴请求；该补丁保持在 workspace 外。真实图片核查由 terminal_probe 的 codex-image／claude-image 模式完成，只插入未发送的附件。lifecycle_probe offline 使用本机动态端口的假 Responses 服务验证真实 CLI 生命周期，不消耗真实模型。


打包从 Cargo metadata 的 target_directory 读取本次 release 二进制，遵循标准 CARGO_TARGET_DIR／build.target-dir。默认构建目录的 EXE 被正在运行的预览占用时，可在当前命令环境设置 CARGO_TARGET_DIR 为另一个忽略的构建目录再执行相同 package 入口；不会覆盖已打开的程序，也不会从旧默认目录误取 EXE。


0.4.0 多窗口开发验收：先用现有工具链运行 `cargo build --offline --locked --example island_probe`，再运行 `target/debug/examples/island_probe.exe output/<unique-island-probe>`。探针只使用合成状态和自身窗口，生成中英文 PNG／JSON 后退出；与真实 Agent 回合分开记录。浮窗默认关闭，性能核查需要明确记录是否开启以及任务状态。

缩放回归另运行 `cargo build --offline --locked --example island_zoom_probe` 和 `target/debug/examples/island_zoom_probe.exe output/<unique-zoom-probe>`，核验真实原生视口在 50%、71%、100% 和 150% 下的物理尺寸、屏幕居中、垂直位置与截图；探针只读取自身窗口并使用合成会话；可附加目标显示器的物理 X 坐标来验证不同 DPI 的屏幕。

制品源码身份还包含本地 egui_glow 的 Cargo 清单、Rust 源码和 GLSL 着色器，避免遗漏图形补丁。

Git Bash 原生启动与截图核查（仅 Windows，PowerShell 或 Git Bash 均可执行）：`cargo build --offline --locked --bins --example bash_probe`，再运行 `target/debug/examples/bash_probe.exe output/<unique-bash-probe>`。探针使用真实 Git Bash、临时 HOME 与合成用户启动文件，核验目录元数据、配置／Agent 入口及 PNG 后退出；不读取真实用户配置、不启动模型。Git 缺失或验证失败会明确返回非零。缺失回退的合成测试与已安装 Git 的原生验收分开记录。

Shell 头像的原生截图核查（仅 Windows，PowerShell／Git Bash）：`cargo build --offline --locked --release --bins --example shell_switch_probe`，再运行 `target/release/examples/shell_switch_probe.exe output/<unique-shell-switch-probe>`。使用正式 App 控件和 B 图标、隔离会话与系统目录，不读取用户 profile、不启动 Agent／模型；只向自身窗口注入 CMD→PowerShell→CMD→逐层 exit 的输入，生成五张截图并正常退出。capture.json 仅说明截图已生成，必须人工确认内容、头像和正常退出，不能当作自动视觉验收。

侧栏溢出与滚动条截图（仅 Windows，PowerShell／Git Bash）：`cargo build --offline --locked --release --bins --example workspace_probe`，再运行 `target/release/examples/workspace_probe.exe output/<unique-sidebar-probe> 1160 many`。many 模式生成 30 个合成会话，在展开、缩放和收起后滚动并悬停滚动条，写出 PNG／JSON；仅使用自身的 ANSI 样例 PTY，不启动 Agent 或模型。检查滚动条保持细窄、不遮挡头像，并核对正常退出。

全屏滚轮／拖选截图（仅 Windows）：`cargo build --offline --locked --release --example scroll_probe`，再用 `rustc --edition=2024 --crate-name mouse_console tests/fixtures/mouse_console.rs -o output/mouse-console.exe` 构建本地样例，运行 `target/release/examples/scroll_probe.exe output/mouse-console.exe output/<unique-selection-probe>`。检查上下翻页、普通左键拖选的高亮及连接说明截图和正常退出；复制行为由 `tests/terminal_interaction.rs` 的真实 ConPTY 回归覆盖。原生探针只向自身窗口输入，不写系统剪贴板、不启动 Agent 或模型。

Windows 兼容性探针：先运行 `cargo build --offline --locked --release --bins --example composer_probe --example titlebar_probe --example discovery_probe`，并用前述 rustc 命令准备 output/mouse-console.exe。分别执行 `target/release/examples/composer_probe.exe output/mouse-console.exe output/<unique-composer-probe>`、`target/release/examples/titlebar_probe.exe output/<unique-titlebar-probe>` 和 `target/release/examples/discovery_probe.exe output/<unique-discovery-probe>`。前两项只使用自身窗口／本地样例，后一项只执行已安装 CLI 的 --version；核对 JSON、PNG 和退出码，Windows 10／11 的结果分开记录。构建会准备固定的原生控制台组件；不能只复制探针 EXE 到另一设备。
