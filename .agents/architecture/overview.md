# 架构总览

当前版本以 Cargo.toml 为准，初始化基线为 0.1.2。Windows x64、Rust 2024、eframe/egui 0.35、
Alacritty 0.26；默认 glow/OpenGL，可选 wgpu/DX12 用于本机对照。依赖版本由 Cargo.lock 锁定。

| 边界 | 职责 |
| --- | --- |
| src/main.rs、app.rs | 原生窗口、会话列表、终端控件、后台事件归约 |
| src/appearance.rs | 正式界面与探针共用字体／主题 |
| src/status.rs | 生命周期状态、并行工具等待、过期回合与子会话过滤、未读 |
| src/bridge.rs、bin/ai-terminal-hook.rs | 认证 loopback 元数据桥与独立 hook helper |
| src/launch.rs | CLI 定位、会话级配置和启动环境 |
| src/model.rs | 会话元数据原子保存与加载 |
| src/platform.rs | Windows Job Object、主进程内存观测 |
| vendor/egui_term | Alacritty/ConPTY 接入、输入、字形与 viewport 渲染 |
| assets/fonts | 固定来源的 Hack 字体和许可证 |
| examples/terminal_probe.rs | 开发核查：真实 TUI、受控未提交文本、截图后退出 |

数据流：ConPTY I/O 线程 → Alacritty 网格 → 活动 viewport → GPU。
另一条独立路径为 Agent hook → 独立 helper → 认证 TCP → 有界队列 → App::logic → 侧栏提示。
UI 是否绘制、会话是否选中不能决定状态能否更新。

保留 src/、vendor/、tests/、examples/ 的现有布局。模块内测试留原处；跨组件 Rust 测试在 tests/。
管理层 Python 检查与包校验测试也放 tests/，不混进生产二进制。

已知边界：系统物理输入法的所有组合、图片粘贴、长期高并发会话与升级后的 CLI 协议仍需验收。
Claude fullscreen renderer 的兼容性尚未建立；现有证据包含其回退界面。
初始阶段没有远端仓库、自动发布服务或产品服务器，不添加臆测的部署配置。
详细选型历史保留在 docs/architecture.md，历史测量不能当作当前构建的性能证明。
