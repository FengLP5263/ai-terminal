# 显式终端交互与会话生命周期

简体中文 | [English](terminal.en.md)

要求 native_controls 能力。只有任务需要读取或操作终端内容时才使用；普通建组、配色、改名无须读取屏幕。目标用完整 UUID，受管调用可用 current（调用者，不是界面 active）。这些命令只操作该管理器拥有的真实 PTY。

```powershell
ai-terminal-cli terminal read <UUID> --offset 0 --limit 100
ai-terminal-cli terminal write <UUID> --text '待输入文字'
ai-terminal-cli terminal write <UUID> --file input.txt
ai-terminal-cli terminal write <UUID> --file -
ai-terminal-cli terminal key <UUID> --key escape
ai-terminal-cli terminal scroll <UUID> --lines 20
ai-terminal-cli terminal scroll <UUID> --offset 0
```

read 读取当前可视网格，scope=current_viewport；不会复制全部 5,000 行历史。offset/limit 用于当前屏幕响应分页；滚动历史用 scroll 再 read，正 lines 向旧内容、offset=0 返回底部。这是终端展示文字，不能据此推断任务完成，状态仍用 session show 的原生 activity。

write 文本最多 64 KiB、UTF-8，不含 NUL；--text 与 --file 二选一，文件 `-` 从 stdin 读取。默认只填入文字；显式 `--submit` 再发送回车，可能执行 Shell 命令或提交模型任务，必须符合用户已授权的任务范围。不要自动响应信任／登录／权限，不要为了“让 Agent 启动”把任意输入打进未知运行程序；有专门的 session run。

文本按终端粘贴模式传递（bracketed paste 时移除 ESC，普通模式统一换行为 CR）。不是附件 API，也不模拟系统剪贴板。key 支持 enter、escape、tab、backspace、up/down/left/right、home/end、page_up/page_down、ctrl_c/ctrl_d/ctrl_l/ctrl_z；方向键保留应用光标模式。scroll 只改变本地历史，不向 CLI 发方向键／鼠标；--lines 与 --offset 二选一，范围不超过 5,000。

输入／读取正文只在有界请求或响应中短暂存在，不写日志、工作区或回执。修改回执存 SHA-256 请求指纹与确认信息；读取内容不进入回执缓存。Agent 自己和 Shell 可能按它们的设置保存输入，管理器不会修改这些外部配置。不要把敏感正文写入命令日志；必要时从 stdin／文件传入，并按调用者自身策略管理输入文件。

```powershell
ai-terminal-cli session acknowledge <UUID>
ai-terminal-cli session stop <UUID> --force
ai-terminal-cli session restart <UUID> --force
ai-terminal-cli session start <UUID>
```

acknowledge 只清除未读，不切换会话。stop 停止该会话拥有的进程树但保留 UUID、组、颜色和排序；restart 重新建立 Shell。运行终端停止／重启需要 --force；未运行时 restart 不需要。要重新运行 Agent，先明确停止原程序，再 session run（不能对正在运行的 Shell 使用 run）。查询 process_id 可核对实际 PTY 身份；running 不是任务就绪或完成。关闭会话与停止进程不同：session close 会删除导航元数据。

多行文本中的换行本身可能在普通终端模式下执行命令；--submit 控制是否额外追加回车，不会转义 Shell 语法。
