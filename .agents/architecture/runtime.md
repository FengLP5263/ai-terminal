# 运行、配置与操作

简体中文 | [English](runtime.en.md)

## 支持环境

已验证 Windows x64、Rust 1.99.1 MSVC、Visual Studio C++ Build Tools 与 Windows SDK。
Cargo.toml 声明最低 Rust 1.92；使用 stable 工具链，制品记录实际 rustc 版本，不宣称位级可复现。
需要 Python 3.11+ 执行治理／打包。POSIX 原生 GUI 构建尚未接入，跨平台脚本不代表产品已跨平台。

## 进程与配置

一个管理器，若干受管 CLI/ConPTY，以及短时 hook helper。每个会话一个 Windows Job Object，
关闭会话／程序只回收该会话自己拥有的进程树。禁止用进程名批量清理 PowerShell、Codex 或 Claude。

桥只监听 127.0.0.1 的动态端口，无固定服务端口。每帧上限 8192 字节、256 项队列，
满队列明确拒绝并由 helper 有限重试；连接和读写有短超时。token 只由管理器随机生成并传给子进程。

| 名称 | 来源与用途 |
| --- | --- |
| LOCALAPPDATA | 默认会话元数据根目录；缺失时回退临时目录 |
| PATH、APPDATA、LOCALAPPDATA、USERPROFILE、SystemRoot、ProgramFiles | 查找已安装的 CLI／Windows 组件；启动时补读注册表用户／系统 PATH |
| NPM_CONFIG_PREFIX、PNPM_HOME、NVM_SYMLINK、VOLTA_HOME、BUN_INSTALL | 可选现有安装工具的目录线索；不设默认值、不写用户环境 |
| AI_TERMINAL_ENDPOINT、AI_TERMINAL_TOKEN | 本轮管理器的 loopback 桥和认证，不能提交值 |
| AI_TERMINAL_SESSION、AI_TERMINAL_GENERATION | 本地会话标识与启动代次 |
| AI_TERMINAL_SHELL_INVOCATION、AI_TERMINAL_REAL_* | 当前嵌套 Shell 调用标识与已发现的真实程序路径，仅传入本次子进程 |
| TERM、COLORTERM、NO_COLOR、FORCE_COLOR、CLICOLOR_FORCE | GUI 启动前在本进程内规范交互终端颜色能力 |
| CLAUDECODE | 子会话避免继承父 Claude 的嵌套标记 |
| RC | 可选 Windows SDK 资源编译器路径，仅构建时读取；默认从 PATH／SDK 查找 |
| CARGO_HOME、RUSTUP_HOME | 可选本机 Rust 安装位置，由忽略的 .tools/env.ps1 提供 |
| AI_TERMINAL_PYTHON | 可选脚本 Python 可执行文件路径 |

不在仓库创建 API Key 模板：管理器没有独立模型接入，沿用 CLI 的既有认证和模型设置。
GUI 只改自己的进程环境，不改 Windows 用户／系统环境或 Warp。系统字体的默认 Windows 路径目前在源码中，
非默认系统盘／不同字体环境尚未验证，后续移植时处理，不能写成已支持。

## 启动、就绪、退出

`scripts/windows/project.ps1 run` 构建并打开交互窗口；点＋只选择 PowerShell、CMD 或 Git Bash；无需目录或 Agent 表单。默认用户主目录，随后沿用当前／最近的有效目录。
左侧会话可拖动排序；悬停铅笔或右键编辑名称，标题栏的面板按钮收起为头像栏，再次点击展开。
右键会话 → 会话颜色可选 7 种颜色或恢复默认；颜色随会话保存，当前行采用更亮的底色和文字，行间分隔线保持一物理像素。标题 AI终端 与两个按钮共用 32 逻辑像素的高度和中心。
面板右边缘可调宽度。窗口内 Ctrl＋滚轮缩放整个界面（50%–250%），Ctrl＋加减同样可用；Ctrl＋0 或标题栏比例按钮恢复 100%。终端跟随剩余内容区重新计算行列，普通滚轮保留历史滚动。
展开和收起侧栏共用固定 3 逻辑像素的滚动条，位于分割线内侧约 1 逻辑像素；悬停／滚动只改变明暗，不扩宽。列表右侧预留间隙，不覆盖头像或状态点，保留滚轮和拖动滚动；样式只作用于侧栏。展开时，分割线外侧的独立拖动区调整面板宽度（190–420 逻辑像素），避免抢占滚动条操作。
首次 Codex 在 /hooks 中审查状态 hook。打开窗口直接进入上次选中的 Shell；其余标签按单击打开，不批量启动。首次空工作区直接创建 PowerShell。
正常退出通过应用窗口；活跃任务先保存／确认。没有全局 stop/reset 命令，不删除用户会话数据。

