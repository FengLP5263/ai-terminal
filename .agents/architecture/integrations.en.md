# External integrations and related repositories

[简体中文](integrations.md) | English

| Integration | Ownership and coordination |
| --- | --- |
| Codex CLI | Uses current/persistent PATH and user installation. CLI owns authentication, trust and hook schema; the manager injects only session-scoped options. Recheck parsing, `/hooks`, and lifecycle after upgrades. |
| Claude Code | Uses current/persistent PATH and user installation. CLI owns authentication, models, plugins and permissions; `--settings` is session-scoped. Model requests require explicit authorization. |
| PowerShell | Ordinary Windows shell. The manager owns its session process tree, not the user's profile; do not infer AI status. |
| Git Bash | Optional Git for Windows installation. Load system/user startup configuration before adding session-scoped directory notifications and Agent entries; do not edit user files. When absent, explicitly offer available PowerShell/CMD. WSL and other Bash distributions are not substituted. |
| User working directory | The user and that project's rules own its contents. Opening it does not authorize modifying its code. |
| Warp | Manual reference/static REA comparison only, not a runtime dependency or data interface. Do not inject into its process or copy private implementation. |

No other source repository is known as a dependency. Adjacent projects are not automatically in scope. Use relative paths or `REPO_ROOT`, keeping personal workspaces and session IDs in ignored local records.

The hook protocol in `src/status.rs`, `bridge.rs`, and `launch.rs` carries local session, generation, Agent session and turn metadata needed by the reducer, not prompt/output bodies. The helper returns an empty result and makes no authorization decision. Preserve stale-generation/turn/child filtering and deduplication; CLI updates can break hooks, so static parsing alone is not end-to-end verification.

Optional parent_invocation and console_processes correlate nested-shell parents and console membership; older senders default to None. ShellStarted/ShellEnded update only transient avatar identity, never task state. Console process ID lists are not persisted.

Validation order: installed configured CLI → manager/helper build → offline tests → isolated native smoke → user reviews hooks → authorized real turn. Synthetic smoke events, unsent text in real TUI, and actual model turns are different evidence. No cross-repository deployment ordering exists.

Since 0.3.0, session-local `codex`/`claude` entries forward to the installed CLI; launch/exit and task lifecycle are separate. Do not guess identity/status for unsupported launchers. The stable Codex hook reads `AI_TERMINAL_HELPER` rather than embedding an installation path; no user/project trust record is changed. Explicit in-process resume/clear/fork can change the foreground session ID; terminal output is not used for identity or state.

Since 0.3.6, `agent_id` child events do not drive root state; `agent_type` alone does not indicate a child. Claude fullscreen wheel events use its requested mouse protocol without changing the user's renderer preference. Since 0.3.7, Codex keeps its native TUI and own engine; SessionStart may wait until the first task. A real CLI with a loopback fake Responses service must be called an offline fake-model test, not real-model acceptance. Windows image paste forwards native Alt+V after a format check; the CLI handles the image.

0.4.0 adds optional `exit_code` to HookEvent; old senders default to None and managed ShellAgentEnded forwards the real code. Claude StopFailure and non-interrupted nonzero exits mark Failed. A tool error followed by recovery remains Working; user interruption is not failure. Current Codex hooks do not expose StopFailure, so internal errors absent from official events are not guessed from output or silence. Verify details in the CLI. The floating monitor navigates only the current manager's sessions, without taking over another process or changing user CLI trust/configuration.

## Native console and public launchers

Microsoft.Windows.Console.ConPTY is a pinned local runtime dependency with provenance and MIT license in assets/windows/conpty, not a model proxy or related source repository. It runs from files beside the app and follows the existing ConPTY/Job lifecycle.

Literal-target checks for standard npm launchers execute no script, read no CLI authentication/model settings, change no global PATH, and never retry model commands based on arbitrary exit codes. Unknown wrappers retain their original invocation behavior.
