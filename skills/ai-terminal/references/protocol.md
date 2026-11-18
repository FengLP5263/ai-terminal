# 协议、退出码与排错

简体中文 | [English](protocol.en.md)

普通命令输出一个 UTF-8 JSON 对象；--help/--version 输出文字。JSON 顶层为 schema_version（当前 1）、request_id、instance_id、ok，以及 result 或 error。调用失败时 error.code 可编程判断，message 用于解释。解析失败或无法连接时实例／请求标识可能为空。没有图形窗口、字体或 GPU 初始化。

| 退出码 | 常见原因 | 处理 |
| --- | --- | --- |
| 0 | 成功，包括 dry-run | 检查实际 result |
| 1 | 启动／保存／传输失败或 outcome_unknown | 读取 error.code；超时先查回执和现状 |
| 2 | 参数、目录、版本或大小错误 | 修正输入，整批参数错误不改变工作区 |
| 3 | not_running | 打开支持 CLI 的对应管理器；不要编辑配置冒充执行 |
| 4 | not_found / no_current_session | 查询 UUID；外部调用不能用 current |
| 5 | conflict / session_running / request_id_reused | 回读并核对意图；不得盲目强关或换请求 ID 重试 |
| 6 | busy | 队列满、停止中或尚未执行即过期；稍后核对现状 |

## 超时与去重

修改命令可指定 --request-id <UUID>。管理器保存最近 64 条、合计最多 4 MiB 的修改回执，仅当前进程内有效。同一个 ID、同一完整请求返回原回执，不重复执行；同 ID 不同请求返回 request_id_reused。查询和 dry-run 不占修改回执缓存。缓存不是持久任务队列，不承诺重启或淘汰后去重。

命令超时可能已执行，也可能尚未执行。先 `request show <UUID>`（同一管理器），并回读 session list／workspace show。确认回执仍在时可用相同 ID 和完全相同参数恢复；若回执已淘汰或管理器已重启，检查实际对象后决定下一步，不能直接换 ID 重建。

建议由调用者预先生成并保留请求 ID；默认自动生成的 ID 若没有收到响应则无法用于回执查询。不要把 dry-run 的 ID 复用为实际提交，或变更 caller_session/参数后复用旧 ID。

## 边界与隐私

独立管理通道监听 127.0.0.1 随机端口，与生命周期 hook 桥分离。管理 token 仅在管理器和它的子进程环境中明文存在，实例文件中由 Windows 当前用户 DPAPI 加密。不要读取或展示 token；CLI 自行处理发现／解密。显式 --data-dir 优先；继承的实例失效时不自动回退到其他工作区。新版本同一数据目录只允许一个管理器写入，多个预览需独立数据目录；旧版本没有此互斥保护，不能与新版共用数据目录同时运行。

请求最多 256 KiB、响应最多 1 MiB；32 项管理队列、16 项连接队列、4 个连接处理线程，读取与执行等待都有截止时间。大列表用 --offset/--limit；批量最多 128 操作。没有空闲轮询；事件明确唤醒根窗口，最小化时仍在后台归约。修改动作需要运行中的管理器，不直接离线修改 sessions.json。

0.14.0-preview.2 增加 native_controls 能力发现及原生设置、查看和显式终端交互。读取内容不缓存，修改回执仅保存 SHA-256 请求指纹及无正文的确认结果。running 只表示终端存在；状态仍来自明确生命周期，不能从屏幕文字推断。

新 CLI 会先用 Info 查询 native_controls；旧管理器明确返回 unsupported_command（退出码 2），不会尝试新协议或打开其他实例。截图不进入回执缓存，结果用 window capture-status 查看。