readiness 是实际窗口／终端与状态消息，不是后台进程存在。应用没有日志采集服务；
隔离 smoke、性能测量与探针把 JSON/PNG 写到被忽略的 output/。首次 UI 标记不等于严格屏幕首帧。
默认检查和测试不调用模型；verify-claude 必须显式 AllowModelRequest，费用边界见项目测试技能。

Windows 文字资源由 TextRendererSession 管理，主程序与原生探针在 on_exit 中先释放；不能依赖线程 TLS 析构顺序。独立 hook helper 不初始化原生文字、字体或 GPU。

## Windows CLI 发现

0.2.2 按当前进程 PATH、注册表保存的系统／用户 PATH、当前用户常见安装目录的顺序查找公开的 exe/cmd/bat 入口，不扫描磁盘，不依赖 npm 包内部布局，不执行用户 profile。按目录保留优先级，同一目录优先 exe；空与相对 PATH 项不隐式指向用户项目。

查找仅在启动会话时发生；合并后的 PATH 只传入本次子进程，便于 npm 启动器找到 Node，不修改管理器或 Windows 的环境。仅 WSL 内的 CLI 与 PowerShell 函数／别名不属于可直接启动的 Windows 程序。

cmd/bat 由独立 ai-terminal-hook.exe 的 --launch-cli 模式通过 Rust Command 启动，不将参数拼成 cmd.exe 命令串。helper 使用 console subsystem 继承已有 ConPTY，仍不初始化字体／GPU；主程序仍是无控制台 GUI。退出码透传，helper、cmd 和 CLI 属于本会话的 Job Object。生命周期 hook 路径和认证协议不变。

## 0.3.0 终端优先

新会话以 shell 为根进程；0.3.2 起，旧 Codex/Claude 标签在加载时兼容为 PowerShell，保留名称、目录与历史 ID，不自动执行 Agent。Shell 提示符发出 ai-terminal-cwd;<generation>;<sequence>;<directory> 元数据，后台验证代次和目录后更新列表／默认目录。Bash 用内置 pwd -W 报告本机路径。目录更新只发生在提示符边界，不轮询。

每次终端启动在隔离 runtime/<session>/<generation>/bin 生成已接入程序的 helper 入口（优先硬链接、跨卷回退复制）；Shell 入口仅在发现对应安装后生成。只对本次子进程 PATH 生效，不改用户／系统 PATH、CLI 安装或 profile。输入 codex/claude 调用真实已安装的 CLI，并报告 ShellAgentStarted／ShellAgentEnded；普通输出不决定图标。显式绝对路径、用户自定义别名和其他 Agent 暂不接入自动识别。

图标身份与工作状态分开：身份来自受管 CLI 启停，Working/Waiting/完成仍来自生命周期 hook。HookEvent 增加可选 invocation、process_id，旧事件默认 None。每次调用的 ID 隔离迟到事件和嵌套 Agent；当前进程句柄仅在根 shell 提示符事件上检查，不扫描进程列表。CLI 退出回到终端，保留用户自定义名称。

Claude 状态配置在同一终端启动代次内复用，仅内容变化时写入。新建终端不加载 Agent 或要求配置 hook。用户显式 --settings 保持优先，不覆盖该文件；此时状态连接以实际 CLI 事件为准。所有交互仍由用户处理，不自动信任或发起模型请求。

PowerShell 使用 -NoProfile，CMD 使用 /D。Git Bash 的会话 rcfile 先加载 /etc/profile，再按 Bash 登录配置的优先级加载首个可读的 ~/.bash_profile、~/.bash_login 或 ~/.profile；是否加载 ~/.bashrc 由上述用户配置决定。保留用户 PS1、别名、补全和标量／数组 PROMPT_COMMAND，再追加目录通知；启动配置加载后仅将本会话的 Agent 入口放回子进程 PATH 前端。设置 CHERE_INVOKING 保留选中目录，按安装目录选择 Git 的 MSYSTEM。不写入用户配置文件；用户启动脚本仍会执行其自身行为。关闭终端仍只回收自身 Job Object。运行中的 CLI、控制台句柄、提示词和正文不持久化。

