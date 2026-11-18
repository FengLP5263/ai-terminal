---
name: ai-terminal
description: "使用 ai-terminal-cli 控制运行中的 AI Terminal：会话、团队分组排序、设置、浮窗、文件及 Git 只读查看、媒体和显式终端交互；不代理模型或自动处理信任授权。"
---

# AI Terminal 会话与团队

简体中文 | [English](SKILL.en.md)

通过管理器执行操作，让界面与保存的数据同步。需要支持管理 CLI 的 Windows 版本；0.13.0 及以前没有此接口。命令默认输出 UTF-8 JSON，可显式加 `--json`。

本技能的独立版本、最低 CLI 版本和来源见 [version.json](version.json)。先核对实例能力；技能更新不会给正在运行的旧管理器增加接口。

## 选择入口与实例

在受管会话中直接运行 `ai-terminal-cli instance info --json`；若 PATH 不可用，PowerShell 用 `& $env:AI_TERMINAL_CLI instance info --json`。从外部终端调用时使用完整程序路径，必要时加 `--data-dir <目录>` 指定管理器的数据目录，不能填项目目录。先核对返回的版本、实例、数据目录和 caller_session。

`current` 是发起命令的受管会话，不是界面选中的会话。外部调用必须使用查询得到的完整 UUID。会话名和组名可重复，不能用名字猜目标。显式 `--data-dir` 覆盖继承的实例；跨实例时 current 不可用。管理器未运行时应提示用户打开对应版本，不直接修改 sessions.json，也不自动打开另一个工作区。

## 按任务读取

- 查询、创建、改名、配色、启动、恢复历史对话、切换、关闭：读 [会话命令](references/sessions.md)。
- 建组、改组、解散和排序：读 [分组命令](references/groups.md)。
- 批量建队伍、引用当前主会话、预演及启动成员：读 [团队编排](references/teams.md)。
- 语言／字体／缩放、窗口、侧栏与浮窗：读 [原生设置](references/settings.md)。
- 文件树、Markdown、图片、只读 Git 和媒体播放：读 [查看命令](references/viewing.md)。
- 明确需要终端输入／读取或停止／重启时：读 [终端交互](references/terminal.md)。
- JSON、退出码、冲突、超时与回执：读 [协议与排错](references/protocol.md)。

## 执行约定

1. 查询现状并确定目标 UUID。以用户要求为范围，不改其他会话。
2. 多项元数据修改用 workspace apply；先 dry-run 检查，再按需附 if_revision 提交。参数错误整批不变；预演生成的 UUID 是临时的，应使用实际提交返回的 UUID。
3. 新会话默认未启动。用户要求启动 Agent 时，单独用 session run；只启动 Shell 则用 session start。run 只接受没有运行终端的会话，不把命令打进正在交互的 CLI。
   用户要继续已有任务时，先确定对应的 Agent 对话 ID，再按会话参考恢复。打开空白 Agent 或收到 process_started 都不代表已经进入原对话。
4. 检查退出码和 JSON 的 ok/error，不能只检查输出存在。启动回执仅表示进程已建立；状态继续来自原生生命周期，不代表登录、hook、模型或任务就绪。
5. 回读会话或工作区，报告实际创建的会话、组、顺序和启动结果。

关闭运行会话需要 --force，会结束其受管进程树。按用户已有授权执行；不要为普通改名、配色、分组反复确认。未明确要求关闭当前主会话时，不关闭它；解散组保留会话。批量元数据和进程启动是分开的事务，启动失败要报告已创建的会话，不重建整队。

不要读取／输出 control.json、管理 token 或账号凭据。终端读取、文字输入与 --submit 只用于用户已授权的任务，不为普通会话管理读取正文。不要自动接受信任／权限提示，或添加绕过权限的参数。安装此 Skill 本身不授权发送任务、修改其他项目或付费模型调用；按用户当前任务的授权执行。
