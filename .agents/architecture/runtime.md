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
| AI_TERMINAL_NSIS | 可选固定 NSIS 编译器路径，仅构建安装版时读取；不影响产品运行 |
| AI_TERMINAL_PYTHON | 可选脚本 Python 可执行文件路径 |

不在仓库创建 API Key 模板：管理器没有独立模型接入，沿用 CLI 的既有认证和模型设置。
GUI 只改自己的进程环境，不改 Windows 用户／系统环境或 Warp。系统字体的默认 Windows 路径目前在源码中，
非默认系统盘／不同字体环境尚未验证，后续移植时处理，不能写成已支持。

## 启动、就绪、退出

`scripts/windows/project.ps1 run` 构建并打开交互窗口；点＋只选择 PowerShell、CMD 或 Git Bash；无需目录或 Agent 表单。默认用户主目录，随后沿用当前／最近的有效目录。
左侧会话可拖动排序；悬停铅笔或右键编辑名称，标题栏的面板按钮收起为头像栏，再次点击展开。
右键会话 → 会话颜色单排提供无颜色及红、绿、黄、蓝、粉、紫；颜色随会话保存。选中与悬停共用强调底色和文字亮度，普通行使用柔和底色，卡片不另描边，行间分隔线保持一物理像素。标题 AI终端 与两个按钮共用 32 逻辑像素的高度和中心。
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

发布前，用户另已实测确认 Codex 旧会话恢复无问题并授权提交发布。该验收独立于开发侧启动截图和合成事件；不据此扩大长期负载或其他平台的验证范围。

## 0.4.7：补充 CLI 身份与首次快捷方式

受管 Shell 的入口目录新增 `kimi.exe`、`zcode.exe`、`traecli.exe`，对应 `AI_TERMINAL_REAL_KIMI`／`AI_TERMINAL_REAL_ZCODE`／`AI_TERMINAL_REAL_TRAE`。Shim 只转发真实程序并按调用 ID 报告 ShellAgentStarted／ShellAgentEnded，不注入参数、不复制或修改用户配置；任务状态保持“等待任务通知”，原因与各自 hook 限制在状态下拉中说明。

默认数据目录首次启动时，在应用主线程之外尝试创建桌面快捷方式：通过 `IShellLinkW`／`IPersistFile` 指向当前 `ai-terminal.exe`，使用 Windows Desktop 已知文件夹（支持 OneDrive 等重定向），不携带参数或凭据。`UiSettings.desktop_shortcut` 在尝试前记录，失败不重试、不阻塞启动；指定非默认目录的 `--data-dir`、隔离 smoke／性能测量、工作区读取失败时不尝试，用户删除后不自动重建。创建失败只写入 stderr，不改动桌面以外的位置。

## 0.4.8 预览：新增 CLI 的明确任务通知

Kimi 受管入口仅对子进程设置 `PI_PROGRAM_STATUS=1`，显式已有值优先；观察 `app=kimi-code` 的 OSC 7501 根记录。与 Codex 共用 32 项、按 app 去重及调用代次隔离的队列；可选 title／msg 当场丢弃，带 id 的子记录不参与根状态。匿名 clear 仅清理已连接状态，不建立身份、判断退出或完成。状态仍在后台 logic 消费；Kimi 与 Codex 原生状态均拒绝迟到 hook 覆盖。

TRAE 以本次 `-c hooks.<event>=…` 接入开始、工具、Notification、Stop、Interrupt 与退出；稳定命令通过进程环境引用轻量 helper，用户在 `/hooks` 自行审查，后续显式参数由 CLI 决定。TRAE 未导出的任务失败不靠终端文本猜测，远端服务不保证可执行本机 hook。

社区 zcode-app-cli 的公开 `ZCODE_CLI_SETTINGS_FILE` 替换整份基础设置，不能当作追加配置。仅当没有显式该变量、原设置不大于 64 KiB 且通过无凭据基础选项检查时，在调用自己的 runtime 目录生成快照并注入 type=process 的六种 hook。已有 hook、自定义列表／命令、MCP／插件定义、未知字符串或敏感字段拒绝快照，原样启动 CLI 并保留等待通知。不会读取 provider／凭据文件、复制自定义 hook 或修改用户／项目文件。快照内的设置修改仅本次调用有效；ZCode 当前未导出可靠授权等待、失败与中断，不补猜测。

