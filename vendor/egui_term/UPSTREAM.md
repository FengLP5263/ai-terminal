Source: https://github.com/kemokempo/egui_term
Revision: 31bbc7ab8503c9518fcee5717cfa29011e59f451
License: MIT (LICENSE preserved).

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
