# Native management CLI

[简体中文](cli.md) | English

Starting with 0.14.0, ai-terminal-cli.exe lets people and Agents manage a running AI Terminal. It ships beside the manager and hook helper; managers through 0.13.0 do not expose this interface. Full Windows releases include the CLI; the Agent Skill is also available as a separate ZIP.

## Quick start

Open the matching ai-terminal.exe, then inside its Shell/Agent:

```powershell
ai-terminal-cli instance info --json
ai-terminal-cli session list --json
ai-terminal-cli session rename current --name 'Coordinator' --json
```

Create and explicitly launch a member:

```powershell
$r = ai-terminal-cli session create --name 'Frontend ✨' --directory $PWD.Path --color blue --json | ConvertFrom-Json
if (-not $r.ok) { throw $r.error.message }
ai-terminal-cli session run $r.result.results[0].id --agent codex --json
```

Creation does not start a process. session start launches a Shell; session run launches a selected Agent only when no terminal is running. A restart does not automatically run Agents again. Login, trust and model tasks retain the CLI's own interaction; the manager does not submit prompts or approve permissions.

External callers use the full executable path and, if needed, --data-dir to select manager metadata, not the project directory. User PATH is unchanged; owned child processes temporarily receive the executable path and AI_TERMINAL_CLI. current identifies the caller, not GUI selection. External callers query full UUIDs first.

## Commands and teams

- [Session reference](../skills/ai-terminal/references/sessions.en.md): query, create, rename, color, start, activate, close.
- [Groups and ordering](../skills/ai-terminal/references/groups.en.md): the representative follows global session order.
- [Team plans](../skills/ai-terminal/references/teams.en.md): create members, reference the primary session, group and order, with dry-run and revision checks.
- [Protocol/troubleshooting](../skills/ai-terminal/references/protocol.en.md): JSON, exit codes, receipts, instance selection and limits.
- [Install/use the Agent Skill](skills.en.md).

Batches validate all metadata before atomic persistence and UI synchronization; failures do not leave partial teams. Existing terminals retain their processes. Startup/closure are separate commands; closing a running session requires --force. Dissolving keeps members. This version provides explicit terminal reads/input when needed for a task, without retaining content or proxying models.

## Native operation coverage

0.14.0 extends the CLI across the workspace's main native operations. Confirm native_controls using `instance capabilities` first:

```powershell
ai-terminal-cli floating set --enabled true --transparency 50
ai-terminal-cli settings set --language en --scale 114
ai-terminal-cli panel open files
ai-terminal-cli file open --path README.md
```

- [Settings/windows](../skills/ai-terminal/references/settings.en.md): all visible settings, floating enable/transparency/border/zoom/position, sidebar, window state and capture.
- [Files/Git/media](../skills/ai-terminal/references/viewing.en.md): lazy tree, paginated reads, rendered Markdown, image zoom, media playback and read-only Git.
- [Terminal interaction](../skills/ai-terminal/references/terminal.en.md): explicit viewport reads, text/keys/local scrolling, stop/restart and unread acknowledgement.

Commands reuse native implementations. Settings validate/save before immediate application. Files/Git/media queue asynchronous work; inspect loading/error and actual results. The CLI does not operate system file pickers, installation/uninstallation, other applications or the user's clipboard, and adds no Git writes absent from the GUI. See `--help` for the complete command list.

## Build and verify

```powershell
.\scripts\windows\project.ps1 build
.\target\debug\ai-terminal-cli.exe --help
```

Builds produce three distinct entry points. See [Development scripts](../scripts/README.en.md) and [Runtime/configuration](../.agents/architecture/runtime.en.md).
