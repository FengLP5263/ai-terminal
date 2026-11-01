# 0.3.6 状态接入与 Claude 滚轮

## 行为

- 点击顶部状态可查看该 Agent 的连接说明，复制 /hooks 后由用户在 CLI 中审查。没有自动输入、信任、全局配置写入或模型代理。
- 固定的 Codex hook 命令继续沿用 0.3.5 定义；不会因本次新增说明界面而重新改变可信命令。
- 排除继承的旧会话启动器路径，防止新窗口将旧 helper 当作真实 CLI；子 Agent 事件不再干扰主会话，根 --agent 仍正常回报。
- 旧版普通滚轮一律滚本地历史，Claude 全屏使用备用屏幕，历史由应用管理，因此滚轮没有效果。现在按应用是否开启鼠标协议路由，支持 SGR 和传统鼠标编码；从不把滚轮合成输入历史的方向键。
- 普通终端继续查看本地历史；Shift 强制本地历史，Ctrl／Command 保留宿主缩放。高分辨率点滚动累积余量，单次应用滚轮事件有 32 步上限。

## 验证

回归覆盖真实 ConPTY 的上／下滚轮字节、备用屏幕本地消息翻页、Shift 本地历史、Ctrl 无输入、普通历史和禁用鼠标时的备用屏幕不发送方向键。状态说明通过实际点击与复制事件测试；原生 helper 过滤子会话，独立子进程验证继承旧启动器的 CLI 发现。

原生 scroll_probe 使用本地 raw-console pager，截图记录翻看旧消息、回到底部和打开状态说明。真实 Claude 仅启动已信任目录下的 CLI，记录生命周期与终端模式；不提交模型请求。该证据与真实模型回合区分。

CLI 只读协议参考：[Claude 终端配置](https://code.claude.com/docs/en/terminal-config)、[Claude hook 字段](https://code.claude.com/docs/en/hooks)。

## 本机最终结果

84 项 Rust、3 项 Python 回归及治理、fmt、全目标 Clippy 通过。原生优化版的普通／MOUSE_MOTION 消息翻页、状态说明弹窗和三种任务状态 smoke 均通过，截图已目视核对，测试进程及其 PTY 正常回收。实际 Claude Code 2.1.316 在全屏模式下收到 SessionStart，启用 ALT_SCREEN／MOUSE_MOTION／SGR_MOUSE；没有提交模型请求或修改用户渲染偏好。该版本的完整长对话与真实模型任务仍由用户验收。
