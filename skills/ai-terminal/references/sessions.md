# 会话命令

简体中文 | [English](sessions.en.md)

以下命令均可追加 --json；在管理器外调用时在命令前加 --data-dir <数据目录>。所有 ID 使用完整 UUID，或在受管会话中用 current。

| 命令 | 参数与行为 |
| --- | --- |
| instance info | 返回管理器版本、PID、实例 ID、数据目录、调用者会话及工作区 revision；不返回凭据 |
| session list | --offset N --limit N；默认 100、最大 200；按 next_offset 翻页，total 为总数 |
| session show ID | 查看名称、目录、Shell、颜色、组、顺序、活动项、运行状态、Agent、生命周期、未读和启动错误；新版 question_unread 表示未查看的提问提醒 |
| session create | 必需 --name NAME --directory PATH；可选 --shell powershell/cmd/bash、--color COLOR、--group UUID、--before UUID 或 --after UUID |
| session rename ID | --name NAME |
| session color ID | --color none/red/green/amber/blue/pink/purple；yellow 是 amber 的命令行别名 |
| session group ID | --group UUID；none 表示移出组 |
| session move ID | 恰好指定 --before UUID 或 --after UUID；移动整个会话，不重建进程 |
| session start ID [--activate] | 后台启动其 Shell，已运行时保留现有程序 |
| session activate ID | 切换并恢复主窗口；未运行时启动 Shell，已运行时保留；清除该会话未读 |
| session run ID | --agent NAME，可选 --activate；仅未启动／已退出的会话可运行 Agent |
| session resume ID | --agent NAME --conversation-id AGENT_ID，可选 --activate；需要 agent_resume 能力，目标终端须未运行 |
| session close ID | 未启动会话直接关闭；运行中必须 --force，结束自身受管进程树 |

示例（PowerShell）：

```powershell
$new = ai-terminal-cli session create --name '前端 ✨' --directory $PWD.Path --color blue --json | ConvertFrom-Json
if (-not $new.ok) { throw $new.error.message }
$id = $new.result.results[0].id
ai-terminal-cli session run $id --agent codex --json
ai-terminal-cli session rename $id --name '前端实现' --json
ai-terminal-cli session activate $id --json
```

run 支持 codex、claude、qoder（国际版，公开 qodercli 入口）、qodercn、kimi、zcode、trae。qodercli、qoderclicn、traecli 也可作名称别名。通过既有会话 shim 启动真实 CLI，不保存自动启动配置；管理器重启后仍恢复为普通 Shell，避免意外自动运行 Agent。

新建默认 PowerShell，目录是已有目录，不创建项目或修改项目文件。创建只建立导航元数据，不自动启动。名称为 1–128 个非控制字符，去除首尾空白。普通 create 的相对目录按调用者工作目录解析。已有运行会话不提供直接改目录；要切换其目录，应由用户在 Shell 中操作。

关闭的会话从导航中移除，Agent 自己的历史保留；最后一个组成员关闭时删除空组。进程清理在后台进行，回执中的 process_cleanup=scheduled 不等于所有子进程已经退出。不要把关闭当前调用者作为流程的前置步骤，否则它会在读取回执前结束。

session list 的分页索引对应当前全局顺序。连续翻页时核对 revision；变化后重新查询，避免并发排序造成遗漏。超过 1 MiB 的完整工作区应改用分页会话列表。

## 启动与恢复历史对话

管理器会话 UUID 和 Codex／其他 Agent 的历史对话 ID 是两个不同标识。继续已有任务时，先按组读取一次会话列表，保留已经进入正确对话的成员；使用用户提供、明确记录或 Agent 自身 resume／agents 选择器核对的对话 ID。按名称和原项目目录共同核对；多条记录仍有歧义时再询问，不能用 --last 猜测，也不依赖私有数据库结构。

先检查 `ai-terminal-cli --help` 是否列出 `session resume`（0.15.0 起提供），再查询 `instance capabilities`。客户端包含此命令且管理器支持 agent_resume 时，可在目标终端未运行时直接恢复；旧客户端按下方 Shell 流程处理：

```powershell
ai-terminal-cli session resume <管理器会话UUID> --agent codex --conversation-id <Codex对话UUID> --json
```

Codex／TRAE 使用 `resume ID`，Claude／Qoder 国际版／Qoder CN 使用 `--resume ID`；Kimi 和 ZCode 暂无此接口。回执 resume_requested 只确认参数已交给真实 CLI，readiness 仍为 process_started，不能据此宣称历史已经载入。

0.14.0 正式版没有 agent_resume 能力。目标已经运行 Shell 时，也不必先打开空白 Codex：用 terminal read 确认当前确实停在空闲 Shell 提示符，再通过显式 terminal write 提交 `codex resume <已核对的对话UUID>`。网页等会话的 Shell 目录与原对话不一致时，先在该 Shell 切换到原项目目录。不要向未知程序或正在执行任务的 Agent 输入命令，不为恢复而强关已有工作。

如果本次操作刚误开了空白 Codex，确认尚未提交任务后可用 ctrl_d 断开，再核对是返回 Shell 还是终端已退出；后者用 session start 建立 Shell。Codex 的更新提示可以跳过本次更新；登录、信任和权限仍交给用户。恢复后核对对话名称、项目目录和历史内容再报告完成，不发送新的模型任务，也不把屏幕文字作为运行状态的来源。

question_unread 与 activity 独立：Agent 可以继续 working，同时有一条未查看的提问提醒。查看会话或 session acknowledge 会清除该提醒，只表示已查看，不表示问题已回答。提醒来自明确的 Codex OSC9 `Question:`／`Plan mode prompt:` 通知，不从铃声或普通输出推断；需要 Codex 使用 OSC9 并允许对应通知类型。
