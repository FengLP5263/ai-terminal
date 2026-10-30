# 0.1.2 字体与颜色：REA 对比及修复记录

2026-10-30。用户提供 ai-terminal 与 Warp 的对照截图，指出字体、颜色、显示畸变。
本轮使用 reverse-engineer-anything 技能和实际 REA MCP，只读检查本机 Warp 制品；
没有对 Warp 进程注入、调试、修改设置或读取会话内容。

## REA 证据与边界

Warp 安装目录 `C:/Program Files/Warp`。主程序 390372712 字节，SHA-256：
`b50810c5e6f86f6d277ab90744444cdbc3cbda2160f38c1b6063d44195bfa5d2`。

- 目录清单：`ev_a010a77e6442bf8e6364068a8537d0e482baebdc9588021d65a168c98bd9b3cf`。
  扫描 231 个条目、12 个文件头、3496 字节；该范围没有 Electron 候选，不能据此证明任意嵌入代码不存在。
- PE 导入：`ev_c394b41df4c89a41ef1b3a19dd4464b663e6d2d392b56ae1d54e39567afcd29d`。
  完整表 41 DLL / 670 导入，包含 OpenGL WGL、GDI 和 DXGI。静态导入不能证明当前运行选择的后端，也不能直接证明字体渲染算法。
- 保留清单视图：`ev_3a3d169a8382d3a4e4a86b47e1369c6b169903f79ec0276de2a080af7d220695`，
  定位到 `resources/settings_schema.json`。四条 Evidence 导出在 `output/rea-warp-visual-20261030.json`（SHA-256 4a932412bf70b5d1d5c63fe8430daaa2baad2a6e1df27ce3168c3b923559f044）。

随后读取已定位的 schema 和用户 settings.toml 中的字体/缩放字段：schema 默认 font_name=Hack，
font_weight=normal，line_height_ratio≈1.2，ligatures=false；当前 settings 的 font_size=12、window zoom=125。
没有显式 font_name 覆盖，推断沿用默认；这不是运行时字形 face 的直接观测。
REA 的 native UI/PTY capture 在当前版本要求 macOS/Linux，本机 Windows 未调用不支持的动态图形接口。
运行时验证使用本项目已有的 Windows ConPTY + 应用 GPU 截图。

## 已解决的问题

1. 运行环境观察到 NO_COLOR=1、TERM=dumb，Windows 用户和系统级 NO_COLOR 均未设置。
   它们来自启动器进程环境，导致 CLI 主动抑制颜色。现在在应用入口、创建线程之前清理本进程的无色标记，
   并声明 xterm-256color / truecolor；没有改注册表环境变量，也没有强行重画不存在的 ANSI 颜色。
2. 旧版用整数逻辑宽度，在 150% 缩放下形成半像素字符间距；每个字符独立居中导致基线和位置不稳定。
   现在网格在物理像素上取整，逻辑坐标保留小数；ConPTY 的像素尺寸、绘制和选择使用一致的度量。
   每个字符按共同基线绘制，避免逐字垂直居中；字形位置按实际 DPI 对齐。
3. 内置官方 Hack v3.003 四个字体文件，真实粗/斜体由终端属性选择，字体含完整原许可证。
   中文沿用系统回退，UI 使用 Segoe UI + 中文回退。常用颜色与明暗层级统一。
4. 正式界面与探针现在调用同一 appearance 模块，避免测试字体与产品字体不同。

Hack 官方来源与许可证：[source-foundry/Hack v3.003](https://github.com/source-foundry/Hack/tree/v3.003/build/ttf)。
制品来源与各文件 SHA-256 见 `assets/fonts/SOURCE.json`。NO_COLOR 语义参考 [no-color.org](https://no-color.org/)。

## 验证与未覆盖项

- fmt / 全目标 Clippy -D warnings / 36 项测试 / release 优化构建通过。
- 新测试覆盖 100% / 125% / 150% / 200% 物理像素网格、共同基线、真实粗体与粗斜体；
  之前的中文组合、特殊键、编辑与后台状态回归保持通过。
- Codex 实际 TUI：`output/typography-codex-20261030-142305`，测试文本可见，71 个非默认前景字符。
- Claude 实际 TUI：`output/typography-claude-20261030-142305`，测试文本可见，384 个非默认前景字符。
  两者均使用受控 IME/Text 事件，没有发送 Enter 或发起模型请求；PNG 已人工检查。
- 最终优化版主程序冒烟：`output/smoke-20261030-143105`；真实三个 PowerShell + 合成生命周期事件。
- 当前 Claude 自身提示此机 fullscreen renderer 已回退；本轮验证的是其回退界面，没有擅自更改此配置，
  Claude fullscreen 兼容性仍未建立。系统物理输入法的全部按键组合仍需实际使用验证。

本机预览 `output/preview-v0.1.2/ai-terminal.exe`，同目录 build.json 含校验与字体资产身份。
不把旧版本的内存测量当作新版测量，不宣称已达到 Warp 的全部终端功能或视觉实现。