菜单中的 Git Bash 需要真正的 Git for Windows 安装：支持当前／持久 PATH（包括 Git 的 cmd 目录）、HKCU/HKLM 两种注册表视图中的 GitForWindows/InstallPath、环境变量提供的系统／用户安装目录及用户 Scoop 路径；便携安装需将 Git 的 cmd 或 bin 加入 PATH。不扫描磁盘，不把 WSL 的 bash.exe 别名或无 Git 配置的其他 Bash 当作 Git Bash。缺失时明确提示并提供改用 PowerShell 的按钮，PowerShell 也缺失则提供 CMD；点击后更新真实会话类型、默认名称和图标，保留 UUID、目录、颜色、自定义名称与对话 ID。没有可用回退时保留错误，不自动安装软件。

会话级入口还识别 powershell、cmd、bash 的交互式嵌套调用；按原参数启动程序，进入时发送 ShellStarted，结束时发送 ShellEnded。头像优先显示前台 Agent，其次是当前嵌套 Shell，最后是创建会话时的 Shell；exit 逐层恢复，侧栏和浮窗共用该身份。仅支持已接入入口和明确的交互式参数；单次 -Command／-File／/c／-c、重定向、Agent 工具内部的 Shell 不切换头像。启动时仅查询自身控制台的进程 ID，以确认同属根 PTY；不扫描系统进程，不读取输入或猜测输出。最多保留 32 层临时身份；过期代次、父调用不符、脱离当前控制台的通知被拒绝。嵌套身份不改变或持久化根 Shell 类型，也不表示 AI 任务开始／完成。原始启动参数保持不变，目录通知仍以实际提示符集成为准；绝对路径和自定义别名不会被伪装成已接入入口。

## 0.3.3 普通滚轮

普通滚轮只滚动本地终端显示，不再因 ALTERNATE_SCROLL 与 ALT_SCREEN 合成上下方向键。Codex 的受管入口附加其公开参数 --no-alt-screen，以内联模式保留正常终端滚动历史；无需改用户 Codex 配置。已运行的旧 Codex 需自行退出并重新启动才使用新模式。键盘方向键、状态 hook 和 Ctrl＋滚轮整体缩放维持原行为。其他全屏程序若不提供滚动历史，本地滚轮不虚构历史内容。

0.3.4 起窗口使用内嵌 256×256 RGBA B「会话叠窗」图标，EXE 包含 16–256 像素的 10 档 ICO 资源。图标无运行时外部文件依赖；调试示例 EXE 和可见调试窗口也使用同一 B 图标；hook helper 不链接 GUI 图标资源。

## 0.3.5 状态恢复

生命周期 Stop 对应 Completed，与未读分开；后台完成产生未读绿点，进入并查看会话后清除绿点，但保留「已完成」状态；下一轮 UserPromptSubmit 恢复 Working。Idle 表示会话就绪，不表示本轮完成。
同一 Agent 调用内的 SessionStart（source 为 resume／clear／fork）可以更新主会话 ID 并清除旧回合；迟到的其他会话事件与子会话 startup 仍被过滤。compact 保持正在工作的状态。
Codex 状态命令固定为 `& $env:AI_TERMINAL_HELPER --hook`，helper 路径从本次子进程环境读取。安装位置、预览目录和版本号不进入 hook 定义；没有写入 hooks.state，也不绕过 CLI 审查。旧版升级需由用户在 /hooks 审查新的定义；未连接时标题栏明确标识。

## 0.3.6 状态接入与全屏滚轮

顶部状态可点击查看 Agent 连接说明和复制 /hooks，不替用户输入、执行或信任命令。已识别 Agent 与已收到生命周期通知分别说明；状态仍只由任务事件驱动。
从另一个受管终端启动管理器时，CLI 发现及子 PATH 会排除继承的 AI_TERMINAL_SHIM_DIR，避免把旧会话的 helper 当作真实 Agent。helper 忽略带非空 agent_id 的子 Agent 事件；单独 agent_type（根会话 --agent）仍保留。
滚轮在应用明确开启 MOUSE_MODE 时使用真实鼠标滚轮协议，由 Claude 全屏界面等自行翻页；其余情况滚动本地历史。Shift 强制本地历史，Ctrl／Command 仍交给宿主整体缩放，不再模拟上下方向键。无需更改 Claude 用户配置或强制退出全屏。


