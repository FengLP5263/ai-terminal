# Groups and ordering

[简体中文](groups.md) | English

```powershell
ai-terminal-cli group list --json
ai-terminal-cli group create --name 'Engineering' --members 'current,<member-UUID>,<member-UUID>' --json
ai-terminal-cli group rename <group-UUID> --name 'Product engineering' --json
ai-terminal-cli session group <session-UUID> --group <group-UUID> --json
ai-terminal-cli session group <session-UUID> --group none --json
ai-terminal-cli session move <session-UUID> --after <other-session-UUID> --json
ai-terminal-cli group dissolve <group-UUID> --json
```

Replace angle-bracket placeholders before running. Obtain UUIDs from list/create results rather than guessing from non-unique names. members is comma-separated without whitespace. Groups require 1–128 valid distinct members. Creating a group moves those sessions from their previous groups; there are no empty placeholder groups. Removing the final member removes the old empty group.

Member display order follows the global session list, not the members argument order. Keep the primary session before the other members globally so it becomes the floating-bar representative; use session move or a batch plan to make this explicit. There is no additional leader-role field or group-completed state. Floating reminders still reflect actual member lifecycle states.

Dissolving removes only membership, preserving sessions, processes, histories and relative order. Reassignment, renaming and recoloring do not restart terminals. Use workspace apply to create a group, assign multiple members and order them together; see [Team orchestration](teams.en.md).

Unreleased floating aggregation retains the group's first avatar and prioritizes attention (unread failure, blocking input or unseen questions), then unread completion, then running work. Viewed completion does not obscure running work. An asynchronous question's yellow unread marker is independent of actual Working state and implies neither blocking nor completion of the team.
