# AI终端程序图标

简体中文 | [English](README.en.md)

用户选择的 B「会话叠窗」方案，原始矢量与设计候选 b.svg 字节一致；原创图形使用项目 MIT 许可。

- ai-terminal.svg：矢量源。
- ai-terminal.png：256×256 RGBA，用于原生窗口图标。
- ai-terminal.ico：16、20、24、32、40、48、64、96、128、256 像素，各尺寸直接由矢量独立绘制，用于 Windows EXE 资源。
- SOURCE.json：来源、导出方法、大小与 SHA-256。

程序构建直接使用固定导出文件，无需浏览器或图片处理依赖。Windows 资源由 build.rs 调用 Windows SDK rc.exe 编译，链接主程序和调试示例程序，不链接 hook helper；主窗口、浮窗及可见调试窗口共用相同图标。图标内嵌于 EXE，不依赖外部文件或安装字体。

## 0.3.4 验证

- 治理、fmt、全目标 Clippy 与本次相关的 15 项 Rust／3 项 Python 检查通过。
- 优化版原生 smoke：`output/smoke-20261101-155248-137/`，正常退出且测试自身 3 个 PTY 回收。
- `output/brand-b-20261101-154347/verified.json`：Windows 资源 API 读取 EXE，10 个图标帧均与固定 ICO 逐字节一致，hook helper 未链接 GUI 图标。
- 对新窗口读取 WM_GETICON，实际取得大／小图标并查看截图；当前 150% DPI 对应 48×48 与 24×24，均为 B 方案。
- 已打开 `output/preview-v0.3.4-20261101-155258/ai-terminal.exe`，窗口标题 ai-terminal 0.3.4；独立数据位于 `output/preview-state-v034-20261101-155258/`，从上一版复制元数据，没有接管旧窗口进程。

以上证据目录被忽略，不加入源码或公开包；本次没有提交、打标签或远端发布。
