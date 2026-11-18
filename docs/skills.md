# Agent 技能包

简体中文 | [English](skills.en.md)

面向使用者的技能位于 [skills/ai-terminal](../skills/ai-terminal/SKILL.md)，与 .agents/skills 中维护本项目源码的开发／测试技能分开。包含标准 SKILL.md、英文版本、Agent 展示元数据，以及会话、分组、团队编排、协议与排错参考。技能包不依赖本仓库源码或额外 MCP 服务，命令能力由已安装的 ai-terminal-cli 提供。

## 安装

将整个 ai-terminal 文件夹放到所用 Agent 的技能目录中，保留 references 和 agents 子目录。支持项目级 .agents/skills 的 Agent 可放在 `<项目>/.agents/skills/ai-terminal/`；其他 Agent 使用其自身规定的技能目录。不要只复制入口文件，也不要放进 AI Terminal 的用户数据目录。无需修改用户 hook、认证或模型配置。

## 版本与升级

技能按语义化版本独立管理，软件／CLI 版本和技能版本可以不同；文档修正可单独升级技能。安装目录内的 [version.json](../skills/ai-terminal/version.json) 记录技能版本、最低 CLI 版本、协议版本和源码仓库。安装或升级后回读该文件，再用 instance info／capabilities 核对当前管理器；版本未知的旧技能应明确记为旧包，不能推断成最新版。

独立 ZIP 的 manifest 同时记录技能版本、最低 CLI 版本和构建时的软件版本，SHA-256 对应具体内容；校验器会拒绝包内版本与 manifest 不一致的制品，并继续支持已发布的旧格式包。预览号只表示未发布内容，正式发版时固定版本并生成新的制品，不能覆盖已发布的版本。

恢复已有团队时，技能区分管理器会话 ID 和 Agent 历史对话 ID，核对名称／项目后直接恢复，避免先开空白 Agent 再重来。步骤见[会话参考](../skills/ai-terminal/references/sessions.md#启动与恢复历史对话)。

源码包内直接提供 skills/ai-terminal；本地构建独立 ZIP：

```powershell
python scripts/skill_bundle.py build
python scripts/skill_bundle.py verify --path output/skills/<技能包文件名>.zip
```

独立 ZIP 含 ai-terminal/、LICENSE.txt 和可回读的 SHA-256 manifest。完整 Windows 软件 ZIP 也携带 CLI 与技能目录。历史安装包没有 CLI 时，应先使用支持此接口的版本；安装 Skill 不会给旧管理器补接口。

## 使用场景

> 使用 $ai-terminal，把当前会话作为协调员，创建前端和后端会话，组成“产品研发队”，按协调员、前端、后端排序，并分别启动 Codex 和 Qoder。

Agent 会先确认实例和调用者，按团队参考创建、预演和提交元数据，然后单独启动成员并回读结果。普通修改遵从用户已有授权；关闭运行会话会终止其受管进程树。新增设置／查看／终端交互参考；正文读取和任务提交须在用户已授权的范围，技能不授权自动接受信任。

完整参数见 [CLI 文档](cli.md)和[技能入口](../skills/ai-terminal/SKILL.md)。

最低 CLI 版本表示基础命令与兼容流程的要求；新增命令仍须核对客户端帮助和管理器能力。`session resume` 从 0.15.0 CLI 提供，旧客户端可按会话参考使用显式 Shell 恢复，不因更新 Skill 自动获得新命令。
