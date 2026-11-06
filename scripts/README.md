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

同一探针追加 `codex-direct`：`target/release/examples/shell_switch_probe.exe output/<unique-codex-direct> codex-direct`，在当前已信任目录以绝对路径启动真实 `codex --remote unix:// -C <cwd>`，生成 Shell／Codex 两张正式界面截图。不发送模型提示、不确认信任或 hook、不停止共享后台；结束只回收自己的终端。POSIX 原生限制不变，没有新增 project 动作。

侧栏溢出与滚动条截图（仅 Windows，PowerShell／Git Bash）：`cargo build --offline --locked --release --bins --example workspace_probe`，再运行 `target/release/examples/workspace_probe.exe output/<unique-sidebar-probe> 1160 many`。many 模式生成 30 个合成会话，在展开、缩放和收起后滚动并悬停滚动条，写出 PNG／JSON；仅使用自身的 ANSI 样例 PTY，不启动 Agent 或模型。检查滚动条保持细窄、不遮挡头像，并核对正常退出。

全屏滚轮／拖选截图（仅 Windows）：`cargo build --offline --locked --release --example scroll_probe`，再用 `rustc --edition=2024 --crate-name mouse_console tests/fixtures/mouse_console.rs -o output/mouse-console.exe` 构建本地样例，运行 `target/release/examples/scroll_probe.exe output/mouse-console.exe output/<unique-selection-probe>`。检查上下翻页、普通左键拖选的高亮及连接说明截图和正常退出；复制行为由 `tests/terminal_interaction.rs` 的真实 ConPTY 回归覆盖。原生探针只向自身窗口输入，不写系统剪贴板、不启动 Agent 或模型。

Windows 兼容性探针：先运行 `cargo build --offline --locked --release --bins --example composer_probe --example titlebar_probe --example discovery_probe`，并用前述 rustc 命令准备 output/mouse-console.exe。分别执行 `target/release/examples/composer_probe.exe output/mouse-console.exe output/<unique-composer-probe>`、`target/release/examples/titlebar_probe.exe output/<unique-titlebar-probe>` 和 `target/release/examples/discovery_probe.exe output/<unique-discovery-probe>`。前两项只使用自身窗口／本地样例，后一项只执行已安装 CLI 的 --version；核对 JSON、PNG 和退出码，Windows 10／11 的结果分开记录。构建会准备固定的原生控制台组件；不能只复制探针 EXE 到另一设备。

浮窗跨屏／DPI 核查（至少两块真实显示器）：cargo build --offline --locked --release --example island_move_probe，然后运行 target/release/examples/island_move_probe.exe output/unique-island-move。探针只移动自身浮窗，在 71%、142%、200% 等缩放下检查原生客户区尺寸并生成逐阶段 PNG／JSON；没有第二屏时明确失败，不操作用户窗口、不启动 Agent 或模型。截图仍需人工查看；原因与 Codex 代理入口边界见 [界面渲染](../docs/ui-rendering.md)。
真实 Claude 手动模式：在已有可信目录运行 terminal_probe.exe claude-manual output/unique-manual-probe，路径指向已构建的 examples 程序。只为本次 CLI 添加 --permission-mode default，不改变用户偏好、不提交模型请求。探针缺少截图或结果时非零退出；信任提示不能当作成功。

收起侧栏临界布局：cargo build --offline --locked --example workspace_probe，再运行 target/debug/examples/workspace_probe.exe output/unique-sidebar-threshold 900 threshold。使用七个合成会话与独立样例 PTY，在正常大小、缩放和重设窗口尺寸后连续比较多帧会话区域，生成三张 PNG 与 stability_checks 计数；布局抽动会使探针失败。普通 many 模式仍用于长列表滚动条截图，不代替临界布局检查。

