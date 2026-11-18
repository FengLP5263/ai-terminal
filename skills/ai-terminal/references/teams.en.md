# Team orchestration

[简体中文](teams.md) | English

Keep the caller as the primary session, create frontend/backend sessions, group and order them together, then explicitly launch requested Agents. Metadata batches and process startup are separate so a missing CLI does not cause duplicate teams.

## Plan format

```json
{
  "schema_version": 1,
  "operations": [
    {"op":"session.create","key":"frontend","name":"Frontend ✨","directory":".","shell":"powershell","color":"blue"},
    {"op":"session.create","key":"backend","name":"Backend","directory":".","color":"green"},
    {"op":"group.create","key":"team","name":"Product engineering","members":["$current","$frontend","$backend"]},
    {"op":"session.move","id":"$frontend","after":"$current"},
    {"op":"session.move","id":"$backend","after":"$frontend"}
  ]
}
```

Save team.json in the project directory, or use absolute directory values. Relative directories resolve against the plan file's parent. --file - reads UTF-8 stdin and resolves relative paths against the caller's working directory. A UTF-8 BOM is accepted; UTF-16 is not.

key is local to a plan: 1–64 ASCII letters, digits, underscores or hyphens, unique, with current reserved. $frontend/$team can reference only earlier creations. $current identifies the caller session; UUIDs can be used directly. Use single-quoted PowerShell strings/here-strings for literal $ references.

Supported op values: session.create (key/name/directory/shell/color), session.rename (id/name), session.color (id/color, null clears), session.group (id/group, null removes), session.move (id plus exactly one of before/after), group.create (key/name/members), group.rename (id/name), group.dissolve (id). Unknown fields are rejected. Maximum 128 operations; names/directories use the same constraints as individual commands.

## Preview and commit

```powershell
$state = ai-terminal-cli workspace show --json | ConvertFrom-Json
if (-not $state.ok) { throw $state.error.message }
ai-terminal-cli workspace apply --file ./team.json --dry-run --if-revision $state.result.revision --json
# Verify the preview, then perform the already-authorized metadata change.
$receipt = ai-terminal-cli workspace apply --file ./team.json --if-revision $state.result.revision --json | ConvertFrom-Json
if (-not $receipt.ok) { throw $receipt.error.message }
$frontend = $receipt.result.results[0].id
$backend = $receipt.result.results[1].id
```

The entire plan is validated on a copy; navigation metadata changes in memory only after saving succeeds. Existing PTYs are moved intact. Validation/save errors leave no new sessions or partial groups. dry-run neither saves nor launches; its UUIDs are provisional. if_revision can be supplied in the plan or on the command line. A GUI/Agent metadata edit causes conflict; read and review the plan again.

## Explicit startup and delivery

```powershell
ai-terminal-cli session run $frontend --agent codex --json
ai-terminal-cli session run $backend --agent qoder --json
ai-terminal-cli workspace show --json
```

Run only Agents the user requested. No prompt or automatic login/trust approval is submitted. Use session start for a Shell. A running target returns session_running: preserve it or create a new session instead of forcibly closing it merely for convenience. A successful launch means process creation, not task readiness; report login/trust needs separately from real lifecycle state.

If the second launch fails, keep the created team and running member, resolve the specific cause, and retry only the failed member. A timeout may follow successful execution: follow [receipt recovery](protocol.en.md), not a wholesale plan retry. Identify the primary session with current, never active. The intended order is primary, frontend, backend; inspect any existing earlier group members as well.