0.4.8-preview.2：悬停文字通过浮窗 UI 线程创建的原生 tooltip 显示。窗口置顶、无激活、鼠标穿透，按头像位置、当前物理缩放和显示器工作区定位并换行；移开／拖动时隐藏，浮窗销毁时释放窗口与 GDI 字体。只在悬停内容／位置变化时更新，不轮询或持续重绘。0.4.8：ZCode 的开始／工具恢复直接显示 Working，问题显示 Waiting，Stop 显示 Completed；失败与退出按明确事件归约。移除 Unconfirmed 降级及其角标；没有后续事件时保留最后一次 hook 状态，过期调用／子事件继续拒绝，不设置超时或读取错误正文。

0.4.9 预览：Qoder 的自动审批／不提问模式中的 permission_prompt 不再表示需要用户处理；显式问题与 MCP 交互仍等待。手动权限等待按 tool_use_id 关联；最近已结束的 32 个工具 ID 阻止迟到通知重新置黄，新 SessionStart 清空缓存。未知模式保留原有提示语义，不解析通知正文。普通／私有光标查询分别返回 CSI 行;列 R／CSI ? 行;列 R，不将协议回复当作用户文本。

0.4.9-preview.4：原生多窗口的主终端尺寸和后台可见性仅由根窗口决定；布局测量、不可见、已标记废弃或缺少有效 screen_rect 的帧不提交 ConPTY resize。真实窗口调整仍有效。该保护的合成回归、原生单次 Qoder 启动与用户重复页面现象的完整复现分别记录，不以一项通过替代另一项验收。

0.4.9-preview.5 的原生受控复现与真实 Agent 分开：旧的逐网格同区域上传在 48 次固定表格截图中有一帧文字拉扯，新的整帧上传对应 48 次无像素变化。滚动后可恢复的用户闪烁仍需在真实会话中验收；不把此修复写成模型／CLI 协议变更。

0.4.9 正式发行验收：用户实际测试 preview.5 后确认无问题并授权正常发行；保留已选定的 Cascadia Mono／微软雅黑 UI、固定会话数据根目录和旧窗口。受控渲染复现、656 帧像素核查与验收边界见 [0.4.9 渲染核查](../../docs/rendering-0.4.9.md)。


0.4.10 预览：设置默认 JetBrains Mono，可选 Cascadia Mono、Hack、Consolas、Courier New，选项仅显示字体名称，仅影响终端正文。选项立即刷新字体／单元格度量，所有运行会话保留原 ConPTY、身份和状态；已保存的字体选择保持不变；旧设置缺失／未知字体 ID 回退默认，系统字体缺失时禁用选择并提示回退。安装版默认 `%LOCALAPPDATA%/Programs/AI Terminal`，数据仍为 `%LOCALAPPDATA%/ai-terminal`。升级／卸载不结束现有程序，文件占用时拒绝修改；`.ai-terminal-installed` 将桌面快捷方式管理交给安装器。原生 smoke 在六个固定样例的就绪标记后截图，此正文检查仅存在隔离测试中，不参与生产任务状态。

0.4.11 预览：`run_agent` 在原调用与嵌套调用中都从子进程 PATH 过滤自己的会话入口，按 Windows 路径分量比较大小写、分隔符和末尾斜杠；不删除前缀相似的其他目录，不改父进程或持久 PATH。这样 qodercn 公共包装器查找 qoderclicn 时会到达真实安装，避免多层批处理及 Ctrl+C 终止确认堆积。

## Git 工作区

活动终端的目录驱动 GitClient 的单一后台工作线程；请求／响应各保留最新一份，有界唤醒队列与代次取消拒绝旧目录结果。Git 通过既有发现目录查找，只使用绝对入口；本次子进程禁用可选锁、fsmonitor、hook、外部 diff、textconv、内容过滤器、延迟对象获取、传输协议和交互提示，清除继承的仓库定位变量，不写用户配置或自动绕过 safe.directory。每条命令限 6 秒、2 MiB，读取上限与展示规则见 [Git 面板](../../docs/git.md)。