preview.3 的 island_move_probe 另检查原生移动开始／结束通知、移动后多帧位置保持与异常客户区尺寸恢复。通知和程序化跨屏分别执行，不模拟系统鼠标模态循环；结果 JSON 的 physical_mouse_drag 为 false，实际鼠标跨屏仍需独立验收。

preview.4 跨屏探针沿用上述命令，同时移动自己的主窗口与浮窗，原生移动状态覆盖 DPI 切换，保存 main-*.png／move-*.png 和 dpi_trace。检查建议／实际矩形误差、递归深度、移动期间尺寸修正延迟和重复跨屏尺寸稳定。physical_mouse_drag 仍为 false；真实鼠标模态拖动另行验收。本地 winit 的构建源码／嵌入资产已计入制品源码身份及依赖许可证清单。

README 公开截图（仅 Windows）：构建 `cargo build --offline --locked --release --example readme_capture --example island_probe`，分别运行 `target/release/examples/readme_capture.exe output/<unique-readme-workspace>` 和 `target/release/examples/island_probe.exe output/<unique-readme-floating>`。前者复用正式侧栏、字体与终端组件，仅输出固定文本、示例目录和合成状态；后者捕获正式浮窗。两者不启动 Agent／模型，不加载用户 profile，只管理自身窗口和 PTY。核对退出码、JSON、PNG 及隐私后，将 workspace.png 和 island-zh.png 原样作为 README 图片，更新 assets/readme/SOURCE.json 的版本、来源、尺寸与 SHA-256。

Codex 原生状态／总览与模式图标（仅 Windows）：构建 `cargo build --offline --locked --release --bins --example agents_status_probe`。在已有可信目录运行 `target/release/examples/agents_status_probe.exe codex output/<unique-agents-probe>`，只发送左箭头进入 agents 总览和 Esc 返回，生成三张 PNG 与有限状态 JSON；不发送提示词、不接受项目／hook 信任、不停止用户共享后台。`target/release/examples/agents_status_probe.exe badges output/<unique-badge-probe>` 使用固定 PowerShell 文本，生成 100%／146%／200% 的相邻三角／暂停／emoji 截图。检查 JSON、退出码及实际图像；启动导航不是模型回合。POSIX 原生限制与既有入口一致，没有新增 project 动作。

侧栏拖动及失败未读截图（仅 Windows，PowerShell／Git Bash）：`cargo build --offline --locked --release --example sidebar_drag_probe`，再运行 `target/release/examples/sidebar_drag_probe.exe output/<unique-sidebar-drag-probe>`。正式侧栏和浮窗组件、标准 B 图标、固定示例目录；生成展开／收起的未读、已读、拖动卡片和松手排序共八张原生 PNG 及 result.json。只向探针自身 egui 发送合成输入，不启动 PTY、Agent 或模型，不读用户配置、不注入系统鼠标。核对进程退出码、抓取点、卡片让位与红点清除，截图完成不替代视觉检查。

Qoder CN 启动／合成状态截图（仅 Windows，PowerShell／Git Bash）：`cargo build --offline --locked --release --bins --example qoder_cn_probe`；`target/release/examples/qoder_cn_probe.exe output/<unique-qoder-fixture> fixture` 生成十一种状态图；省略 fixture 时仅启动已安装的 Qoder CN 并截图，不输入提示或确认信任。默认模式使用当前目录；请在已有可信目录执行。result.json 分别记录 synthetic、hook_connected、事件名称及未发送模型请求，必须核对退出码和实际画面。

package 另将图标来源清单和可获得的原始许可复制到 ZIP 的 agent-icons/，并附带双语 THIRD_PARTY_NOTICES；品牌资产按来源分别声明，不统一套用 Lobe MIT。Windows／POSIX 的统一入口和参数不变。