已识别的 Codex／Claude 会话优先用普通左键拖动选择终端文字，Ctrl+C 复制选区；没有选区时 Ctrl+C 仍中断 CLI。应用鼠标模式下，按下后移动至少 3 逻辑像素才进入本地拖选，不向 CLI 泄漏点击／拖动事件；没有拖动的单击在松开时发送真实按下与释放。滚轮仍由应用处理。其他终端程序保留原有鼠标协议，可按住 Shift 强制本地选择。使用现有终端网格和选区，不新增对话记录。

## 0.3.7 原生 Codex 与图片粘贴

移除受管入口强制附加的 --no-alt-screen，让 Codex 保留默认全屏 TUI、启动 Logo、底部输入框和应用内部滚动。使用 --no-daemon 将引擎和其会话环境留在自身 Job 内，不操作用户已有的共享 daemon。当前在 Codex CLI 0.164.0 验证；用户主动传入的 CLI 参数仍由 CLI 解释。

Codex 欢迎页通常到首次提交才产生 SessionStart。此前显示「等待任务通知」，不以进程存在伪造就绪、进行中或完成。只有任务后仍无通知时才建议检查 /hooks；已 Active 的 hook 无需反复授权。状态命令定义保持不变。

Windows 的 egui-winit 补丁保留无文本剪贴板的 Paste 事件。只在已识别的 Codex／Claude 终端收到粘贴时查询图片格式元数据，存在图片时传递其共同支持的 Alt+V，由 CLI 读取与附加。Ctrl+V／Shift+Insert 是宿主入口，Alt+V 本身也继续按键盘协议传输。不读取像素、不生成图片文件、不轮询剪贴板；文字、Shell 与 UI 文本框不走图片快捷键。图片识别与编码错误仍由 CLI 显示。


## 0.4.0 设置与灵动岛

左侧顶部用标准齿轮按钮替代品牌文字，展开与收起状态都可进入设置。语言为跟随系统／简体中文／English；Windows 在启动时使用 GetUserDefaultUILanguage，其他未支持的系统语言回退 English，手动选择覆盖自动检测且立即生效。仅改变管理器控件与状态文字，不改变 CLI 进程、输出、用户名称和工作目录。

浮窗默认关闭，在设置中开启。原生 deferred viewport 使用透明背景、黑色紧凑头像栏、AlwaysOnTop、无标题栏和无任务栏入口。首次置于主窗口所在显示器工作区顶部；拖动浮窗空白边缘移动并保存位置，重新开启时按现有显示器工作区校正。全局缩放或会话数变化时在浮窗所在显示器横向居中，使用屏幕物理坐标和目标显示器 DPI 换算，保留物理垂直位置并同步重设原生视口尺寸。头像和状态角标复用侧栏样式，悬停显示名称与状态；最多同时展示 10 个会话，超出宽度时仅在按住会话区拖动时横向浏览。点击按 UUID 激活既有 PTY 并恢复／聚焦主窗口，不重启 Agent；通过设置关闭浮窗。主窗口正常退出仍由原有确认流程处理。

没有进行中动画或主动拖动时不持续重绘；进行中动画按约 80ms 间隔请求子窗口重绘，后台通知明确唤醒根窗口。根窗口即使最小化仍归约桥事件、更新浮窗快照和保存偏好，避免隐藏主窗口的光标／状态动画继续重绘。没有后台轮询剪贴板、进程树或终端内容。

Windows 主窗口标题栏固定使用深色。普通窗口跨显示器拖动时，松开鼠标后依据鼠标所在显示器的工作区校正越界；先等待 DPI 同步，再按需移动或缩小窗口。最大化窗口由 Windows 管理；同一显示器内的拖动不会被校正。仅检查鼠标左键状态和拖动结束时的指针位置，不记录输入。

## 需要处理状态的信号

Codex 的 PermissionRequest 在自动审批前也会触发，不能单独证明用户看到审批。受管 Codex 进程使用会话级 `tui.notifications`、`tui.notification_method` 和 `tui.notification_condition` 参数接收实际用户提示的 OSC 9；只识别已知提示前缀，不保存通知文本，并按运行代次去重。显式传入的后续 CLI 配置可以覆盖这些参数。部分 Codex 提示类型或未来版本未必发出可识别通知；已有进程需要重新启动才能应用新参数。

