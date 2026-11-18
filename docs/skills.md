# Agent 技能包

简体中文 | [English](skills.en.md)

面向使用者的技能位于 [skills/ai-terminal](../skills/ai-terminal/SKILL.md)，与 .agents/skills 中维护本项目源码的开发／测试技能分开。包含标准 SKILL.md、英文版本、Agent 展示元数据，以及会话、分组、团队编排、协议与排错参考。技能包不依赖本仓库源码或额外 MCP 服务，命令能力由已安装的 ai-terminal-cli 提供。

## 安装

将整个 ai-terminal 文件夹放到所用 Agent 的技能目录中，保留 references 和 agents 子目录。支持项目级 .agents/skills 的 Agent 可放在 `<项目>/.agents/skills/ai-terminal/`；其他 Agent 使用其自身规定的技能目录。不要只复制入口文件，也不要放进 AI Terminal 的用户数据目录。无需修改用户 hook、认证或模型配置。

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