0.4.7 新增已安装 CLI 的原生启动核查：先构建 `cargo build --offline --locked --release --bins --example shell_switch_probe`，再分别执行 `target/release/examples/shell_switch_probe.exe output/<unique-cli-probe> kimi`（末项也可为 `zcode`、`traecli`）。仅输入启动命令，聚焦终端空白区后发送 Esc／Ctrl+C，不提交提示或接受信任；人工检查三张截图中的头像、等待任务通知及返回 Shell，记录实际发行版。README 头像由 `cargo build --offline --locked --release --example agent_icons_capture` 和 `target/release/examples/agent_icons_capture.exe output/<unique-agent-icons>` 生成；无 PTY、Agent 或模型，复用正式头像组件及 B 程序图标。核查六枚 PNG 后原样复制到 assets/readme 并更新 SOURCE.json。均仅支持 Windows 原生运行，没有新增 project 动作或参数。

0.4.8 预览 README 截图：Release 构建 `readme_capture` 后分别运行 `target/release/examples/readme_capture.exe output/readme-zh` 和 `target/release/examples/readme_capture.exe output/readme-en en`，每次使用全新目录；英文模式同时切换控件、名称及终端合成文本。`island_probe` 一次生成六种 Agent 的中英文浮窗；检查退出码、result.json 和四张实际 PNG 后原样复制到 assets/readme 并更新 SOURCE.json。不是模型验收。

原生浮窗提示核查（Windows）：`cargo build --offline --locked --release --example island_hint_probe` 后运行 `target/release/examples/island_hint_probe.exe output/<unique-hint-probe>`。探针只向自己的 egui 窗口输入合成悬停，在 100%／142%／200% 缩放检查完整中英文提示超出胶囊、长名称换行、归属窗口、无激活、鼠标穿透和离开隐藏；生成原生提示 PNG、hook 进行中状态截图与 JSON，不启动 Agent／模型、不操作用户窗口。位置纯函数另覆盖负坐标显示器与工作区边缘。截图需人工检查，合成 hook 状态不等于真实模型请求。

0.4.9：qoder_cn_probe fixture 扩展为 11 张状态图，覆盖自动审批、真实问题、手动等待、工具失败后恢复与迟到通知。island_hint_probe 使用实际 egui 提示视口截图，检查爱心／ZWJ 彩色 emoji、窗口归属、无激活、穿透及离开隐藏。`cargo build --offline --locked --release --example typography_probe` 后运行 `target/release/examples/typography_probe.exe output/<unique-typography-probe>`，用真实 PowerShell ConPTY 固定样例检查 100%／103%／110%／150%／200% 下的中英文、彩色 emoji 与灰色输入区黑色宽字符光标；不启动 Agent／模型或读取用户 profile。terminal_interaction 分别验证普通与私有光标查询，以及中文宽字符的背景／光标／反色字形顺序。原生图片逐张验收，不能仅以回归计数证明观感。

Qoder 已信任目录输入核查：`target/release/examples/qoder_cn_probe.exe output/<unique-qoder-input> input`。仅输入固定未提交标记、100%／103%／110% 缩放及一次 Ctrl+C 后重新输入，生成四张原生图与可见性记录；绝不发送 Enter 或确认信任，不能冒充真实任务完成验收。

字体对比：`typography_probe output/<unique-font-check> compare` 打开交互窗口，分别切换三种英文／中文字体及缩放；不保存选择、不改用户工作区。自动截图用第二参数 `hack`／`consolas`／`cascadia`，第三参数可选 `yahei`／`dengxian`（默认微软雅黑 UI），仍捕获五档比例。候选系统字体须已安装；缺少时不能把回退字形冒充该字体。正式 App 仍使用 appearance.rs 默认字体和 0.4.8 原生参数。

0.4.9-preview.3 默认内嵌 Cascadia Mono 四种静态字形，中文用系统 Microsoft YaHei UI；不重新启用 preview.1 的 GDI classic 拟合。package 核对 assets/fonts/SOURCE.json 的全部字节与哈希，并复制原始 Cascadia OFL 和字体来源；verify-package 保留旧包兼容，新包声明 fonts 元数据时要求完整来源与许可。Windows／POSIX 入口和参数不变。