Claude 的 PermissionRequest／Elicitation 本身也可能被其他 hook 自动处理；实际权限与交互提示由 Notification 确认，权限提示通常等待约六秒才发出。PermissionDenied 与 PostToolBatch 用于处理拒绝或批次完成后的状态恢复。带显式 `--settings` 的 Claude 仍遵循用户设置，不注入本项目 hook。权限提示解决后，若 CLI 未提供关闭事件，状态可能要等对应工具或批次结束才能恢复；不能把这当成完整的实时状态协议。

## 多会话退出

点击关闭后先确认，不会提前终止会话。确认窗口使用活动终端的最近快照，不等待后台输出持有的网格锁，终端输出仍继续排空。确认退出后在后台保存会话元数据，并以最多四路并行回收本程序拥有的 Job 与 PTY；界面显示完成数量并保持响应。保存或回收失败时保留窗口并提示，可重新确认退出；不扫描或结束无关进程。异常强制终止不承诺完成保存。
## Windows OpenGL 缓冲区

原生字形绘制可能在部分 Windows OpenGL 驱动中触发进程私有内存持续增长。项目维护的 `egui_glow` 补丁按需扩容并复用顶点和索引缓冲区，保留原有纹理、绘制顺序及字形内容；其他平台使用上游方式。内存回归应区分管理器和 Agent 子进程，并使用隔离会话及真实原生窗口验证。

## Windows 兼容性修复（未发布）

默认随包使用 Microsoft.Windows.Console.ConPTY 1.25.260930003 的 x64 conpty.dll 和 OpenConsole.exe。Windows 10 内置控制台在原生样例中丢失全屏／鼠标模式，同一程序改用固定组件后通过。组件与程序同目录，不修改系统控制台、默认终端、PATH 或用户配置；会话创建前缺失组件会明确报错。build.rs 按实际 Cargo profile 为程序、examples 和 deps 准备组件，相同内容不重写。

深色标题栏通过当前窗口的 DwmSetWindowAttribute 请求，优先属性 20、再尝试旧 Windows 10 属性 19。窗口子类在 WM_THEMECHANGED／WM_SETTINGCHANGE 的默认处理结束后重新应用，在 WM_NCDESTROY 移除；没有空闲轮询或全局个性化更改。不支持该属性的窗口保留后端主题。

CLI 发现跳过已确认目标缺失的标准 npm .cmd／.bat 启动器，继续按原目录优先级查找。不运行探测命令，不猜测包私有布局：仅检查公开标准启动器中明确的字面目标。自定义、无法理解或不可读取的脚本保留优先级；不能因 CLI 返回非零状态就再次执行另一入口。全部标准入口失效时，错误列出启动器与缺失目标；会话中缓存的入口也在实际调用时复核。

界面修复：收起列表选中框为正方形；滚动条在鼠标离开侧栏后隐藏，宽度和预留空间不变。状态颜色由 chrome 统一。浮窗位置使用已有子窗口的当前 DPI 换算，子窗口 DPI 变化后重设逻辑尺寸，避免跨屏裁切。Codex 的代理入口缺失来自独立模式，非底部裁切；边界与核查见 [界面渲染](../../docs/ui-rendering.md)。
收起侧栏会话块固定为 44×44 逻辑像素，不从滚动区当前宽度回推行高；否则在内容刚好溢出时，滚动条显隐会触发持续尺寸跳变。几何回归须覆盖临界数量、多帧静止、缩放及悬停，空闲时不得持续请求立即重绘。

preview.3 浮窗位置记录不再回写为每帧定位命令。仅创建、缩放或会话数改变时主动定位；`floating_native.rs` 在自身 UI 线程缓存并检查自己的窗口，读取原生物理矩形，处理原生移动开始／结束、DPI 与尺寸通知。后端处理后合并投递客户区尺寸核对，拖动期间推迟主动定位，销毁时释放子类状态；不轮询其他窗口或进程。收起侧栏的三个工具按钮与会话统一 44×44 占位、32 点圆形底座、20 点图标与 4 点间距。

