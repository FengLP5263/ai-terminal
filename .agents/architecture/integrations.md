# 外部集成与关联仓库

简体中文 | [English](integrations.en.md)

| 对象 | 定位 | 合约与所有权 | 协调 |
| --- | --- | --- | --- |
| Codex CLI | 当前／持久 PATH 与用户安装目录 | CLI 自有认证、信任和 hook schema；管理器只注入本次参数 | 升级后重新验证配置解析、/hooks 审查及实际生命周期 |
| Claude Code | 当前／持久 PATH 与用户安装目录 | CLI 自有认证、模型、插件和权限；--settings 仅会话级 | 模型请求必须显式授权；CLI 可自行保存它自己的运行状态 |
| Qoder CN CLI | 公开 qodercn／qoderclicn 入口，支持原生与 npm 安装 | CLI 自有登录、模型、目录信任；--settings 仅会话级，显式设置优先 | 合成状态与真实启动分开验证；不自动确认信任或发起模型请求 |
| Kimi Code CLI | 公开 `kimi` 入口 | CLI 自有模型、登录与配置；本版未接入任务通知 | 仅身份／启停识别，不改用户或项目配置 |
| ZCode CLI | 公开 `zcode` 入口；本次实测社区 zcode-app-cli | 配置与认证归对应发行版及 CLI 管理 | 同上，区分官方与社区版验收结果 |
| TRAE CLI | 公开 `traecli` 入口 | CLI 自有模型、登录与配置；本版未接入任务通知 | 仅身份／启停识别，不改用户或项目配置 |
| PowerShell | Windows 系统组件 | 普通交互 shell，无 AI 工作状态推断 | 管理器负责本会话进程树，不修改用户 profile |
| Git Bash | 可选的 Git for Windows 安装 | 加载系统／用户启动配置后追加本会话目录通知与入口，不修改用户文件 | 无 Git Bash 时明确提示，允许改用可用的 PowerShell／CMD；WSL 和其他 Bash 不代替 Git Bash |
| 用户工作目录 | 用户主目录、最近目录或终端 cd 后的目录 | 被打开的项目仍归用户和其项目规范管理 | 选择目录不授权管理器／开发者修改其中代码 |
| Warp | 仅手动参考／REA 静态对比 | 不属于运行依赖，无代码／数据接口 | 不修改或注入其运行进程，不复制其私有实现 |

没有已知的其他源码仓库依赖；桌宠等相邻目录不是本项目的自动修改、提交或发布范围。
统一仓库位置通过相对路径或 REPO_ROOT 表示，个人工作区和会话 ID 留在忽略的本机台账。

hook 协议由 src/status.rs、bridge.rs、launch.rs 维护：事件带本地会话、代次、Agent session、turn 等元数据。
可选 parent_invocation 和 console_processes 关联嵌套 Shell 的父调用与所属控制台；旧发送端缺失时兼容 None。ShellStarted／ShellEnded 仅更新临时头像，不推断任务状态，进程 ID 列表不持久化。
只传 reducer 所需字段，不转发输入／输出正文；helper 返回空结果，不作权限决定。
旧代次／旧回合／子会话过滤与重试去重须保持。CLI 版本变动可能使 hook 失效，不能仅依赖静态配置解析判定联调通过。

验证顺序：安装并配置用户 CLI → 构建 manager/helper → 离线测试 → 隔离原生 smoke → 用户审查 hook → 按授权做真实回合。
原生 smoke 的合成事件、实际 TUI 的未提交文本、真实模型回合是三类不同证据。
不存在跨仓库部署顺序；未来若建立服务／仓库关系再补相应契约及独立发布授权。

0.3.0 终端会话只通过本会话的 codex/claude 入口转发真实 CLI，启动／退出与任务状态协议分开。CLI 始终沿用已有认证、模型和信任；未支持的入口不猜测图标或工作状态。

Codex 的会话级 hook 以固定命令读取 AI_TERMINAL_HELPER，不将安装目录写进可信定义。用户／项目 hook 与信任记录不由管理器修改。状态恢复允许前台调用内显式 resume／clear／fork 切换，不以输出文本猜测会话身份。

0.3.6：agent_id 标识的子 Agent hook 不参与根会话状态，agent_type 本身不代表子会话。Claude 全屏的滚动通过其请求的终端鼠标协议传输，不解析绘制文字作为任务状态，不修改用户 renderer 偏好。

0.3.7：Codex 默认 TUI 与独立引擎均保留 CLI 自有逻辑。其 SessionStart 可能延迟到首轮任务；使用真实 CLI＋loopback 模拟 Responses 验证 hook 时，必须明确标识为离线假模型回合，不称为真实模型验收。Codex 与 Claude 的 Windows 图片粘贴交由原生 Alt+V；管理器只查图片格式并传递快捷键，不代理附件或模型请求。