Qoder 尺寸诊断：构建 qoder_cn_probe 后运行 `target/release/examples/qoder_cn_probe.exe output/<unique-qoder-trace> trace <isolated-directory>`。只启动一次 CLI，在 103% 下程序化调整四次窗口并记录最多 128 项行列／布局元数据和四张截图；不输入提示、不接受信任。目录信任页或鉴权页与真实模型任务分开记录。

Windows 瞬时错绘核查：`cargo build --offline --locked --release --example render_update_probe`，随后运行 `target/release/examples/render_update_probe.exe output/<unique-render-update> 1.03 48`。可选参数为缩放（0.5–2.5）与 8–512 次截图比较；该探针检查单窗口固定表格；多窗口和悬停提示由 island_hint_probe 单独核查。两个本地 PowerShell 固定样例分别提供文件表格与变换字号的字形区域，不启动 Agent／模型、不读取用户 profile。比较固定表格的逐帧像素和可见网格哈希，保存基线、阶段图、首个异常和 JSON；超时或任何像素／网格差异均失败。根窗口大小按所选缩放创建，大比例可能超过屏幕，截图来自自身 GPU 帧缓冲区。正式 App 不增加轮询或空闲重绘。

## Windows 安装版（0.4.10 预览）

```powershell
.\scripts\windows\project.ps1 installer -AllowDirty
.\scripts\windows\project.ps1 installer -Path output/packages/<package>.zip -Acceptance -AcceptanceId <unique-id> -AllowDirty
.\scripts\windows\project.ps1 verify-installer -Path output/installers/<build>/<setup>.json
.\scripts\windows\project.ps1 installer-smoke -Path output/installers/<new>/<setup>.json -Previous output/installers/<old>/<setup>.json
```

POSIX 对应动作与参数相同；`verify-installer` 可跨平台只读运行，其他安装动作仅支持 Windows／Git Bash。`installer` 不带 `-Path` 时先调用既有 ZIP 构建。正式包拒绝脏源码／脏 ZIP；本地预览显式 `-AllowDirty`。`-Acceptance` 使用独立的验收身份，两个版本配合同一个 `-AcceptanceId`（6–64 个小写字母／数字／连字符）。原生验收拒绝普通安装包，首次安装、升级、占用阻止与卸载只处理该测试自己的文件和注册表；额外文件与会话数据保留。

构建依赖固定 NSIS 3.13 官方 ZIP：从 [NSIS 官方下载](https://nsis.sourceforge.io/Download) 获取 `nsis-3.13.zip`，完整解压到忽略的 `.tools/nsis/`（编译器位于 `.tools/nsis/nsis-3.13/makensis.exe`），或以 `AI_TERMINAL_NSIS` 指定同一解压树的编译器。构建前核对 `scripts/installer/TOOLCHAIN.json` 的归档和完整文件树 SHA-256；不自动下载或安装工具，不执行用户 NSIS 配置。原始许可见 `scripts/installer/NSIS-LICENSE.txt`，源代码地址记录在 TOOLCHAIN.json。编译工具留在开发设备，用户无需安装它。

安装版生成 EXE、`.sha256` 和 `.json`；EXE 校验不等于签名或实际安装验收。源清单记录 ZIP／安装脚本／编译器身份及预览状态，不包含个人路径。程序与测试边界见 [安装与字体说明](../docs/installation.md)。

字体原生截图：未指定字体时使用正式默认 JetBrains Mono。构建 `typography_probe` 后，以 `cascadia`、`hack`、`consolas`、`courier_new`、`jetbrains_mono` 作为第二参数，分别捕获 100%／103%／110%／150%／200%。第四参数可用 `settings-zh`／`settings-en`（第三参数使用 `default`）捕获正式设置控件。缺失字体明确失败，不冒充回退字体。截图需目视验收，样例仅含本地固定文本。
