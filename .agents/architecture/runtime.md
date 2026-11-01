# 运行、配置与操作

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

每次终端启动在隔离 runtime/<session>/<generation>/bin 生成两份 helper 入口（优先硬链接、跨卷回退复制）。只对本次子进程 PATH 生效，不改用户／系统 PATH、CLI 安装或 profile。输入 codex/claude 调用真实已安装的 CLI，并报告 ShellAgentStarted／ShellAgentEnded；普通输出不决定图标。显式绝对路径、用户自定义别名和其他 Agent 暂不接入自动识别。

图标身份与工作状态分开：身份来自受管 CLI 启停，Working/Waiting/完成仍来自生命周期 hook。HookEvent 增加可选 invocation、process_id，旧事件默认 None。每次调用的 ID 隔离迟到事件和嵌套 Agent；当前进程句柄仅在根 shell 提示符事件上检查，不扫描进程列表。CLI 退出回到终端，保留用户自定义名称。

Claude 状态配置在同一终端启动代次内复用，仅内容变化时写入。新建终端不加载 Agent 或要求配置 hook。用户显式 --settings 保持优先，不覆盖该文件；此时状态连接以实际 CLI 事件为准。所有交互仍由用户处理，不自动信任或发起模型请求。

终端默认保持受控启动：PowerShell -NoProfile、CMD /D、Bash 使用会话 rcfile，不修改或自动载入用户启动文件。关闭终端仍只回收自身 Job Object。运行中的 CLI、控制台句柄、提示词和正文不持久化。

## 0.3.3 普通滚轮

普通滚轮只滚动本地终端显示，不再因 ALTERNATE_SCROLL 与 ALT_SCREEN 合成上下方向键。Codex 的受管入口附加其公开参数 --no-alt-screen，以内联模式保留正常终端滚动历史；无需改用户 Codex 配置。已运行的旧 Codex 需自行退出并重新启动才使用新模式。键盘方向键、状态 hook 和 Ctrl＋滚轮整体缩放维持原行为。其他全屏程序若不提供滚动历史，本地滚轮不虚构历史内容。

0.3.4 起窗口使用内嵌 256×256 RGBA B「会话叠窗」图标，EXE 包含 16–256 像素的 10 档 ICO 资源。图标无运行时外部文件依赖；调试示例 EXE 和可见调试窗口也使用同一 B 图标；hook helper 不链接 GUI 图标资源。

## 0.3.5 状态恢复

生命周期 Stop 对应 Completed，与未读分开；已读／当前会话仍显示绿点与「已完成」，下一轮 UserPromptSubmit 恢复 Working。Idle 表示会话就绪，不表示本轮完成。
同一 Agent 调用内的 SessionStart（source 为 resume／clear／fork）可以更新主会话 ID 并清除旧回合；迟到的其他会话事件与子会话 startup 仍被过滤。compact 保持正在工作的状态。
Codex 状态命令固定为 `& $env:AI_TERMINAL_HELPER --hook`，helper 路径从本次子进程环境读取。安装位置、预览目录和版本号不进入 hook 定义；没有写入 hooks.state，也不绕过 CLI 审查。旧版升级需由用户在 /hooks 审查新的定义；未连接时标题栏明确标识。

## 0.3.6 状态接入与全屏滚轮

顶部状态可点击查看 Agent 连接说明和复制 /hooks，不替用户输入、执行或信任命令。已识别 Agent 与已收到生命周期通知分别说明；状态仍只由任务事件驱动。
从另一个受管终端启动管理器时，CLI 发现及子 PATH 会排除继承的 AI_TERMINAL_SHIM_DIR，避免把旧会话的 helper 当作真实 Agent。helper 忽略带非空 agent_id 的子 Agent 事件；单独 agent_type（根会话 --agent）仍保留。
滚轮在应用明确开启 MOUSE_MODE 时使用真实鼠标滚轮协议，由 Claude 全屏界面等自行翻页；其余情况滚动本地历史。Shift 强制本地历史，Ctrl／Command 仍交给宿主整体缩放，不再模拟上下方向键。无需更改 Claude 用户配置或强制退出全屏。


## 0.3.7 原生 Codex 与图片粘贴

移除受管入口强制附加的 --no-alt-screen，让 Codex 保留默认全屏 TUI、启动 Logo、底部输入框和应用内部滚动。使用 --no-daemon 将引擎和其会话环境留在自身 Job 内，不操作用户已有的共享 daemon。当前在 Codex CLI 0.164.0 验证；用户主动传入的 CLI 参数仍由 CLI 解释。

Codex 欢迎页通常到首次提交才产生 SessionStart。此前显示「等待任务通知」，不以进程存在伪造就绪、进行中或完成。只有任务后仍无通知时才建议检查 /hooks；已 Active 的 hook 无需反复授权。状态命令定义保持不变。

Windows 的 egui-winit 补丁保留无文本剪贴板的 Paste 事件。只在已识别的 Codex／Claude 终端收到粘贴时查询图片格式元数据，存在图片时传递其共同支持的 Alt+V，由 CLI 读取与附加。Ctrl+V／Shift+Insert 是宿主入口，Alt+V 本身也继续按键盘协议传输。不读取像素、不生成图片文件、不轮询剪贴板；文字、Shell 与 UI 文本框不走图片快捷键。图片识别与编码错误仍由 CLI 显示。


## 0.4.0 设置与灵动岛

左侧顶部用标准齿轮按钮替代品牌文字，展开与收起状态都可进入设置。语言为跟随系统／简体中文／English；Windows 在启动时使用 GetUserDefaultUILanguage，其他未支持的系统语言回退 English，手动选择覆盖自动检测且立即生效。仅改变管理器控件与状态文字，不改变 CLI 进程、输出、用户名称和工作目录。

浮窗默认关闭，在设置中开启。原生 deferred viewport 使用透明背景、黑色紧凑头像栏、AlwaysOnTop、无标题栏和无任务栏入口。首次置于主窗口所在显示器工作区顶部；拖动浮窗空白边缘移动并保存位置，重新开启时按现有显示器工作区校正。全局缩放或会话数变化时在浮窗所在显示器横向居中，使用屏幕物理坐标和目标显示器 DPI 换算，保留物理垂直位置并同步重设原生视口尺寸。头像和状态角标复用侧栏样式，悬停显示名称与状态；最多同时展示 10 个会话，超出宽度时仅在按住会话区拖动时横向浏览。点击按 UUID 激活既有 PTY 并恢复／聚焦主窗口，不重启 Agent；通过设置关闭浮窗。主窗口正常退出仍由原有确认流程处理。

没有进行中动画或主动拖动时不持续重绘；进行中动画按约 80ms 间隔请求子窗口重绘，后台通知明确唤醒根窗口。根窗口即使最小化仍归约桥事件、更新浮窗快照和保存偏好，避免隐藏主窗口的光标／状态动画继续重绘。没有后台轮询剪贴板、进程树或终端内容。