工作树、Git 目录及公共 Git 目录使用可取消的重叠 ReadDirectoryChangesW，64 KiB 对齐缓冲区、最多 64 个待检查路径、250ms 安静窗口和持续变动时最多 1 秒合并；Git check-ignore 过滤构建缓存事件；元数据通知只关注 HEAD、索引、分支引用和相关配置，忽略日志、锁文件及对象写入。首次监控附加后再次读取，避免丢失发现期间的变化；没有空闲定时扫描或持续重绘。目录变化和用户刷新直接查询；健康监控下提示符、焦点和面板打开复用快照，失效时这些操作最多每两秒补查一次，后台结果明确唤醒 ROOT；监控不可用时可手动刷新。收起面板继续更新摘要，切换会话只跟踪当前目录，关闭只取消自身 Git 查询及通知句柄。

0.4.12-preview.3：git_ui::header 为正式 App 和探针提供固定单行布局，名称／路径／Git 元数据按可用宽度缩略，不加第二行；字体家族来自当前终端字体设置，字号继承 Body 样式，缩放只经既有全局 UI zoom。分支符号改用独立 Codicons 字形；ui_text::label_with_font 保留组合 emoji 的原生覆盖绘制。

0.5.0 为上述会话配色与 Git 预览功能的正式发行；0.4.12-preview.N 仅为既有本地验收编号。标题栏及 Git 面板跟随应用设置选定的终端字体。后续按 docs/versioning.md 的版本规则定版。

终端右侧历史条使用固定 12 逻辑像素边距、3 逻辑像素滑块，悬停只改变亮度。拖动或点击轨道定位；无历史或备用屏幕时隐藏，不模拟键盘、滚轮协议或 Agent 内部历史。最大 5000 行。Git 差异打开时不向终端路由输入，关闭后恢复原焦点和尺寸；后台状态与输出继续处理。

文件语法高亮仅由已打开的提交文件触发，使用库内置语法与项目颜色，不查找项目或用户目录下的语法配置，不加载插件或运行代码。后台生成／缓存结果并复用；关闭或目录切换取消旧代次。超出语法软预算或无法识别时保留普通文本；不改文件缩进、工作目录、会话进程或字体。

preview.4：GitView 的当前文件指示只从 GitClient.patch 的提交 ID／路径派生，不由悬停或提交详情查询改变，不持久化。文件正文严格 UTF-8 校验，首行 BOM 不参与代码词法解析；源数据已经保存成乱码时保持原文，非 UTF-8 文本明确报错，不做系统代码页猜测。

preview.5：ui_text::layout_job 在保留语法 TextFormat 与原始字符串的同时，为完整 emoji 字素建立透明字形槽；Git 文件视图复用 paint_emoji 的既有原生有界图集。语法段不得拆开组合序列；显式 VS15 文本变体保留文字颜色，不新增字体资产、后台扫描或重绘。

preview.6：Actions.file 与 GitView 当前文件键使用 PatchSource 加路径，历史文件与工作区同名文件互不混淆。GitClient 只接受当前快照列出的本地文件，重复点击复用结果；快照的事件／手动刷新结果重新提交当前工作区读取并保留旧正文，改回干净时关闭视图。UTF-8、命令 6 秒／2 MiB、20,000 行／每行 4,000 字符与取消代次保持；未跟踪内容有界读入内存，路径不得越界或经过链接／junction。没有新的轮询与空闲重绘。

0.8.0 预览：floating_scale 与 ui_scale 分开保存。旧记录缺失字段用 0 哨兵反序列化，normalize 首次继承已规范化的 ui_scale；新默认值为 1.0。浮窗以独立比例计算实际布局和命中区域，字体与头像纹理按最终物理像素生成。原生尺寸与位置按浮窗比例和当前显示器 DPI 处理，主窗口缩放不再重新居中浮窗。提示文字直接按局部比例布局以保留原生 emoji 栅格清晰度。raw_input_hook 只消费对应 viewport 的缩放操作，不改变其他 viewport 的比例或终端输入。

