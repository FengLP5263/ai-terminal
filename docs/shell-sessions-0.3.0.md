# 0.3.0 终端优先的会话流程

新建只展示 PowerShell、CMD、Bash 菜单；默认用户主目录，随后沿用当前或最近有效目录。cd 后的目录会在提示符边界更新，默认目录随之改变。旧会话不丢失、不自动启动。

用户输入 codex/claude 才启动 Agent，图标根据本会话真实启动／退出通知切换，退出回到 shell。只输出 codex 或 claude 字样不改变图标。工作中／等待／完成仍依赖原有生命周期消息，不根据进程存在或输出静默推测。

配置只在本会话生效，已有用户配置和信任不被修改，Claude hook 文件复用，不反复写入。Bash 当前为 Git Bash；其他 Agent、WSL、绝对路径与用户自定义别名的识别后续再扩展。

## 验证记录

- 已通过三种真实 shell 的目录变更、中文空格路径与目录继承测试。
- 已通过三种真实 shell 输入 codex/claude 的启停通知测试，使用独立原生测试 CLI，不发模型请求。
- 旧会话数据、后台状态更新、调用间迟到事件、图标恢复和用户名称保留均有回归。
- 完整检查、原生画面与本地预览交付结果在完成后补记；当前未提交／发布。

## 本地交付（2026-11-01）

- 治理、fmt、Clippy -D warnings、62 项 Rust 和 3 项 Python 检查通过，共 65 项；包含展开／收起侧栏的新建菜单真实绘制点击，以及 echo codex 不误触发识别。
- output/shell-codex-v030-20261101-025559：真实 PowerShell 中提交 codex 启动命令，收到 ShellAgentStarted:codex，实际 Codex TUI 中中文输入可见，截图已查看，正常退出且本次 root PTY 已回收；未向 Agent 提交 Enter 或模型请求。
- output/smoke-20261101-025827-855：三个真实 PowerShell＋合成 Agent 生命周期，图标／未读／等待／工作中状态、GPU 截图、正常退出和自身 PTY 回收通过。该截图为合成状态，不冒充真实 Agent 工作回合。
- output/idle-shell-v030-20261101-025839-442：空工作区 8.025 秒 8 帧，UI CPU 12.673ms；5 秒主进程 CPU 0.296875 秒，工作集约 105.1 MiB。不是长期多会话性能结论。
- 本地 ZIP：output/packages/ai-terminal-v0.3.0-windows-x64-20261031T190728Z-b35506.zip，316 项，回读校验通过；dirty=true，未发布。
  SHA-256：572de634f0d5a60c5356c3ef1c3a324607f94df20ddb2323be4bdb1fbd5a0dd4。
- output/preview-v0.3.0 的两个 EXE 与已校验包哈希一致，预览使用独立元数据。未自动启动保存的旧会话，不修改用户现有终端。
- 当前位于 feature/shell-first-sessions，基于已提交的 45dc789 CLI 修复；本轮新功能尚未提交、推送或发布。
