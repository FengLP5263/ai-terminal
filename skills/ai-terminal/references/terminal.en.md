# Explicit terminal interaction and session lifecycle

[简体中文](terminal.md) | English

Requires native_controls. Use terminal content only when needed for the task; ordinary grouping, colors and renames need no screen reads. Target full UUIDs; managed callers may use current (caller, not GUI active). Commands address only real PTYs owned by this manager.

```powershell
ai-terminal-cli terminal read <UUID> --offset 0 --limit 100
ai-terminal-cli terminal write <UUID> --text 'Text to enter'
ai-terminal-cli terminal write <UUID> --file input.txt
ai-terminal-cli terminal write <UUID> --file -
ai-terminal-cli terminal key <UUID> --key escape
ai-terminal-cli terminal scroll <UUID> --lines 20
ai-terminal-cli terminal scroll <UUID> --offset 0
```

read returns the current visible grid, scope=current_viewport, without copying the entire 5,000-line history. offset/limit paginate that screen response. Use scroll then read for history: positive lines moves toward older output, offset=0 returns to the bottom. Screen text is not a completion signal; session show's native activity remains authoritative.

write accepts at most 64 KiB of UTF-8 without NUL. Choose --text or --file; file `-` reads stdin. By default it only enters text. Explicit --submit also sends Enter and may execute a Shell command or submit a model task, so it must fit the user's authorized scope. Never automatically answer trust/login/permission dialogs. Do not inject arbitrary input into an unknown running program to start an Agent; use session run.

Text follows terminal paste mode (ESC stripped in bracketed paste; ordinary newlines normalized to CR). This is not an attachment API or system clipboard simulation. Keys: enter, escape, tab, backspace, up/down/left/right, home/end, page_up/page_down, ctrl_c/ctrl_d/ctrl_l/ctrl_z. Arrow sequences respect application cursor mode. scroll changes only local history, sending no CLI mouse/arrow input. Choose --lines or --offset, bounded to 5,000.

Input/read contents are transient bounded request/response data, never manager logs, workspace data or receipts. Mutation receipts store SHA-256 request fingerprints and acknowledgements; read contents are excluded. Agents/Shells may retain their own inputs under their settings, which the manager does not change. Avoid sensitive content in command logs; use stdin/files if needed and manage files under the caller's own policy.

```powershell
ai-terminal-cli session acknowledge <UUID>
ai-terminal-cli session stop <UUID> --force
ai-terminal-cli session restart <UUID> --force
ai-terminal-cli session start <UUID>
```

acknowledge clears unread without selecting. stop ends the owned process tree but keeps UUID, group, color and order; restart creates a new Shell. Stopping/restarting a running terminal requires --force; restart of an unstarted session does not. To relaunch an Agent, explicitly stop the old program then session run; run refuses an already running Shell. process_id identifies the actual PTY; running is not task readiness/completion. session close additionally removes navigation metadata.

Newlines in multiline text can themselves execute commands in ordinary terminal mode. --submit controls an additional final Enter; it does not escape Shell syntax.
