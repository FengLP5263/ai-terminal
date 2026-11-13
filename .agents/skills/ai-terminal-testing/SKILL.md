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

公开 README 截图用 Release 的 readme_capture 与 island_probe 分开生成；命令见 scripts/README.md。readme_capture 的参数为 `<output> [zh|en] [history]`，分别捕获中英文“更改”和“提交历史”；历史模式仅用自身 egui 合成点击展开固定样例提交；复用正式侧栏／顶部摘要／Git 面板／终端／浮窗组件，只用固定合成文本、示例目录及模拟 Git／任务状态，不查询用户仓库、不读取用户 profile、不启动 Agent 或模型。核对正常退出、自身 PTY 回收、原生 PNG 和隐私后原样复制，更新 assets/readme/SOURCE.json；不能用公开展示样例代替真实 Git 或 Agent 验收。

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

Git 历史详情：git_probe 扩展为 12 张图，向自身 egui 注入合成点击以展开真实隔离提交的文件，并捕获中英文悬停卡片；不注入系统输入、不改剪贴板。git_panel 检查完整 ID 的 CopyText 输出、指针移入提示卡片、复制不触发展开、长摘要单行缩略及展开／收起。git_repository 覆盖根／空／合并提交、二进制、中文与重命名、正文、外部程序禁用、重复读取复用及旧目录结果取消；原生截图与自身 PTY 回收另行核对。

preview.2：git_panel 比较更改与展开文件的分类字形，并在 100%／114%／200% 核对图标左缘与提交说明一致、读取状态不移动列表。git_repository 检查重复提示符／焦点式请求复用快照、手动刷新仍有效、监控失效补查限频，以及 Git 日志／锁文件不触发完整重读但真实文件和分支变化仍更新。git_probe 的真实提交样例包含多种文件类型，仍生成 12 张原生 PNG。

0.7.0 预览：history_probe 命令见 scripts/README.md，真实无 profile PowerShell 固定输出、自身 egui 拖动生成 100%／103%／200% 顶部／中间／底部九张图。terminal_interaction 验证 SGR all-motion 下轨道／拖动／区域外松手不发送输入，以及 5000 行上限、viewport-only 快照及历史出现前后列数稳定。git_probe 增至 16 图，点击隔离提交文件并查看差异；git_repository 检查提交字节、字面路径、行号、根／合并、二进制、驱动禁用、限额、复用与取消；git_panel 验证只读、虚拟化、标准红绿背景／标记及代码文字保持正常颜色。file_view 回归核对中间改动前后的首／末行、连续行号及仅权限变化的完整正文。单独核对实际 PNG、正常退出及自身 PTY 回收，不冒充真实 Agent 验收。

preview.3：syntax 单测验证 JSON 键／值／数字／布尔、Markdown 标题、内联／围栏代码、未知类型回退、Tab 停靠及删除注释不污染新版本。git_panel 核对缩进线、语法色与红绿背景独立、正文无 @@／file mode；git_probe 增至 20 图，追加中英文 JSON／Markdown 在 100%／200% 的原生截图。检查实际图像及同一 PTY 正常回收；语法截图不作为 Agent 模型回合验收。

preview.4：file_view 编码回归使用真实隔离 Git 检查 UTF-8 BOM、中文／重音／emoji、已有乱码原样保留及无效字节明确失败；opened_file 回归核对持续指示、悬停不迁移、切换／关闭和相同路径不同提交隔离。git_probe 的 JSON 样例带 UTF-8 BOM 与正常中文，继续生成 20 张图，人工核对中文与持续选中标记；实际用户历史只读核查仅记录字节摘要，不复制正文或修改关联仓库。

preview.5：ui_text 回归检查语法段切开 ZWJ／肤色／变体时的完整槽位、源字节不变、文字配色和显式文本变体；git_panel 检查增删行都产生原生彩色覆盖层。git_probe 增至 22 张图，Markdown 表格包含火焰、禁止标志、组合 emoji 与文本变体，覆盖 100%／103%／114%／200% 和 JetBrains Mono／Hack；必须目视检查原生颜色与基线，并回收自身 PTY。

preview.6：git_repository 以真实隔离 Git 验证暂存＋未暂存的净差异、索引／文件字节不变、未跟踪／无 HEAD／删除后重建、字面中文路径、二进制、无效编码、限额、过滤器禁用、复用与刷新／取消。git_panel 覆盖“未跟踪”标签、点击 WorkingTree 来源、持续选中和只读。git_probe 增至 26 图，追加中英文工作区新增／修改／二进制及 100%／103%／114%／200% 截图；仅操作自己的仓库和 PTY，不运行模型。

