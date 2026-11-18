# 分组与排序

简体中文 | [English](groups.en.md)

```powershell
ai-terminal-cli group list --json
ai-terminal-cli group create --name '研发队' --members 'current,<成员UUID>,<成员UUID>' --json
ai-terminal-cli group rename <组UUID> --name '产品研发' --json
ai-terminal-cli session group <会话UUID> --group <组UUID> --json
ai-terminal-cli session group <会话UUID> --group none --json
ai-terminal-cli session move <会话UUID> --after <另一会话UUID> --json
ai-terminal-cli group dissolve <组UUID> --json
```

尖括号表示待替换项，不能原样执行。先从 list 或 create 回执获得 UUID，不能按重名名称猜测。members 使用逗号分隔，不含空白；一个组需要 1–128 个有效且不同的成员。创建组会把这些会话从原组移入新组；没有独立的空组占位。移走最后一名成员会移除旧空组。

组成员显示顺序来自整个会话列表，而不是 members 参数的顺序。要使现有主会话作为浮窗代表，保持它在全局顺序中排在组内其他会话之前；用 session move 或批量计划明确排序。分组的首位没有额外角色字段或“整组完成”状态。浮窗仍按真实成员状态展示提醒。

解散组仅移除关系，保留所有会话、进程、历史和相对顺序。跨组移动和改名／改色不会重启原终端。需要同时建组、移入多个成员、排序时使用 workspace apply，参考[团队编排](teams.md)。
