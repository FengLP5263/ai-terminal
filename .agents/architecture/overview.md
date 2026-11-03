# 架构总览

简体中文 | [English](overview.en.md)

当前版本以 Cargo.toml 为准，初始化基线为 0.1.2。Windows x64、Rust 2024、eframe/egui 0.35、
Alacritty 0.26；默认 glow/OpenGL，可选 wgpu/DX12 用于本机对照。依赖版本由 Cargo.lock 锁定。

| 边界 | 职责 |
| --- | --- |
| build.rs | Windows 图标资源编译，链接主程序及调试示例，helper 保持独立 |
| src/main.rs、app.rs | 原生窗口、会话列表、终端控件、后台事件归约 |
| src/sidebar.rs、zoom.rs | UUID 会话导航、拖动排序、头像和全窗口缩放 |
| src/appearance.rs、chrome.rs、ui_text.rs | 正式界面与探针共用字体／主题，以及统一的头像、矢量按钮、紧凑桌面尺寸和名称的彩色 emoji |
| src/status.rs、connection_ui.rs | 生命周期状态、并行工具等待、过期回合与子会话过滤、未读及连接说明 |
| src/bridge.rs、bin/ai-terminal-hook.rs | 认证 loopback 元数据桥与独立 hook helper |
| src/discovery.rs、launch.rs、shell.rs | 公开 CLI 与 shell 入口、会话级集成、目录通知、Agent 启停元数据 |
| src/model.rs | 会话元数据原子保存与加载 |
| src/platform.rs | Windows Job Object、主进程内存观测 |
| vendor/egui_term | Alacritty/ConPTY、输入、字形与 viewport 渲染；Windows 原生文字、组合 emoji 和有界 atlas |
| vendor/egui-winit | 与 eframe 相同的 0.35.0；仅保留无文本剪贴板的粘贴请求 |
| assets/fonts、assets/icons、assets/branding | 固定来源的 Hack 字体、Agent 图标，以及原创 B 方案程序图标和来源校验 |
| examples/terminal_probe.rs、workspace_probe.rs | 开发核查：真实 TUI 或本地 ANSI 样例、受控输入、GPU 截图后退出 |
| examples/bash_probe.rs | 真实 Git Bash 与隔离 HOME，启动配置、目录通知、Agent 入口和原生截图核查；不请求模型 |
| examples/shell_switch_probe.rs | 正式 App 控件的嵌套 Shell 头像与逐层返回截图；隔离会话，仅向自身窗口输入，不启动 Agent／模型 |

数据流：ConPTY I/O 线程 → Alacritty 网格 → 活动 viewport → GPU。
另一条独立路径为会话级 CLI 启停／Agent hook → 独立 helper → 认证 TCP → 有界队列 → App::logic → 侧栏提示。
Shell 提示符通过带启动代次和序号的专用 OSC 标题通知当前目录，不扫描磁盘、进程列表或终端历史。
UI 是否绘制、会话是否选中不能决定状态能否更新。

保留 src/、vendor/、tests/、examples/ 的现有布局。模块内测试留原处；跨组件 Rust 测试在 tests/。
管理层 Python 检查与包校验测试也放 tests/，不混进生产二进制。

已知边界：系统物理输入法的所有组合、所有图片剪贴板来源、长期高并发会话与升级后的 CLI 协议仍需验收。
Claude fullscreen 已验证启动及其 MOUSE_MOTION／SGR 滚轮协议；完整长期交互兼容性仍需验收。
源码托管于 [GitHub](https://github.com/FengLP5263/ai-terminal)，使用 MIT 许可证；贡献入口见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
Windows 预览包通过 GitHub Releases 分发；尚无自动发布服务、云 CI 或产品服务器。
详细选型历史保留在 docs/architecture.md，历史测量不能当作当前构建的性能证明。


0.4.0：src/i18n.rs 管理系统／中文／英文偏好；settings.rs 提供设置窗口；floating.rs 使用同进程的 deferred viewport 构建透明、置顶、圆角胶囊。跨窗口仅共享会话元数据快照，通过有界 32 项消息通知根窗口，不共享或复制终端内容。根窗口在最小化时只执行 logic；所有 PTY 和桥通知明确唤醒 ROOT，避免子窗口活跃时任务事件无人归约。

## Windows 兼容性修复（未发布）

`src/window_frame.rs` 为应用自己的窗口请求深色 DWM 标题栏，并在原生主题／设置通知处理后恢复该策略；`src/native_console.rs` 在创建会话前检查随包组件。`assets/windows/conpty/` 保存固定版本的微软原生控制台、许可证与来源。Alacritty 使用程序目录的组件，build.rs 为正式程序、测试和示例准备相同版本。

`examples/composer_probe.rs` 覆盖恢复至 151.13608% 与随后 125%、150%、151.13608% 缩放的全屏输入区；`titlebar_probe.rs` 截取自身原生窗口及标题栏；`discovery_probe.rs` 仅调用已安装 CLI 的公开 --version。合成样例、公开版本查询和真实模型回合分别记录。

`vendor/winit` 固定 0.30.13 的 Apache-2.0 本地快照。Windows 原生移动期间采用系统的 DPI 建议矩形，避免上游旧屏幕推回逻辑导致重复 DPI 变化；主窗口与浮窗共用该路径。非拖动路径保持上游实现。来源、归档哈希与范围见该目录 UPSTREAM.zh.md。