0.7.0 README 原生捕获新增 readme_capture 的第三参数 diff：使用固定 TSX 旧／新增行和正式只读文件组件，正文由现有语法准备生成，不查询用户仓库；英文模式所有展示文字使用英文。普通终端样例先写入固定历史，核对右侧滚动条。检查中英文主图、历史、文件差异和浮窗八张 PNG 后原样复制并更新 SOURCE.json；展示内容不能替代真实 Git、滚动交互或模型验收。

0.8.0 独立缩放：settings_floating 验证旧比例一次迁移、保存重载、不同主窗口／浮窗比例下的原生尺寸请求和实际控件命中；App 回归检查浮窗输入不改变主窗口比例、ConPTY PID 或网格。floating_scale_probe 用正式浮窗和提示，在六组主窗口／浮窗比例下生成 18 张原生图，并核对独立尺寸、提示大小及缩放后头像点击。island_hint_probe 和 island_move_probe 的旧缩放场景须显式同时设置浮窗比例，不以全局缩放耦合冒充独立设置。git_panel 检查 U 的实际悬停说明、简化面板首行及非仓库顶部隐藏。

0.8.0-preview.2：git::syntax 检查常见语言／特殊文件名、真正 TS/TSX、PowerShell 参数／环境变量／命令／引号／Here-String／多行注释、旧新状态、未知类型及取消。shell_icons 检查三种标准字形与缩放。git_syntax_probe 用正式只读差异组件在 100%／150% 下生成 16 类固定样例的 32 张原生图；不读用户仓库、不执行脚本或模型。许可回归拒绝版本漂移、篡改和 ZIP 缺失许可。目视检查截图，计数不能代替验收。

0.8.0-preview.3：settings_floating 验证头像纹理及标准字形使用最终物理像素，覆盖不同主／浮窗比例与实际点击；检查语言／字体／浮窗滑块即时生效、无完成按钮、宽窄中英列对齐及全局样式不变。floating_scale_probe 的 18 图改含三种 Shell 与三种 Agent，并在根窗口独立绘制同尺寸 Shell 参考图；核对像素差异、尺寸、定位与点击。settings_probe 生成 10 图，覆盖中英、宽窄、150%、浮窗禁用、重命名与两种确认框，不管理真实进程或调用模型。

0.9.0 项目文件核查：`cargo build --offline --locked --release --bins --example project_files_probe`，再运行 `target/release/examples/project_files_probe.exe output/<unique-project-files>`。12 张原生图覆盖文件树、中文／emoji、JSON、Rust、脚本、UTF-16、空／大／二进制／无效编码和返回终端；仅使用自建样例和无 profile PowerShell，Job Object 只回收自身 PTY，不执行查看的代码或请求模型。project_files 回归验证惰性、限额、刷新、取消、过期结果与 junction；project_ui 验证实际文件夹／文件／切换按钮及字体图标；App 回归验证文档下输入隔离、后台排空及原 PID／输入恢复。截图与真实用户验收分别记录。

0.9.0 公开截图新增 `readme_capture <output> [zh|en] files`，在自己的输出目录建立固定示例，复用正式文件／Git 共用面板和只读正文；顶部使用固定示例路径，不读取用户项目。逐张核查中英文项目文件、主界面、Git 历史、差异和浮窗十张 PNG、当前文件标记、文件夹图标、语法与 emoji，原样复制并更新摘要。

0.9.1-preview.3：运行 file_icons_probe（命令见 scripts/README.md）核对三字体／三比例的 Seti 原生矩阵，另用 readme_capture files／history 核查文件树、Git 行和标题。离线回归覆盖语言区分、文件名优先、大小写、未知／二进制回退、全部声明字形存在及许可证随包。合成名字不代表真实项目／模型验收。

0.9.1-preview.4 复用 file_icons_probe 的三字体／三比例矩阵和 readme_capture 的正式视图截图。补 SVG 可渲染／非空、尺寸缓存上限／颜色隔离、文件标题相邻左对齐、顶部状态顺序及许可／来源防篡改回归。原生截图与模型调用分开，探针不启动模型。

0.10.0 预览：settings_floating 检查透明度旧字段默认、边界／非有限值、保存恢复、设置未关闭时生效、复用既有 deferred 回调更新底板 alpha、头像不透明和点击区域不变。floating_opacity_probe 命令见 scripts/README.md：固定 100%／50%／20% 不透明度及中英文设置，生成 8 张真实 PNG；检查透明通道、同一原生 HWND／几何和透明时的合成头像点击，不注入系统输入或调用模型。project_ui／git_panel 验证右边缘操作布局，缩放后先等待布局稳定再读取上一帧响应。

