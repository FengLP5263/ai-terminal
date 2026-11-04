
[简体中文](UPSTREAM.zh.md) | [English](UPSTREAM.en.md)
Source: https://github.com/kemokempo/egui_term
Revision: 31bbc7ab8503c9518fcee5717cfa29011e59f451
License: MIT (LICENSE preserved).

Unreleased: fit complete native emoji clusters inside their cell bounds, including
narrow text-default pause/play badges. Preserve VS15 text presentation and the
emulator's original cell coordinates. See the paired UPSTREAM records.

Local changes: viewport-only snapshots; 5000-line scrollback; no background redraw;
coalesced output wakeups; receiver-disconnect termination; per-child env and Windows
argument escaping; IME commit support; keyboard focus independent of pointer;
bracketed paste; selection copy from the live grid; respect hidden cursor.

Color palettes and default key bindings are cached and shared. Rendering does not
rebuild mappings, clone color strings, or parse hex colors per cell. Theme OSC color
queries are answered for CLI compatibility. IME candidate positioning is supplied.

Terminal row height accounts for the CJK fallback face, preventing mixed Chinese
and Latin output from overlapping adjacent rows.

Input/display repair: stable widget identity; focus is requested only when needed
(repeated requests cancel native IME); terminal owns Tab/arrows/Escape; composition
suppresses raw candidate keys; literal Text events are never parsed as key names.
Cell widths round up rather than truncate. Paint backgrounds before glyphs; retain
combining marks and concealed cells; use native cursor shape, wide cursor position,
and dynamic terminal colors; clip all drawing to the terminal pane.

Draw common box rules and block/quadrant glyphs as cell geometry to prevent seams.

0.1.2 typography: cell geometry is in logical points quantized on the physical
DPI grid, ConPTY receives physical cell pixels, selection uses the same float
metrics. Glyphs share a baseline instead of centering each character vertically;
real bold/italic faces are selected by terminal flags, with placeholder-color
galleys reused across ANSI colors. Application embeds licensed Hack faces.

## 0.2.0：清晰度与尺寸稳定性

- 默认纯黑背景和中性灰文字；字体尺寸与网格／基线共同对齐物理像素。
- 用混合字形 ink bounds 与 1.2 倍字号计算行距，避免 CJK fallback 行框撑高全部行。
- ConPTY 只在行列或字符物理尺寸变化时收到 resize，微小布局变化只更新本地布局。
- Ctrl＋滚轮由宿主局部消费后更新 TerminalFont，普通滚动、IME 和 PTY 输入保留。

## 0.2.1：Windows 原生文字与完整界面缩放

- 宿主在 raw_input_hook 消费 Ctrl＋滚轮／加减／0，调整 egui 全窗口比例。
- TerminalFont 可指定进程内字体集合；Windows 路径用 DirectWrite 字形布局及 Direct2D/WIC 生成 atlas，启用系统彩色字体；不安装字体到系统。
- 中文指定 Microsoft YaHei UI；中英文按统一基线与物理像素绘制，采用灰度抗锯齿避免 RGB 彩边。
- NativeFont 缓存最多 64 种格式／度量、每个上下文 8 张 1024² atlas，线程缓存最多两个上下文；按需栅格化，不在后台轮询。
- 按 Unicode grapheme 合成肤色修饰、ZWJ 和旗帜序列，保留模拟器的原始单元格坐标。VS16 爱心在紧随空格时使用完整两格画面。
- 原生图块使用实际背景／光标颜色，插入光标在文字之后绘制。
- TextRendererSession 必须在 on_exit 释放缓存，在窗口／COM 拆卸之前销毁原生资源；Drop 为补充保护。
- Windows 原生路径失败或其他平台保留 epaint 回退；回退不代表支持彩色 emoji。

0.3.0：将专用 ai-terminal-cwd; OSC 标题事件转发给宿主，用于有代次验证的工作目录更新；其他标题仍不用于推测 Agent 身份或任务状态。

