# 0.3.7 原生 TUI、状态与图片

## 修改

- 取消 Codex 的强制 --no-alt-screen，保留其全屏欢迎 Logo、固定底部输入区和原生消息滚动。受管入口用 --no-daemon 保留自身会话环境及 Job 所有权，不停止已有 daemon。
- Codex 可能到首轮提交才触发 SessionStart。此前显示「等待任务通知」；没有根据终端输出或进程存在推断任务状态。连接说明不再将已 Active 的钩子误判为尚需授权。
- 窗口层原本只在剪贴板有非空文本时发出 Paste，图片因此无事件。固定版本 egui-winit 0.35.0 的本地补丁保留图片／空文本粘贴请求，附有上游 commit、归档摘要及许可证。
- 终端仅在当前识别为 Codex／Claude 时查询 Windows 图片格式。图片粘贴转发两者原生支持的 Alt+V；用户仍用 Ctrl+V 或 Shift+Insert。像素、附件编码与上传完全由 CLI 负责。普通文字、Shell 和会话名称编辑保持原路径。

## 已验证

- 优化版原生 smoke：三个真实 PowerShell、进行中／需要处理／完成提示、GPU 截图与自身 PTY 回收通过；截图已目视核对。独立预览窗口标题为 ai-terminal 0.3.7，保留原有窗口与活动进程。
- 权威 ci：治理、fmt、全目标 Clippy、89 项 Rust（包括额外窗口层补丁测试）、3 项 Python 回归通过。
- Codex CLI 0.164.0：真实原生窗口截图中恢复大 Logo，使用 ALT_SCREEN、SGR_MOUSE、MOUSE_MOTION；等待完整启动后粘贴，输入区出现 [Image #1]，未按 Enter。
- Claude Code 2.1.316：真实窗口输入区出现 [Image #1]，未按 Enter。观察到了 fullscreen 与 CLI 自身回退 classic 的启动；本修复不修改用户 renderer 配置。
- Codex 状态：真实 /hooks 浏览器显示 8 类状态钩子为 Active；仅欢迎页未触发 SessionStart。真实 CLI 接入本地回环假 Responses 服务后收到 SessionStart、UserPromptSubmit、Stop，验证与管理器使用相同的会话启动器及独立 helper。另验证了正常退出的 SessionEnd。
- 上述图片不发送，假服务没有调用真实推理服务；CLI 自身的登录检查／预热等启动网络行为与模型计费回合分开记录。

测试证据位于忽略目录 output 的 codex-ready-image-v037、claude-image-wait、codex-offline-complete 等独立目录；不将终端内容提交到仓库。

## 边界

核查覆盖当前 Windows x64 安装的 CLI 与当前图片剪贴板。不同截图软件、图片文件列表、远程／WSL 剪贴板、用户自定义键位和后续 CLI 升级仍应分别验收。状态 hook 的信任必须由用户在 CLI 中审查；本版本没有改写用户级 hook 配置或信任文件。

协议来源：[Codex CLI 参数](https://learn.chatgpt.com/docs/cli/reference)、[Codex 生命周期 hooks](https://learn.chatgpt.com/docs/hooks)、[Claude 图片输入快捷键](https://code.claude.com/docs/en/interactive-mode)。
