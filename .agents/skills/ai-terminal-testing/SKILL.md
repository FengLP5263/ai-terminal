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
