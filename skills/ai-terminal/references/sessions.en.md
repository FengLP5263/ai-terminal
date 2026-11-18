# Session commands

[简体中文](sessions.md) | English

All commands accept --json. Outside the manager, prepend --data-dir <data-directory>. Use full UUIDs, or current inside a managed session.

| Command | Parameters and behavior |
| --- | --- |
| instance info | Manager version, PID, instance ID, data directory, caller session and workspace revision; no credentials |
| session list | --offset N --limit N; default 100, maximum 200; follow next_offset; total is the full count |
| session show ID | Name, directory, Shell, color, group, order, active/running flags, Agent, lifecycle, unread and startup error |
| session create | Required --name NAME --directory PATH; optional --shell powershell/cmd/bash, --color COLOR, --group UUID, --before UUID or --after UUID |
| session rename ID | --name NAME |
| session color ID | --color none/red/green/amber/blue/pink/purple; yellow is a CLI alias for amber |
| session group ID | --group UUID; none removes membership |
| session move ID | Exactly one of --before UUID or --after UUID; moves the live session without restarting it |
| session start ID [--activate] | Start its Shell in the background; preserve an already-running program |
| session activate ID | Select and restore the main window; start an unstarted Shell, preserve a running program, clear unread |
| session run ID | --agent NAME, optional --activate; only an unstarted/exited session can launch an Agent |
| session close ID | Close an unstarted session; a running session requires --force and stops its owned process tree |

PowerShell example:

```powershell
$new = ai-terminal-cli session create --name 'Frontend ✨' --directory $PWD.Path --color blue --json | ConvertFrom-Json
if (-not $new.ok) { throw $new.error.message }
$id = $new.result.results[0].id
ai-terminal-cli session run $id --agent codex --json
ai-terminal-cli session rename $id --name 'Frontend implementation' --json
ai-terminal-cli session activate $id --json
```

run supports codex, claude, qoder (international CLI via its public qodercli entry), qodercn, kimi, zcode and trae; qodercli, qoderclicn and traecli are aliases. It launches the real CLI through the existing session shim and does not persist an autostart command. A manager restart restores an ordinary Shell rather than automatically running an Agent.

Creation defaults to PowerShell and requires an existing directory. It creates navigation metadata without starting a process or modifying project files. Names contain 1–128 non-control characters, with surrounding whitespace trimmed. Relative create directories resolve against the caller's working directory. A live session's directory cannot be rewritten through this API; users can change it inside the Shell.

Closing removes navigation metadata; Agent-owned conversation history remains. Empty groups disappear when their last member closes. Process cleanup is asynchronous: process_cleanup=scheduled does not mean every child has exited. Do not close the caller early in a workflow; it can terminate before reading its own receipt.

Pagination follows the current global order. Check revision between pages and restart if it changes. Use paginated session lists when the full workspace would exceed 1 MiB.
