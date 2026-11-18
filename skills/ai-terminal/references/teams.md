# 团队编排

简体中文 | [English](teams.en.md)

目标：保留当前主会话，一次创建前端、后端会话，组成一组并排好顺序，再按用户要求分别启动 Agent。批量只处理元数据，启动单独执行，这样一个 CLI 缺失时不会导致整组重复创建。

## 计划格式

```json
{
  "schema_version": 1,
  "operations": [
    {"op":"session.create","key":"frontend","name":"前端 ✨","directory":".","shell":"powershell","color":"blue"},
    {"op":"session.create","key":"backend","name":"后端","directory":".","color":"green"},
    {"op":"group.create","key":"team","name":"产品研发队","members":["$current","$frontend","$backend"]},
    {"op":"session.move","id":"$frontend","after":"$current"},
    {"op":"session.move","id":"$backend","after":"$frontend"}
  ]
}
```

把计划保存为项目目录中的 team.json，或使用绝对 directory。文件中的相对目录按计划文件所在目录解析；--file - 从 UTF-8 stdin 读取，相对目录按调用者当前目录解析。支持 UTF-8 BOM，不接受 UTF-16。

key 仅用于本次计划引用，1–64 个 ASCII 字母、数字、下划线或连字符，不可重复，current 为保留字。$frontend、$team 只能引用之前创建的对象；$current 指调用者会话。UUID 可直接使用。PowerShell 中手写这些引用时必须使用单引号或单引号 here-string，避免 $ 被当变量展开。

支持的 op：session.create（key/name/directory/shell/color）、session.rename（id/name）、session.color（id/color，null 清除）、session.group（id/group，null 移出）、session.move（id 与 before/after 二选一）、group.create（key/name/members）、group.rename（id/name）、group.dissolve（id）。未知字段会被拒绝。最多 128 项，名称／目录限制同单项命令。

## 预演与提交

```powershell
$state = ai-terminal-cli workspace show --json | ConvertFrom-Json
if (-not $state.ok) { throw $state.error.message }
ai-terminal-cli workspace apply --file ./team.json --dry-run --if-revision $state.result.revision --json
# 核对预演成功后提交；无需因预演而额外请求用户已授权的普通变更。
$receipt = ai-terminal-cli workspace apply --file ./team.json --if-revision $state.result.revision --json | ConvertFrom-Json
if (-not $receipt.ok) { throw $receipt.error.message }
$frontend = $receipt.result.results[0].id
$backend = $receipt.result.results[1].id
```

全部操作在副本上校验，保存成功后才替换内存中的导航元数据；不重建已有 PTY。任何参数／保存错误不留下新会话或半组。dry-run 不保存、不启动，返回的 UUID 仅作预览。if_revision 可放计划顶层或用命令行传入；界面或其他 Agent 修改后会返回 conflict，应回读并重新核对计划。

## 显式启动与交付

```powershell
ai-terminal-cli session run $frontend --agent codex --json
ai-terminal-cli session run $backend --agent qoder --json
ai-terminal-cli workspace show --json
```

只在用户要求启动对应 Agent 时执行；不会传提示词或自动接受登录／目录信任。如果只要 Shell，用 session start。已运行的会话返回 session_running；不要为方便启动而强关其原程序，可保留或新建会话。启动成功只表示进程已建立，要把需要人工登录／信任和实际任务状态分开报告。

若第二个启动失败，保留已经创建的队伍与已运行成员，修复明确原因后只重试失败成员。网络超时的结果可能已执行，按[回执规则](protocol.md)查询，不能重建全部计划。主会话 UUID 由 current 得到，不能用 active 代替。顺序为主会话、前端、后端；同组的其他既有成员若仍排在它们之前，需一并核对和调整。