preview.4：本地 winit 补丁在原生移动期间直接采用 WM_DPICHANGED 的系统建议矩形，保留窗口相对鼠标的锚点，避免推回旧屏幕触发递归缩放。浮窗额外尺寸修正全部推迟至 WM_EXITSIZEMOVE 后；主窗口不另加拖动中校正。展开侧栏的会话卡片两侧各保留约 8 逻辑像素，滚动条仍在分割线内侧。

## 0.4.5 Codex 原生状态与 agents 总览

默认受管 `codex` 不再注入 `--no-daemon`、状态 hook 或通知 `-c` 参数，保留 CLI 自身的共享后台选择与 `← for agents`。状态改由当前 PTY 的明确 OSC 7501 生命周期通知提供：idle、working、blocked（permission／question／auth）、done、error；clear 只清理当前终端状态，不算完成。已收到原生状态后，迟到的旧 hook／OSC 9 不覆盖它。协议通知按调用代次隔离，最多保留 32 条状态、合并唤醒 ROOT，隐藏窗口仍由 logic 消费；不读取正文判断状态，不增加轮询。

未提供该协议的旧 Codex 可显式运行 `codex --no-daemon`，继续注入原有会话级 hook，按 `/hooks` 审查且没有共享总览。不自动查询 CLI 健康、不重试模型命令、不修改用户配置或信任。用户传入的 profile／覆盖参数仍由 Codex 决定是否使用共享后台。原生协议不携带对话 UUID，不据此伪造或覆盖已知恢复 ID；对话恢复及切换由 Codex 的 resume／agents 完成。共享后台由 Codex 管理，退出本程序只回收自有终端及 helper，不停止用户共享服务，其中的任务可能继续运行。

## 会话未读与拖动排序

失败红点与完成绿点都是未读标识：查看会话或已聚焦会话收到结果时清除角标，保留失败／完成状态。等待用户处理的黄点与进行中动画仍表示当前状态。Agent 失败、终端启动失败和进程异常退出都使用同一未读语义，侧栏和浮窗复用绘制逻辑。

侧栏拖动只投影元数据顺序：前景卡片保持鼠标按下时的相对位置，原列表预留完整空位，其余卡片让位；目的位置根据固定行槽计算，避免移动命中框引起往返抖动。松手才按 UUID 修改实际会话顺序；Esc 或列表外松手取消。展开与收起共用该行为，不改变 PTY、活动会话或状态。仅按住卡片且接近溢出列表边缘时继续滚动，取消后无空闲重绘循环。

## 0.4.6 Codex 直接启动与 Qoder CN 会话入口

Shell 启动后即可观察严格匹配的 Codex 原生状态协议；收到带 `app=codex` 的状态才建立运行时身份。因此通过绝对路径、别名或 `--remote … resume …` 启动也可接入，无需改写命令或注入 hook。无 app 身份的 clear、普通输出和 OSC 9 不建立身份，其他已识别 Agent 不被覆盖。clear 可能是临时终端交接，不据此判断退出；未经过 helper 的 Codex 在本代 Shell 的提示符通知到达后恢复 Shell，重置通知代次并继续观察后续调用。既有对话 UUID 保持原样。

在普通 Shell 输入 `qodercn` 或 `qoderclicn`，本会话 helper 转发真实 CN CLI 并按调用 ID 报告启动／退出；头像和状态快照同时用于侧栏与浮窗，不改会话名称。用户明确传入 `--settings` 时由其配置决定任务 hook；没有收到事件时保持等待通知，不假定正在工作或完成。缺少 CLI 时显示发现错误，不自动安装、登录或修改权限。

本机 0.4.6-preview.1 核查：166 项 Rust、13 项 Python 回归及治理／格式／Clippy 通过，Release smoke 与七种合成状态截图已检查。Qoder CN 1.1.82 的真实启动已完成鉴权并显示当前目录的信任确认；探针未确认该提示，未收到 SessionStart，因此不将其记录为真实 hook／模型回合已验证。信任与后续任务验收由用户在 CLI 中进行。

0.4.6：用户已验收 Qoder 预览；最终 168 项 Rust、13 项 Python 回归、治理／格式／Clippy、Release smoke 与原生截图通过。正式 App 已核查真实 Codex 0.166.0 的绝对路径／--remote unix:// 启动、头像、就绪和 agents 入口；恢复参数、等待／完成、clear、返回 Shell 和重复调用另由真实 ConPTY 的伪 CLI 覆盖。没有发送模型提示，不能将两类证据称为真实模型回合通过。
