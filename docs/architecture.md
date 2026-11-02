# 架构决策：Windows 原生优先

简体中文 | [English](architecture.en.md)

Rust 负责应用、PTY 与状态桥。egui/eframe 默认通过 glow/OpenGL 绘制轻量 GPU 界面，保留可选的 wgpu/DX12 构建用于 A/B 测量。终端复用 Apache-2.0 的 Alacritty 解析器和 ConPTY 后端；固定版本的 MIT egui_term 快照提供输入与渲染控件，本地性能和输入补丁在其目录内记录。

管理器因此不需要浏览器引擎和 JavaScript 运行时。这不意味着 Agent 本身轻量；其进程和内存开销独立计算。性能结论需测量优化版。

## 热路径

ConPTY → I/O 线程上的 Alacritty 解析器 → 最多 5000 行历史。可见终端只把视口单元复制到复用缓冲；隐藏终端继续解析输出，但不请求重绘。输出唤醒合并到 16 ms 重绘期限。侧栏活动动画运行于 10 Hz，空闲会话不安排动画。不扫描目录、对话正文或进程。

## 状态路径

Agent 生命周期 hook → `ai-terminal-hook.exe --hook` → 仅元数据的认证 loopback TCP 消息 → 有界队列 → UI 状态 reducer，与当前选择无关。每次启动都有新 generation ID，过滤过期进程事件；Agent session 和 turn ID 拒绝子会话与上一轮事件。重复投递不能重复改变状态。确认未读结果不改变活动状态。

Hook 返回 `{}`，不批准、拒绝或修改工具调用。通过 Claude 的 `--settings` 和 Codex 单次调用配置注入会话，不写用户级 Agent 配置、API Key 或信任记录。Codex 需要审查非托管 hook；收到事件前提示状态未连接。ConPTY 独立检测进程退出；管理器为每个会话拥有一个 Windows Job，关闭会话会关闭其受管后代。

仅原子保存会话名称、目录和实际 Agent 恢复 ID。重启后恢复列表；本段记录的是早期 Continue 启动恢复命令的设计，当前启动行为以运行规范为准。程序退出后不恢复终端历史和运行进程。

## 可见指示

- 转圈：工作中。
- 黄点：需要输入或批准。
- 绿点：本轮完成；本早期记录关联未读，当前已读后的显示行为见运行规范。
- 空闲：无角标；错误／未连接使用简短上下文提示。

## 验证与限制

核心测试覆盖过期回合拒绝、后台完成、未读确认、并行工具等待和桥认证。原生 smoke 使用三个真实 PowerShell ConPTY 和合成生命周期事件，生成 GPU 截图和 JSON；它不是 Claude/Codex 真实任务测试。CLI 升级可能改变 hook schema 或信任处理；实际 Agent 版本和真实事件验证需单独记录。

没有心跳可以证明静默推理期间 Agent 的语义进度；不使用静默超时伪造完成。后台任务和定时 Agent 工作流需要额外生命周期覆盖。

## 主要参考

- https://github.com/alacritty/alacritty
- https://github.com/emilk/egui
- https://github.com/kemokempo/egui_term/tree/31bbc7ab8503c9518fcee5717cfa29011e59f451
- https://learn.chatgpt.com/docs/hooks
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/cli-reference

完成角标表示当前 Agent 回合结束，持续运行的开发服务器等后台进程不会让转圈持续；新的自主回合报告 UserPromptSubmit 并恢复转圈。这不表示项目验收通过。

## 渲染器选择

同一 Windows 机器使用相同 GUI、字体和三个终端样例测试。最初 DX12 debug 构建选择 Intel UHD 630，驻留内存约 721 MiB；OpenGL 选择 Quadro RTX 4000，约 117 MiB。这是本机配置选择，不是同显卡纯后端比较，也不宣称普遍最快。release 测量单独记录。

默认：`cargo build --release`。
对照：`cargo build --release --no-default-features --features renderer-wgpu`。
切换构建功能会改变 `target/release/ai-terminal.exe`；在不同输出目录保留测量，不比较旧二进制。

Agent 事件在 eframe 的 logic 回调中消费，暂停绘制时仍运行。最小化会话关闭终端重绘唤醒，但继续归约 hook 事件。原生测试在最小化且没有绘制时投递完成事件，验证首次绘制前已更新未读状态，防止依赖焦点的状态刷新。
