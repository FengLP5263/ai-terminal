# 外部集成与关联仓库

简体中文 | [English](integrations.en.md)

| 对象 | 定位 | 合约与所有权 | 协调 |
| --- | --- | --- | --- |
| Codex CLI | 当前／持久 PATH 与用户安装目录 | CLI 自有认证、信任和 hook schema；管理器只注入本次参数 | 升级后重新验证配置解析、/hooks 审查及实际生命周期 |
| Claude Code | 当前／持久 PATH 与用户安装目录 | CLI 自有认证、模型、插件和权限；--settings 仅会话级 | 模型请求必须显式授权；CLI 可自行保存它自己的运行状态 |
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
