# External integrations and related repositories

[简体中文](integrations.md) | English

| Integration | Ownership and coordination |
| --- | --- |
| Codex CLI | Uses current/persistent PATH and user installation. CLI owns authentication, trust and hook schema; the manager injects only session-scoped options. Recheck parsing, `/hooks`, and lifecycle after upgrades. |
| Claude Code | Uses current/persistent PATH and user installation. CLI owns authentication, models, plugins and permissions; `--settings` is session-scoped. Model requests require explicit authorization. |
| Qoder CN CLI | Public qodercn/qoderclicn entries, native/npm installs. CLI owns login, models and directory trust; settings are session-scoped and explicit user settings win. Verify synthetic state separately from real startup, without automatic trust approval or model prompts. |
| Kimi Code CLI | Public `kimi` entry. CLI owns models, login and configuration. Stable 0.4.7 has identity only; 0.4.8 preview adds task events as described below. |
| ZCode CLI | Public `zcode` entry; tested with community zcode-app-cli. The distribution owns configuration/authentication. Distinguish community/official results and preview limitations below. |
| TRAE CLI | Public `traecli` entry. CLI owns models, login and configuration. Stable 0.4.7 has identity only; 0.4.8 preview adds session hooks as described below. |
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

User decision on 2026-11-03: prioritize accurate Codex state indicators and defer the shared agents entry. Keep independent mode and session-scoped hooks; do not remove injected arguments, change user configuration, or infer state from output merely to restore for agents. The 0.166.0 startup source still excludes the shared server for incompatible session-level configuration. Restoring the entry must also verify cross-terminal state ownership, conversation switching, and shutdown; see [UI checks](../../docs/ui-rendering.en.md).

Follow-up development uses explicit Codex OSC 7501 lifecycle to remove the conflict between default session overrides and the shared server. Retain the 2026-11-03 deferral as history; the runtime record defines native and explicit --no-daemon compatibility modes. Native lifecycle belongs only to the managed PTY; it does not subscribe to or control other user daemon sessions or collect conversation text/credentials.

0.4.6: Native Codex notifications with app identity can establish identity for the current PTY, including absolute paths, aliases and remote resume entries. Do not rewrite these commands or subscribe to the shared server. Clear does not establish identity or mean exit; this generation's shell prompt clears identity established without a helper. Other Agents still require managed entries.

## Qoder CN CLI (0.4.6)

Recognize the official public `qodercn`/`qoderclicn` commands only, keeping Qoder IDE's `qoder` and international CLI distinct. Reuse PATH, persistent registry PATH and existing npm/user installation directories. Prefer native exe within a directory before cmd/bat; skip only confirmed stale standard npm wrappers.

Session `--settings` points only to a runtime hook file. Exec-form native helpers with `args` avoid a PowerShell launch per event and return no permission decision or context. Register only Qoder events, excluding Claude's PostToolBatch. Preserve explicit user `--settings` without merging private contents. Resume uses `--resume <id>`. Login, models, permissions and directory trust remain CLI-owned; the CLI may block hooks in untrusted directories.

Status follows official SessionStart, UserPromptSubmit, tool/interaction notices, Stop, StopFailure and actual process exit, never terminal-output heuristics or process presence. The bridge still drops child Agents, conversation content and authentication data, with invocation/generation isolation. Qoder source=new can switch foreground conversation. Reuse production drag selection and Alt+V image paste.

Sources: [flags](https://docs.qoder.cn/en/cli/cli-reference), [settings merge](https://docs.qoder.cn/en/cli/settings), [hooks](https://docs.qoder.cn/en/cli/hooks-reference), [shortcuts](https://docs.qoder.cn/en/cli/interface). Record installed-version checks, unsubmitted startup captures, synthetic hooks and real model tasks separately.

## Kimi Code / ZCode / TRAE CLI (0.4.7)

Recognize the installed public entries `kimi`, `zcode` and `traecli`. Session shims forward original arguments and report ShellAgentStarted/ShellAgentEnded with invocation IDs for avatar and launch/exit recognition. They do not parse output or infer task progress. This release does not integrate session-scoped task notifications for these three CLIs, and does not copy or edit user/project hook, provider or authentication configuration. Upstream hook capabilities vary by version; lack of manager integration does not mean a CLI has no hooks.

Installed versions checked for this release are Kimi Code 2.5.0, TRAE CLI 0.213.1 public, and the community zcode-app-cli 3.17.2-44 with runtime 0.17.1. The latter is an independent community terminal frontend, not Z.ai's official standalone CLI distribution; its startup check does not establish compatibility of other ZCode builds. Product-icon provenance is independent of the installed CLI distribution and is recorded per file in assets/icons/SOURCE.json.

Native shell_switch_probe modes `kimi`, `zcode`, `traecli` launch the selected installed entry in a fresh manager workspace, capture startup, and focus the empty terminal and cancel only their own CLI with Esc/Ctrl+C before checking the return-to-shell capture. They never submit a model prompt or accept directory/hook trust. Real startup, synthetic lifecycle and paid model rounds remain separate evidence.

## 0.4.8 preview integration

Kimi Code 2.5.0 uses its pi-tui OSC 7501 `app=kimi-code`. The managed entry enables process-local `PI_PROGRAM_STATUS`, respecting explicit disablement. TRAE CLI 0.213.1 uses its own `-c` TOML hook overrides and CLI trust review; actual events and native startup are verified separately. Community ZCode uses the settings path from its public [Host integration v1](https://github.com/kingsword09/zcode-cli/blob/main/docs/HOST_INTEGRATION.md), never a private vendor entry. Runtime notes describe configuration protection for whole-file replacement and missing events. The community frontend is not represented as the official ZCode CLI.

All three avoid screen-text parsing, automatic login and trust approval. Real model tasks still require user acceptance.

0.4.8-preview.2: community ZCode lacks a public model-failure/interruption end event attributable to the current TUI turn. Following user acceptance feedback, 0.4.8 displays received hook states directly: start events show Working, retaining that state until a later event arrives. Internal turn.failed, notification text and debug logs are not stable integration protocols. An explicit completion hook is required for completed status; abnormal process exit can still report failure. The current enterprise TRAE CLI requires an Enterprise flagship account and PAT; personal subscriptions and third-party API keys do not replace that login.

0.4.9 preview Qoder metadata adds only allowlisted permission_mode and bounded toolName/toolUseId from Notification.details; input, message and unknown modes are discarded immediately. Public CN 1.1.83 installed code and official hook/permission documentation were checked. Synthetic helper events, real CLI startup and user model-task acceptance remain separate evidence.

0.4.11 preview: before launching a real Agent, exclude only this session's shim directory from child PATH so public dispatchers resolving another alias cannot reenter manager wrappers. The outer interactive shell retains its entries; user PATH, CLI files and configuration remain unchanged, and hooks retain absolute helper paths.
