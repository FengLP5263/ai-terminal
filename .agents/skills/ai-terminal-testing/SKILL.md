---
name: ai-terminal-testing
description: 验证 ai-terminal 的离线 Rust 回归、治理脚本、Windows 原生 smoke、DPI 和受控 Agent 联调，区分付费模型请求。
---

# 测试与验收

简体中文 | [English](SKILL.en.md)

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 measure-idle -Release
```

POSIX 配对入口见 scripts/README.md，macOS/Linux 只验证支持的静态治理／ZIP 校验。
测试保留现有模块内和 tests/ 布局。常规 test 包括 Rust workspace 与 Python 包校验测试，不调用模型 API。
Codex 配置测试在 CLI 未安装时可能跳过；Windows 的实际安装／真回合必须另行记录。
smoke 使用六个真实 PowerShell PTY 和合成生命周期（覆盖六个 Agent 头像），隔离 output/ 唯一目录，验证 JSON、PNG、退出和自身 PTY 回收。
Git Bash 原生回归使用临时 HOME 和合成启动文件，禁止执行开发者真实启动配置；覆盖登录文件优先级、PS1、别名、补全、标量／数组 PROMPT_COMMAND、目录跟踪和 Agent 入口。发现与回退测试覆盖便携路径、其他 Bash 的拒绝、缺少 Git／PowerShell 以及点击回退后的真实类型和数据保留。Git 未安装时原生 Bash 核查明确记录未运行，不宣称跨设备完全验证。
嵌套 Shell 核查使用真实 CMD→PowerShell→CMD 和本地 Agent 样例，验证父调用、根控制台归属、逐层退出与原 PTY 保留；PowerShell 测试带 -NoProfile，Git Bash 使用临时 HOME。头像归约测试另覆盖脱离控制台、过期父调用和 Agent 优先级；不得将合成事件当作真实模型回合。
异常时只停止本次创建的进程。性能记录应区分 manager 与 Agent 子进程、debug/release、GPU、测量窗口和当前版本。

未提交文字的 TUI 探针仅在已信任目录运行，不自动通过信任／授权；CLI 仍可能执行自有启动行为。
真实模型回合会沿用用户接入并计费，必须明确用户授权并传 `verify-claude -AllowModelRequest`；
预算上限 0.15 美元，禁用工具，不作为默认 CI。当前治理验收不自动运行它。
维护 tests/、scripts/、脚本目录、相关架构与验收记录；失败或未验证项如实保留，不把不同类型证据混称完整兼容。

原生截图／空闲测量应在优化构建结束后执行，避免重型编译干扰启动与 PTY 时间限制。
terminal_probe 与 workspace_probe 必须等待 CLI／样例内容就绪；没有实际内容的空白截图不能通过。

文字改动覆盖彩色 emoji、修饰／连接序列、光标绘制与多档缩放后的终端边界。原生探针必须检查进程退出码，截图完成不等于退出正常；TextRendererSession 在 on_exit 显式释放。

scroll_probe 使用独立 raw-console 本地消息样例验收应用内部滚轮，等待实际翻页内容后截图。覆盖正常／SGR 鼠标、MOUSE_MOTION、Shift 本地历史和 Ctrl 缩放；仍不发送历史方向键。真实 Claude 启动记录 SessionStart 与终端模式，不把启动核查当作模型任务完成验证。

鼠标拖选改动用 terminal_interaction 的真实 ConPTY 测试覆盖普通／SGR、1002／1003、普通单击、Shift 选择、选区复制和无选区中断。scroll_probe 另截图验证左键拖选的实际高亮，不改写系统剪贴板；不要用测试中启用的选择偏好代替正式 App 对已识别 Codex／Claude 的接入核查。


图片／窗口粘贴改动另跑 `cargo test --offline --locked -p egui-winit --lib paste_tests`。该补丁不作为独立 workspace 成员，常规 test 动作必须显式包含此测试。terminal_probe 的 codex-image／claude-image 模式仅在明确的图片粘贴核查中读取现有剪贴板，不改写剪贴板、不发送 Enter；应等待 CLI 完整就绪后检查附件标记及截图。lifecycle_probe 的 offline 模式仅向本机临时 Responses 假服务提交固定样例，明确记录假模型回合与真实 hook 事件，不调用真实推理服务，不接受 hook 信任。


0.4.0 设置／浮窗核查另跑 examples/island_probe.rs 的原生多窗口探针（参数为独立 output 目录）。覆盖中英文截图、主窗口最小化、浮窗可见／置顶、点击恢复及关闭／重开，数据为合成任务状态，不启动 Agent 或模型。tests/settings_floating.rs 覆盖旧设置兼容、偏好保存、实际控件选择、横向滚动后的点击；App 测试保留实际 PTY PID 核查。探针只读取自身窗口属性，不向系统注入输入，不触碰用户窗口。

Windows 10／11 分别运行 composer_probe、titlebar_probe 和 discovery_probe，命令见 scripts/README.md。composer_probe 检查恢复到 151.13608% 后的输入区及 125%／150%／151.13608% 切换、末行与视口边界；titlebar_probe 只捕获自身窗口，检查初始及主题通知后的标题栏背景和按钮；discovery_probe 仅查询 --version，不把它当作模型回合。可用 AI_TERMINAL_MOUSE_TRACE 为独立 mouse_console 样例记录模式和最多 256 字节的合成输入，不用于用户 CLI 或真实会话。升级控制台组件后同时核查实际关闭和自身 PTY 回收。
浮窗跨屏核查：构建并运行 examples/island_move_probe.rs（命令见 scripts/README.md），至少两块真实显示器；只移动探针自己的原生窗口，检查 71%、142%、200% 等缩放下的客户区尺寸与头像截图，不注入用户输入、不启动 Agent。缺少第二屏明确失败。
收起侧栏修复必须覆盖刚好溢出的边界：运行 sidebar_interaction 的多帧／空闲重绘回归，以及 workspace_probe 的 threshold 模式（scripts/README.md）。many 模式的长列表截图无法证明临界点不发生滚动条与行高的反馈循环。

浮窗的程序化移动与真实鼠标拖动必须分别记录。island_move_probe 单独测试原生进入／退出移动通知、程序化跨屏、移动后多帧位置保持及异常客户区尺寸恢复；不得将其标成已模拟 Windows 鼠标模态拖动。工具通道不可用时明确保留真实拖动的验收边界，不因探针通过就声明用户现象全部解决。

preview.4 的 island_move_probe 会保持原生移动状态跨越实际 DPI 边界，同时检查主窗口与浮窗，记录建议／实际矩形的差值、DPI 递归深度、拖动期间不额外修正尺寸、松手后恢复及重复跨屏尺寸稳定。该测试仍不注入系统鼠标，不等同于真实鼠标模态拖动；旧 preview.3 仅在跨屏前分别发送进入／退出通知的证据不能代替它。

公开 README 截图用 Release 的 readme_capture 与 island_probe 分开生成；命令见 scripts/README.md。复用正式侧栏／终端／浮窗组件，只用固定合成文本、示例目录和模拟状态，不读取用户 profile、不启动 Agent 或模型。核对正常退出、原生 PNG 和隐私后原样复制，更新 assets/readme/SOURCE.json；不能用公开展示样例代替真实 Agent 验收。

原生 Codex 状态／agents 验收使用 agents_status_probe，命令见 scripts/README.md。codex 模式只发送左箭头和 Esc，核对真实入口、总览、返回及原生 idle 通知；不发送提示词、不接受 hook／项目信任、不停止共享后台。badges 模式只用合成文本截图核对 100%／146%／200% 下相邻模式图标。队列、旧代次、隐藏 ConPTY、未读及旧 hook 覆盖另由离线回归验证；启动／导航截图不能冒充真实模型回合。

会话拖动与失败未读核查使用 `sidebar_drag_probe`（命令见 scripts/README.md）。只用合成元数据和自身 egui 输入，截图展开／收起的未读、已读、拖动及松手状态，不启动 PTY、Agent 或模型，不注入系统输入。`sidebar_interaction` 覆盖非中心抓取点、固定行槽让位、多档缩放、向上／向下排序、Esc／列表外取消、边缘滚动及取消后空闲；`settings_floating` 同时验证失败红点与完成绿点在查看后清除。App 的启动／退出失败回归与真实模型任务分别记录。

Qoder CN：运行 tests/qoder_cn.rs 的发现、配置、恢复和真实 helper 的合成元数据回归；shell_sessions 使用伪 CLI 验证 PowerShell／CMD／Git Bash 的两个入口和显式设置保留。qoder_cn_probe 的 fixture 模式只画合成状态；默认模式只启动已安装 CLI，不发送提示、不确认信任或授权。截图完成与 hook_connected 分开记录，不把登录／信任页面当成已连接，也不冒称真实模型任务通过。

Codex 直接启动回归使用 App 的真实 Shell／ConPTY 和绝对路径伪 codex.cmd，保留 --remote／-C／resume 参数，核对未识别阶段、原生状态、隐藏后台归约、clear 临时交接、返回提示符与重复调用。`shell_switch_probe` 可追加 `codex-direct` 做实际绝对路径／--remote unix:// 启动截图，仅在已信任目录使用，不发送模型提示、不确认信任、不关闭共享后台；截图需人工核对，不等于真实模型回合。

0.4.7 三款新增入口的真实启动用 shell_switch_probe 的 kimi／zcode／traecli 模式，命令见 scripts/README.md；仅向自建窗口输入启动命令，聚焦终端空白区后发送 Esc／Ctrl+C，核查身份和返回 Shell，不输入提示、不接受信任。记录 ZCode 官方或社区发行来源；本机 zcode-app-cli 不等于官方独立 CLI。README 六枚深色底衬头像用 agent_icons_capture 复用正式组件原生生成，不启动 PTY 或 Agent；人工核对后原样复制并记录来源。

0.4.8 预览：Kimi 的真实目录信任页应收到原生 permission 状态，探针只取消，不接受信任。ZCode 的会话级设置快照与 TRAE 的会话参数通过合成 helper／ConPTY 测试核查；启动 TUI 不代表 hook 已获信任或模型回合已通过。ZCode 自定义配置保留、子事件过滤及 camelCase turnId 别名另有回归；缺失的授权／失败／中断事件不得补猜测。readme_capture 的可选 `en` 参数切换控件、名称和终端文本，并等待固定样例末行；公开使用中英文两套主图，逐张人工检查，不采用不完整截图。

原生浮窗提示核查（Windows）：`cargo build --offline --locked --release --example island_hint_probe` 后运行 `target/release/examples/island_hint_probe.exe output/<unique-hint-probe>`。探针只向自己的 egui 窗口输入合成悬停，在 100%／142%／200% 缩放检查完整中英文提示超出胶囊、长名称换行、归属窗口、无激活、鼠标穿透和离开隐藏；生成原生提示 PNG、hook 进行中状态截图与 JSON，不启动 Agent／模型、不操作用户窗口。位置纯函数另覆盖负坐标显示器与工作区边缘。截图需人工检查，合成 hook 状态不等于真实模型请求。

0.4.9：qoder_cn_probe fixture 扩展为 11 张状态图，覆盖自动审批、真实问题、手动等待、工具失败后恢复与迟到通知。island_hint_probe 使用实际 egui 提示视口截图，检查爱心／ZWJ 彩色 emoji、窗口归属、无激活、穿透及离开隐藏。`cargo build --offline --locked --release --example typography_probe` 后运行 `target/release/examples/typography_probe.exe output/<unique-typography-probe>`，用真实 PowerShell ConPTY 固定样例检查 100%／103%／110%／150%／200% 下的中英文、彩色 emoji 与灰色输入区黑色宽字符光标；不启动 Agent／模型或读取用户 profile。terminal_interaction 分别验证普通与私有光标查询，以及中文宽字符的背景／光标／反色字形顺序。原生图片逐张验收，不能仅以回归计数证明观感。

Qoder 未提交输入验收：在已确认信任的目录运行 `target/release/examples/qoder_cn_probe.exe output/<unique-qoder-input> input`，仅输入固定标记、切换 100%／103%／110% 并按一次 Ctrl+C 后重新输入，不发送 Enter 或接受信任；记录四张原生截图、可见标记和真实事件名称。

字体对比：`typography_probe output/<unique-font-check> compare` 打开交互窗口，分别切换三种英文／中文字体及缩放；不保存选择、不改用户工作区。自动截图用第二参数 `hack`／`consolas`／`cascadia`，第三参数可选 `yahei`／`dengxian`（默认微软雅黑 UI），仍捕获五档比例。候选系统字体须已安装；缺少时不能把回退字形冒充该字体。正式 App 仍使用 appearance.rs 默认字体和 0.4.8 原生参数。

preview.3 的 typography_probe 默认使用正式内嵌 Cascadia Mono，而非本机同名字体；hack 模式明确使用原 Hack 字节，Consolas 仍为系统对比。验证常规／粗体／斜体／粗斜体、中文、emoji、100%／103%／110%／150%／200% 和真实 Qoder 未提交输入；不把用户对系统字体样例的选择冒充当前嵌入版本已经目视验收。

Qoder input 探针必须等待已知空输入框占位文本和稳定的未提交标记；仅收到 SessionStart 不足以输入，鉴权／信任页不得继续。此正文匹配仅用于测试就绪，不参与生产状态归约。未完成四帧或被鉴权阻挡时 result.ok=false，不自动登录或授权。

尺寸回归覆盖 terminal_interaction 的测量／未知视口，以及 App 的副窗口隔离（真实 ConPTY）。`qoder_cn_probe ... trace <isolated-directory>` 仅启动一次 CLI、103% 下四次程序化尺寸与最多 128 项布局记录；不发送提示或确认信任。鉴权／信任页截图不能证明真实模型回合通过，防御性布局回归也不能冒充用户重复页面现象已完整复现。

Windows 闪烁核查：构建并运行 render_update_probe（完整命令见 scripts/README.md），用两个无 profile 的本地 PowerShell 固定样例比较表格像素与可见网格哈希；另一块区域变换字号，浮窗和悬停提示由 island_hint_probe 单独核查。覆盖 100%／103%／110%／200% 和较长的 103% 运行，记录探针进程内存、退出码与自身 PTY 回收。result.ok 或进程退出失败均不得通过；受控像素复现与用户真实 Agent 验收分别记录，不调用模型。


0.4.10 默认 JetBrains Mono；新工作区、缺少／未知字体 ID 的设置使用默认，已有选择保持不变。typography_probe 未指定字体时使用正式默认。字体回归通过正式设置下拉选择 Hack，验证原 ConPTY PID、会话身份、保存和重载；typography_probe 新增 courier_new／jetbrains_mono，并用第四参数 settings-zh／settings-en 截图正式设置控件（第三参数 default）。五款字体分别检查 100%／103%／110%／150%／200%，缺失字体不能当作通过。安装验收只运行 -Acceptance 包，以相同 -AcceptanceId 配对旧／新版本；`project.ps1 installer-smoke -Path <new.json> -Previous <old.json>` 使用唯一 output 路径、HKCU 测试键和快捷方式，验证安装、占用拒绝、升级、包内就绪后六 PTY 截图、卸载及用户文件保留。不得对普通安装执行自动卸载。

启动器递归回归位于 tests/shell_sessions.rs：有界假 qodercn 分发到公开 qoderclicn，核对参数、真实退出码和单次开始／退出通知；旧逻辑最多重入一次后失败，不能用无界递归制造进程风暴。PATH 纯函数另覆盖大小写、斜杠、中文／空格和相似前缀目录。真实版本核查仅调用 --version，在隔离进程树中限时退出，不发送 Ctrl+C 或模型提示，不操作用户会话。

会话配色原生核查：`cargo build --offline --locked --release --example session_color_probe`，随后运行 `target/release/examples/session_color_probe.exe output/<unique-session-colors>`。使用正式侧栏和色盘，在 100%／103%／200% 截取选中、悬停、无颜色和收起状态，不启动 PTY／Agent、不注入系统输入。sidebar_interaction 验证全部色块与清除不切换会话、单排菜单以及悬停／选中底色一致且无独立描边；旧颜色 ID 往返另由 model 回归覆盖。

Git 面板核查：`cargo build --offline --locked --release --example git_probe` 后运行 `target/release/examples/git_probe.exe output/<unique-git-probe>`。探针创建自身独立样例仓库并使用正式 Git 后台、摘要、面板和真实 PowerShell ConPTY，截取 100%／103%／114%／200% 中英文更改、历史和收起状态，核对单行标题栏、Codicons 图标及 JetBrains Mono／Hack 字体继承，检查同一 PTY PID；不操作用户仓库、不调用 Agent／模型。git_repository 覆盖真实本地 Git 的净变化、二进制／大文件、新仓库／游离 HEAD、分支历史、分页、目录通知、忽略文件及输出限额；git_panel 核查正式控件，App 回归检查面板开合后终端身份和偏好保存。
