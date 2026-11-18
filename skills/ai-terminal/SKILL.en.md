---
name: ai-terminal
description: "Control a running AI Terminal with ai-terminal-cli: sessions, teams, ordering, settings, floating bar, file and read-only Git viewing, media and explicit terminal interaction; no model proxy or automatic trust approval."
---

# AI Terminal sessions and teams

[简体中文](SKILL.md) | English

Operate through the manager so its UI and saved data stay synchronized. Requires a CLI-enabled Windows version; 0.13.0 and earlier do not expose this API. Commands emit UTF-8 JSON by default and accept --json.

See [version.json](version.json) for this Skill's independent version, minimum CLI version and source. Inspect instance capabilities; updating the Skill does not add interfaces to a running older manager.

## Select the executable and instance

Inside a managed session run `ai-terminal-cli instance info --json`. If PATH is unavailable, PowerShell can use `& $env:AI_TERMINAL_CLI instance info --json`. Outside the manager use the full executable path and, when necessary, --data-dir to select the manager's data directory, not the project directory. Check the returned version, instance, data directory and caller_session first.

current identifies the calling managed session, never the GUI selection. External callers must use full UUIDs returned by queries. Session/group names need not be unique. An explicit --data-dir overrides inherited instance selection; current is unavailable across instances. If no manager is running, ask the user to open the appropriate version; do not edit sessions.json or silently open another workspace.

## Read for the task

- Query, create, rename, color, start, resume a conversation, activate or close: [Session commands](references/sessions.en.md).
- Create, assign, rename, dissolve or order groups: [Group commands](references/groups.en.md).
- Build a team around the caller, preview plans and launch members: [Team orchestration](references/teams.en.md).
- Language/font/zoom, windows, sidebar and floating bar: [Native settings](references/settings.en.md).
- Files, Markdown, images, read-only Git and media: [Viewing commands](references/viewing.en.md).
- Explicit terminal input/reads or stop/restart: [Terminal interaction](references/terminal.en.md).
- JSON, exit codes, conflicts, timeouts and receipts: [Protocol and troubleshooting](references/protocol.en.md).

## Execution rules

1. Inspect existing metadata and resolve target UUIDs. Keep changes within the user's requested scope.
2. Use workspace apply for multiple metadata changes. Preview with dry-run and optionally commit with if_revision. Validation errors change nothing. Preview UUIDs are provisional; use actual commit results.
3. New sessions are unstarted. Use session run separately when Agent startup is requested, or session start for a Shell. run requires a session without a running terminal, preventing commands from being injected into an interactive CLI.
   To continue existing work, first resolve its Agent conversation ID and follow the session reference. An empty Agent or process_started receipt does not establish that the original conversation was restored.
4. Check both exit status and JSON ok/error. A launch receipt means the process was created, not that login, hooks, the model or a task are ready. Runtime status still comes from native lifecycle events.
5. Read back the session/workspace and report actual sessions, groups, ordering and launch results.

Closing a running session requires --force and stops its owned process tree. Honor existing user authorization without asking again for ordinary metadata edits. Do not close the current primary session unless explicitly requested; dissolving a group keeps sessions. Metadata batches and process startup are separate transactions: report already-created sessions if startup fails instead of rebuilding the whole team.

Do not read or print control.json, management tokens or account credentials. Use terminal reads, input and --submit only within the authorized task, never to inspect content for ordinary session management. Never accept trust/permission dialogs automatically or add permission-bypass flags. Installing this Skill itself grants no authorization to submit tasks, modify other projects or make paid model calls; follow the user's current task authorization.