0.8.0-preview.2 的文件语法选择优先匹配完整文件名／扩展名，再处理固定别名与当前内存中的首行 shebang；扩展语法和 Oniguruma 均来自锁定依赖。准备工作仍在同一后台查询中完成，旧新解析状态、取消代次、每行／总 token／时间预算不变，不引入新的文件监控、解释器或用户配置加载。

0.8.0-preview.3 移除浮窗绘制图层变换，以浮窗比例／主窗口比例缩放布局几何，字体和图片在最终物理像素下生成。角标同步缩放；悬停锚点转换回原生定位用的基础单位。根窗口缩放／PTY 生命周期不变。设置 show 返回本帧偏好变化，App 立即应用语言、字体与浮窗选项并沿用自动保存；关闭设置不承担提交操作。

Windows 原生程序支持 `ai-terminal.exe --settings`，直接显示同一个即时设置窗口；`--data-dir` 可按既有规则同时指定。该参数只影响本次窗口展示，不保存为偏好；smoke／idle 核查忽略它。

0.9.0 预览：ui.project_files_open 默认 false，与 git_panel_open 互斥；普通目录使用会话工作目录，有仓库信息时使用 Git 根目录。同一会话在 Git 内 cd 保留根目录，切换会话／项目取消旧查询。展开、手动刷新和重新获得焦点按需读取，不增加周期扫描。文档视图隐藏终端绘制／输入，保留后台排空、ConPTY 身份和关闭后的焦点恢复。

0.9.1 预览：顶部项目入口显示标准文件夹图标和 project_files.root 的名称，悬停给出完整路径；右侧移除重复根目录行，操作与页签共用单行。长名称／页签按可用宽度省略，保持缩放、刷新触发、数据和终端生命周期不变。

0.9.1-preview.4：顶部状态移动到项目入口右侧、缩放左侧；普通文件标题的图标与文件名相邻并左对齐，长路径仍省略并支持悬停。文件图标按当前视口 DPI／缩放渲染，不依赖系统图标字体；不改状态来源、保存格式或终端生命周期。

0.10.0 预览：ui.floating_opacity 保存浮窗底板不透明度，默认 1.0，有限值约束到 0.2–1.0，非有限值回退 1.0；旧配置缺失字段沿用默认。设置以 0%–80% 背景透明度呈现，修改立即保存并仅在数值变化时唤醒浮窗。底板使用原生透明视口的 alpha，头像、角标与独立提示视口保持原样；不改窗口尺寸、位置、缩放或终端进程。普通文件／Git 差异标题从右侧布置实际宽度的操作区，左侧名称按剩余宽度省略。

0.10.0-preview.2：WGL 配置经 glutin-winit 降级为不透明时，仅设置绘制 alpha 不会启用桌面合成。floating_native 在自身 HWND 建立后启用 DWM 的逐像素透明，并在 WM_DWMCOMPOSITIONCHANGED 经窗口后端处理后恢复；不设置 WS_EX_LAYERED，不改系统个性化或其他窗口。预览／测试使用现有 `--data-dir <独立目录>`，不使用未实现的 AI_TERMINAL_DATA_DIR 环境变量；运行后核对实际保存的配置路径。

0.10.0-preview.3：目录首次加载／刷新时才执行一次 check-ignore --stdin -z，复用单个文件后台线程、Git 的 6 秒／2 MiB 上限和取消代次，输入另限 1 MiB；不递归扫描、不增加定时刷新。保留索引语义、嵌套规则和 Git 全局忽略；查询失败保留普通样式。浮窗在创建／合成通知时对自身 HWND 禁用非客户区绘制、系统圆角和描边，WM_NCCALCSIZE 返回完整客户区以绕过框架阴影的 1 像素边距；保留 DWM alpha，不改其他窗口或系统主题。

0.10.0-preview.4：项目文件夹入口位于 Git 摘要左侧，后接状态和缩放；保留既有宽度上限、窄屏省略、悬停与点击逻辑。

0.10.0-preview.5：设置、重命名、关闭会话与退出确认使用统一的紧凑字号／间距；确认框正文最多 360 点，操作按钮最小 88×36 点。所有尺寸仍是应用缩放前的逻辑单位，不反向抵消用户缩放、不改终端文字或保存格式。