0.4.0 HookEvent 新增可选 exit_code；旧发送端缺失时兼容 None，受管 ShellAgentEnded 传递真实退出码。StopFailure（现有 Claude 配置）和非中断的非零进程退出标记 Failed；工具失败但 Agent 继续恢复时仍保持 Working。用户中断退出码不当作失败。当前 Codex hook 接口没有 StopFailure，未被官方事件导出的 CLI 内部错误不会通过解析屏幕或静默超时猜为失败；具体状态仍应在原 CLI 中核实。

原生浮窗只导航当前管理器的会话，不接管其他进程中的旧版会话，也不修改用户级 CLI 配置或信任。

## 原生控制台与公开启动器

Microsoft.Windows.Console.ConPTY 是固定版本的本地运行依赖，来源和 MIT 许可保存在 assets/windows/conpty，不是模型代理或关联源码仓库。它以程序旁的文件运行，遵循现有 ConPTY／Job 生命周期。

标准 npm 启动器的字面目标核查不执行脚本、不读取 CLI 认证／模型设置、不修改全局 PATH，也不根据任意退出码重试模型命令。未知包装器保持原来的调用方式。

2026-11-03 用户取舍：Codex 优先保留准确的状态提示，暂不启用共享 agents 入口。保持独立模式和会话级 hook；不要仅为恢复 for agents 删除注入参数、改用户配置或退回输出猜测。0.166.0 启动源码仍对不兼容的会话级配置排除共享后台。后续恢复入口必须同时验证跨终端状态归属、切换对话与关闭回收；记录见 [界面核查](../../docs/ui-rendering.md)。

后续开发采用 Codex 的 OSC 7501 明确生命周期，解除默认会话级配置与共享后台冲突。2026-11-03 的暂缓决定作为历史保留；原生模式及显式 --no-daemon 兼容模式的当前边界见运行记录。原生状态只属于当前受管 PTY，不订阅或控制用户的其他 daemon 会话，不收集对话正文或凭据。

0.4.6：Codex 的带 app 身份的原生通知可直接建立当前 PTY 的身份，覆盖绝对路径、别名和远程 resume 入口；不需要修改这些命令，也不订阅共享后台。clear 不建立身份或表示退出，返回当前代次 Shell 提示符才清理未经 helper 的身份。其他 Agent 仍通过已接入入口识别。

## Qoder CN CLI（0.4.6）

只识别官方公开的 `qodercn`／`qoderclicn`，不把 Qoder IDE 的 `qoder` 或国际版当作 CN CLI。沿用 PATH、注册表持久 PATH 和已有 npm／用户安装目录发现；同目录优先原生 exe，再处理 cmd／bat；只跳过已确认目标缺失的标准 npm 包装器。

`--settings` 只引用本次 runtime 的 hook 文件，使用原生 helper 的 exec 形式及 `args`，不引入 PowerShell 启动开销、不返回权限决定或上下文。只注册 Qoder 支持的事件，不注入 Claude 专用 PostToolBatch。显式用户 `--settings` 不覆盖、不合并私有内容。恢复参数为 `--resume <id>`。CLI 自行处理登录、模型、权限和目录信任；未信任目录中的 hook 可能被 CLI 阻止。

状态基于官方 SessionStart、UserPromptSubmit、工具／交互通知、Stop、StopFailure 与实际退出；不以输出或进程存在猜状态。桥继续丢弃子 Agent、正文及认证信息，调用／启动代次隔离。Qoder 的 source=new 可切换前台会话。鼠标拖选与 Alt+V 图片粘贴复用正式终端能力。

接口来源：[参数](https://docs.qoder.cn/en/cli/cli-reference)、[配置合并](https://docs.qoder.cn/en/cli/settings)、[Hooks](https://docs.qoder.cn/en/cli/hooks-reference)、[快捷键](https://docs.qoder.cn/en/cli/interface)。实际版本核查、未提交启动截图、合成 hook 与真实模型回合分别记录。

## Kimi Code／ZCode／TRAE CLI（0.4.7）

识别已安装的公开入口 `kimi`、`zcode`、`traecli`。会话 shim 保留原参数，并按调用 ID 报告 ShellAgentStarted／ShellAgentEnded，仅用于头像与启动／退出识别；不解析输出或推断任务进度。本版尚未接入这三个 CLI 的会话级任务通知，也不复制或修改用户／项目 hook、provider 或认证配置。上游 hook 能力可能随版本变化，管理器未接入不代表 CLI 没有 hook。

本次核查的安装版本为 Kimi Code 2.5.0、TRAE CLI 0.213.1 public，以及社区 zcode-app-cli 3.17.2-44（运行时 0.17.1）。后者是独立社区终端前端，不是 Z.ai 官方独立 CLI 发行包，不能以它的启动结果代表其他 ZCode 版本兼容。产品图标来源与已安装 CLI 的发行来源分别记录；图标逐文件身份见 assets/icons/SOURCE.json。

shell_switch_probe 的 `kimi`、`zcode`、`traecli` 模式在全新管理器工作区中启动实际入口、截取启动画面，再聚焦终端空白区，只向自建 CLI 发送 Esc／Ctrl+C 并检查返回 Shell 的截图。不提交模型提示，不接受目录／hook 信任。真实启动、合成生命周期与付费模型回合保持为三类证据。
