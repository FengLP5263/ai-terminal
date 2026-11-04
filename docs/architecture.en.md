# Architecture decision: native Windows first

[简体中文](architecture.md) | English

Rust owns the application, PTYs and status bridge. egui/eframe draws a small GPU UI
through glow/OpenGL by default. An optional wgpu/DX12 build is retained for A/B measurement. The terminal component reuses the Apache-2.0
Alacritty parser and ConPTY backend; a pinned MIT egui_term snapshot provides the
input and rendering widget. Local performance/input patches are documented beside it.

This avoids a browser engine and JavaScript runtime in the manager. It does not
make the agents themselves lightweight: their process/memory costs are independent.
Measure the release binary before making performance claims.

## Hot path

ConPTY -> Alacritty parser on its I/O thread -> bounded 5000-line scrollback.
The visible terminal copies only viewport cells into a reused allocation. Hidden
terminals continue parsing output but do not request repaint. Output wakeups are
coalesced to a 16 ms repaint deadline. Sidebar activity animation runs at 10 Hz,
and idle sessions do not schedule animation. No directory/transcript/process scans.

## Status path

Agent lifecycle hook -> `ai-terminal-hook.exe --hook` -> metadata-only authenticated
loopback TCP message -> bounded queue -> UI state reducer, irrespective of selection.
Each launch has a new generation ID; stale process events are ignored. Agent session
and turn IDs reject child-conversation and previous-turn events. A duplicate delivery
cannot repeat state changes. Acknowledging unread results never changes activity.

Hooks return `{}` and never approve, deny or modify tool use. They are injected into
the launched session via Claude's `--settings` and Codex's per-invocation configuration.
No user-level agent configuration, API key or trust record is written. Codex requires
review of non-managed hooks; until it reports events the UI says status not connected.
Process exit is independently detected from ConPTY. The manager owns one Windows job
per session, so closing that session also closes its owned descendants.

Only session names, directories and actual agent resume IDs are persisted, atomically.
Restart restores the list; this early record described a Continue action invoking the agent resume command. See the current runtime record for present startup behavior.
Terminal scrollback and live process state are not restored after the program exits.

## Visible indicators

- Spinner: working.
- Yellow dot: needs input/approval.
- Green dot: completed this turn; this early record associated it with unread status. See the runtime record for present behavior after reading.
- Idle: no badge. Errors/disconnected status: short contextual message.

## Verification and limits

Core tests exercise stale-turn rejection, background completion, unread acknowledgement,
parallel-tool waiting and bridge authentication. Native smoke uses six real PowerShell
ConPTYs with synthetic lifecycle events; it produces a GPU screenshot and JSON evidence.
It is not a live Claude/Codex task test. CLI upgrades can change hook schemas or trust
handling; actual agent versions and live-event verification must be recorded separately.

No heartbeat can prove an agent's semantic progress while it is silently reasoning.
We do not use silence timeouts to manufacture a completed state. Background-task and
scheduled-agent workflows need additional lifecycle coverage beyond this first version.

## Primary references

- https://github.com/alacritty/alacritty
- https://github.com/emilk/egui
- https://github.com/kemokempo/egui_term/tree/31bbc7ab8503c9518fcee5717cfa29011e59f451
- https://learn.chatgpt.com/docs/hooks
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/cli-reference

The completion badge means the current agent turn ended. A persistent dev server or
other background process does not keep the working spinner alive. A new autonomous
turn reports UserPromptSubmit and resumes the spinner. It is not project acceptance.

## Renderer selection

The same Windows machine was tested with the same GUI, fonts and three-terminal
fixture. The initial DX12 debug build selected Intel UHD 630 and used about 721 MiB
resident; OpenGL selected Quadro RTX 4000 and used about 117 MiB. This is a measured
configuration choice on this machine, not a same-adapter backend-only comparison.
No universal fastest-renderer claim is made. Release measurements are separate.

Default: `cargo build --release`.
Alternative: `cargo build --release --no-default-features --features renderer-wgpu`.
Switching build features changes `target/release/ai-terminal.exe`; keep measurements
in separate output directories and never compare a stale binary.

Agent events are consumed in eframe's logic callback, which also runs when painting
is suspended. Minimized sessions disable terminal redraw wakeups but still reduce
hook events. A native test delivers completion while minimized and verifies the
unread result before any UI paint, preventing focus-dependent status refresh.