0.3.2：为宿主名称／编辑器暴露 TerminalFont::emoji_shape，复用同一引擎与有界图集，以系统 Segoe UI Emoji 完整布局组合序列。UI 图块保留 PBGRA 透明度并按预乘颜色上传；终端的不透明背景保持相同语义。没有额外的字体引擎缓存或后台轮询。

0.3.3：普通滚轮始终调用 Term::scroll_display，不再为 ALTERNATE_SCROLL＋ALT_SCREEN 生成 ESC OA／OB 方向键；保留真实键盘、指针与 Ctrl 缩放处理。宿主的 Codex 入口同时使用公开的 --no-alt-screen 保留正常网格滚动历史。

0.3.6：滚轮改为独立 MouseWheel 命令，在后端当前模式下路由：应用启用 MOUSE_MODE 时发送标准 SGR／传统鼠标滚轮事件，普通终端使用本地历史。Shift 强制本地历史，Ctrl／Command 留给宿主缩放。鼠标位置采用屏幕网格，单事件最多发送 32 次；仍不合成方向键。保留平滑点滚动的余量和有界历史，鼠标协议按实时模式选择。

0.3.7：TerminalView 可由宿主为已识别的编码 CLI 开启 native_image_paste。收到 Paste 时仅查询 Windows 图片格式；图片转发 Alt+V，CLI 自行读取与附加，文本继续使用原有的 bracketed paste 和换行规则。没有像素复制、临时图片、剪贴板写入或后台查询。Codex 宿主已恢复默认全屏 TUI，不再强制 --no-alt-screen。


0.4.0：PTY 元数据／退出与重绘事件明确请求 ROOT viewport 重绘；管理器新增独立 deferred 状态浮窗后，不能依赖当前正在绘制的 viewport 来唤醒根 logic。终端 I/O、隐藏输出排空、鼠标协议与有界历史保持原有实现。

Managed Codex invocations: a bounded, session-scoped OSC 9 observer forwards only recognized attention-notification metadata; content is discarded. The read/write wrapper preserves the PTY stream and coalesces repeated notices. No ordinary output or bell is used to infer completion.

输入响应优化：无输入的帧不再重复复制网格快照，尺寸不变时不获取网格锁；输入先入 PTY 队列，相邻键盘字节按顺序合并为有界批次。

应用退出确认期间允许绘制最近的终端快照而不等待网格锁，后台输出继续排空；取消后恢复实时画面。

输入法焦点恢复：窗口或终端失去焦点、显式恢复焦点时清理未提交的组合状态，避免失焦期间的取消事件被跳过后吞掉后续按键；普通重绘保留正在输入的候选。真实 ConPTY 回归覆盖失焦时取消、返回后输入与提交。

编码 CLI 的拖选：宿主可启用 set_drag_selection，在应用鼠标模式下将普通左键拖动用于本地选择；移动阈值为 3 逻辑像素，无拖动单击在释放时转发按下／释放。Shift 可强制本地选择，普通终端程序的 1002／1003 拖动协议保持可用；滚轮独立路由。复制选区不向 PTY 发送 Ctrl+C，无选区保留中断。真实 ConPTY 回归覆盖标准／SGR 鼠标、点击、拖动、复制及中断；原生探针截图不改写系统剪贴板。

Windows 兼容性验收：为宿主探针公开重导出 Alacritty 的 WindowSize，以核查最终 ConPTY 行列／物理字符尺寸。宿主在程序目录随带固定版微软 conpty.dll／OpenConsole.exe，使用 Alacritty 既有动态加载路径；本补丁不改其 ConPTY 协议实现。

Native Codex lifecycle: observe only recognized OSC 7501 state/app/kind values, preserving byte-for-byte PTY output. A 32-state queue preserves recent transition order, coalesces wakeups and rejects old invocation epochs. No status text, prompts or activity details are retained. Adjacent narrow mode badges share a one-cell visual budget; only an isolated badge can extend into its following blank, preventing unequal triangle raster scales.
