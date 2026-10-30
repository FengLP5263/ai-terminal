# 运行、配置与操作

## 支持环境

已验证 Windows x64、Rust 1.99.1 MSVC、Visual Studio C++ Build Tools 与 Windows SDK。
Cargo.toml 声明最低 Rust 1.92；使用 stable 工具链，制品记录实际 rustc 版本，不宣称位级可复现。
需要 Python 3.11+ 执行治理／打包。POSIX 原生 GUI 构建尚未接入，跨平台脚本不代表产品已跨平台。

## 进程与配置

一个管理器，若干受管 CLI/ConPTY，以及短时 hook helper。每个会话一个 Windows Job Object，
关闭会话／程序只回收该会话自己拥有的进程树。禁止用进程名批量清理 PowerShell、Codex 或 Claude。

桥只监听 127.0.0.1 的动态端口，无固定服务端口。每帧上限 8192 字节、256 项队列，
满队列明确拒绝并由 helper 有限重试；连接和读写有短超时。token 只由管理器随机生成并传给子进程。

| 名称 | 来源与用途 |
| --- | --- |
| LOCALAPPDATA | 默认会话元数据根目录；缺失时回退临时目录 |
| PATH、APPDATA、USERPROFILE、SystemRoot | 查找已安装的 CLI／Windows 组件 |
| AI_TERMINAL_ENDPOINT、AI_TERMINAL_TOKEN | 本轮管理器的 loopback 桥和认证，不能提交值 |
| AI_TERMINAL_SESSION、AI_TERMINAL_GENERATION | 本地会话标识与启动代次 |
| TERM、COLORTERM、NO_COLOR、FORCE_COLOR、CLICOLOR_FORCE | GUI 启动前在本进程内规范交互终端颜色能力 |
| CLAUDECODE | 子会话避免继承父 Claude 的嵌套标记 |
| CARGO_HOME、RUSTUP_HOME | 可选本机 Rust 安装位置，由忽略的 .tools/env.ps1 提供 |
| AI_TERMINAL_PYTHON | 可选脚本 Python 可执行文件路径 |

不在仓库创建 API Key 模板：管理器没有独立模型接入，沿用 CLI 的既有认证和模型设置。
GUI 只改自己的进程环境，不改 Windows 用户／系统环境或 Warp。系统字体的默认 Windows 路径目前在源码中，
非默认系统盘／不同字体环境尚未验证，后续移植时处理，不能写成已支持。

## 启动、就绪、退出

`scripts/windows/project.ps1 run` 构建并打开交互窗口；点＋选择项目和 Agent。
首次 Codex 在 /hooks 中审查状态 hook。恢复列表后必须用户选择继续，不自动启动所有会话。
正常退出通过应用窗口；活跃任务先保存／确认。没有全局 stop/reset 命令，不删除用户会话数据。

readiness 是实际窗口／终端与状态消息，不是后台进程存在。应用没有日志采集服务；
隔离 smoke、性能测量与探针把 JSON/PNG 写到被忽略的 output/。首次 UI 标记不等于严格屏幕首帧。
默认检查和测试不调用模型；verify-claude 必须显式 AllowModelRequest，费用边界见项目测试技能。
