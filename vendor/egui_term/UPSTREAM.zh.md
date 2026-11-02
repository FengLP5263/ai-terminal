# egui_term 上游与本地补丁

简体中文 | [English](UPSTREAM.en.md) · [原始中英混排记录](UPSTREAM.md)

上游：https://github.com/kemokempo/egui_term

修订：`31bbc7ab8503c9518fcee5717cfa29011e59f451`

许可：MIT；保留原始 LICENSE。

本地变更：仅复制视口快照；5000 行历史；后台不重绘；合并输出唤醒；接收端断开时终止；子进程环境和 Windows 参数转义；IME 提交；键盘焦点独立于指针；bracketed paste；从实时网格复制选区；尊重隐藏光标。

调色板和默认键绑定缓存共享；渲染不按单元重建映射、克隆颜色字符串或解析十六进制。响应主题 OSC 颜色查询以兼容 CLI，提供 IME 候选位置。终端行高考虑 CJK 回退字体，避免中英文相邻行重叠。

输入显示修复：稳定控件 ID，仅在需要时请求焦点（反复请求会取消 IME）；终端处理 Tab／方向键／Esc；组合期间抑制原始候选按键；Text 事件不解析为按键名。单元宽度向上取整，先绘背景再绘字，保留组合和隐藏字符，使用原生光标形状、宽光标位置和动态颜色，并裁剪到终端面板。常见框线、块和象限按单元几何绘制，避免接缝。

0.1.2 字体：逻辑单元按物理 DPI 网格量化，ConPTY 接收物理单元像素，选择使用相同浮点度量。字形共享基线，终端标志选择真实粗／斜体，在 ANSI 颜色之间复用占位色 galley；应用内嵌授权 Hack 字体。

## 0.2.0：清晰度与尺寸稳定性

- 纯黑背景、中性灰文字，字号、网格和基线共同对齐物理像素；按混合字形 ink bounds 和 1.2 倍字号计算行距，避免 CJK 行框撑高全部行。
- 仅在行列或字符物理尺寸变化时调整 ConPTY；轻微布局变化仅更新本地布局。该版宿主局部消费 Ctrl＋滚轮更新 TerminalFont，保留普通滚动、IME 与 PTY 输入。

## 0.2.1：Windows 原生文字与完整界面缩放

- 宿主在 raw_input_hook 消费 Ctrl＋滚轮／加减／0 进行全窗口缩放。TerminalFont 可指定进程内字体集合；Windows 使用 DirectWrite 和 Direct2D/WIC 图集及系统彩色字体，不安装字体。
- 中文使用 Microsoft YaHei UI；中英文共享基线、对齐物理像素，使用灰度抗锯齿。NativeFont 每上下文最多缓存 64 项格式／度量、8 张 1024² 图集，每线程最多两个上下文；按需栅格化，不后台轮询。
- 按 Unicode grapheme 合成肤色、ZWJ、旗帜，保留模拟器原始坐标；VS16 爱心后有空格时占满两格。原生图块使用实际背景／光标颜色，插入光标后绘。
- `TextRendererSession` 必须在 `on_exit`、窗口／COM 拆卸之前释放，Drop 为补充。Windows 原生失败或其他平台回退 epaint；回退不代表支持彩色 emoji。

0.3.0：向宿主转发 `ai-terminal-cwd;` OSC 标题，带代次验证以更新目录；不从其他标题猜测 Agent 身份或状态。

0.3.2：向名称／编辑器提供 `TerminalFont::emoji_shape`，复用引擎、有界图集与 Segoe UI Emoji 完整布局组合序列。UI 图块保留 PBGRA 透明度并按预乘颜色上传；终端不透明背景语义不变，不增加字体缓存或后台轮询。

0.3.3：普通滚轮调用 `Term::scroll_display`，不再为 ALTERNATE_SCROLL＋ALT_SCREEN 生成 ESC OA／OB；真实键盘、指针与 Ctrl 缩放保留。该版 Codex 入口使用公开的 `--no-alt-screen` 保留滚动历史，0.3.7 已替换。

0.3.6：独立 MouseWheel 命令按后端模式路由：MOUSE_MODE 开启时发送 SGR／传统鼠标滚轮，其他情况滚动本地历史。Shift 强制本地历史，Ctrl／Command 留给宿主；位置采用屏幕网格，单事件最多 32 次，不合成方向键。保留平滑点余量、有界历史与实时鼠标协议。

0.3.7：宿主可为已识别编码 CLI 启用 `native_image_paste`。收到 Paste 仅查询 Windows 图片格式，转发 Alt+V 由 CLI 附加；文本保留 bracketed paste 和换行规则。不复制像素、不创建临时图片、不写剪贴板或后台查询。Codex 恢复默认全屏 TUI，不再强制 `--no-alt-screen`。

0.4.0：PTY 元数据、退出与重绘事件显式请求 ROOT 视口重绘；新增 deferred 浮窗后，不能依赖当前正在绘制的视口唤醒根 logic。终端 I/O、隐藏输出排空、鼠标协议和有界历史不变。