0.10.0-preview.2：floating_opacity_probe 在自身置顶白色底板内读取桌面合成像素，补充 4 张 desktop PNG，共 12 张；必须验证白色参照、0%／50%／80% 透明度的桌面 RGB 变化、头像桌面颜色保持、合成状态重建恢复和原有 HWND／几何／点击。只验证 framebuffer alpha 不足以通过。readme_capture 文件样例含灰色 .gitignore。启动可供用户试用的预览时，必须显式传 `--data-dir`，核对保存目录及版本，不能仅凭进程存活或未实现的环境变量声称隔离成功。

0.10.0-preview.3：project_files 使用真实隔离 Git 覆盖嵌套／例外／全局／info-exclude 和已跟踪文件，project_ui 验证灰色条目仍可展开、只读查看。floating_opacity_probe 在 136% 下捕获浅色及灰色底板和窗外 16 像素，检查客户区与 framebuffer 合成差异、窗外无描边／阴影、同一 HWND／几何和透明点击；继续产生 12 张 PNG。readme_capture files 的自建仓库加入忽略文件及文件夹。必须目视检查原生 PNG，不把合成输入当作真实鼠标拖动。

0.10.0-preview.5：settings_probe 增至 14 张原生图，补齐中英文关闭／退出和 133%／150% 退出确认；检查紧凑文字层级、按钮与宽窄布局。继续运行 dialogs、settings_floating 的真实控件交互回归，确认取消／退出选择、中文 emoji、即时保存与全局样式隔离。

会话分组核查：构建并运行 session_groups_probe（命令见 scripts/README.md），检查十二张原生图及 result.json，覆盖 100%／142%／200%、中英文、首成员折叠、数量徽标、跨窗口悬停、成员点击和移出关闭。session_groups 测试覆盖旧数据、原子保存、无效引用和唯一归属；floating 单测覆盖排序／首成员移除及一次关闭计时。App 实际 ConPTY 回归核对组员切换只清除自身未读，分组／解散不重启终端。合成输入不代表真实 Agent 模型回合验收。

分组子菜单回归：sidebar_interaction 在中英文及 75%／100%／133%／200% 下验证主菜单和分组子菜单的所有命令单行且不溢出，长组名保持一行省略。session_group_menu_probe 从正式侧栏右键打开分组菜单，在 100%／133%／200% 输出中英文六张 PNG（命令见 scripts/README.md）；必须目视检查。仅自身样例及合成输入，不操作用户终端。

工牌／边框核查：session_groups_probe 输出十二张 PNG，补充两组工牌、首位中性分隔、190 点窄侧栏、收起侧栏和边框开／关；result.json 检查同一 HWND／尺寸／位置，开关只改变边缘像素。sidebar_interaction 核查组名及黑白样式一致、无角色标记、改名和收起行为；settings_floating 核查旧设置默认开启、真实复选框和持久化；App 真实 ConPTY 测试确认开关不重启终端。floating_opacity_probe 明确关闭边框以隔离既有透明度／圆角像素测试，默认开启与开关另由上述探针核查。

0.11.0-preview.4：sidebar_interaction 检查工牌白底黑字、组名一致且无“主”／L 标记；中性首位分隔线由原生图核对。floating_opacity_probe 增至十五张图，覆盖不透明度 0.0（100% 背景透明）及头像点击，核对 alpha 为 0、桌面背景透出、头像仍不透明、同一 HWND 和无外沿阴影。范围／持久化与实时 0.0 更新由 settings_floating 验证。

0.11.0-preview.5：sidebar_interaction 核对完整工牌文字左右各 6 点留白；floating 的实际绘制回归覆盖三档比例的白底黑字组名首字角标。floating_group 的正式内容组件回归覆盖中英文、三档比例、组名 × N、普通／悬停／选中背景及成员点击。session_groups_probe 的未选中普通成员带粉色底色，PNG 内像素核对该背景确实存在；继续目视核对十二张图及无激活／点击／边框结果。

0.11.0 README 更新：readme_capture 使用固定三成员分组和正式侧栏工牌；session_groups_probe 追加 `en` 可用全英文成员与组名进行同样的十二图核查。公开组图选 case-0-members.png，与中英文主界面／历史／差异／文件／浮窗／设置共十四张 PNG 原样复制并更新 SOURCE.json。所有图片均为隔离样例，不读取用户分组或调用模型。

0.11.0 浮窗系统外框回归：floating_scale_probe 在六组比例下验证真实 HWND 的标题栏／菜单／边缘位均为空；向自有浮窗模拟后端恢复装饰样式及 WM_NCPAINT，确认过滤后仍无系统外框、主窗口样式不变。继续核对十八张原生截图、尺寸、头像像素与点击；floating_opacity_probe 验证原 alpha、合成恢复和边缘。只操作探针自己的窗口，不向用户窗口注入输入。
