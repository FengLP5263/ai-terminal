# ai-terminal collaboration rules

[简体中文](AGENTS.md) | English

## Scope and boundaries

Windows-native Agent terminal manager. The project focuses on project/session navigation, a real terminal, reliable status indicators, and conversation resumption. It does not implement a model proxy, store API keys, or automatically handle CLI trust or authorization. Stack: Rust, egui/eframe, Alacritty, and ConPTY; no browser runtime.

## Required reading

- All work: [architecture](.agents/architecture/overview.en.md) and [runtime and configuration](.agents/architecture/runtime.en.md).
- Data, fonts, and artifacts: [data and assets](.agents/architecture/data-and-assets.en.md).
- Agent protocols and related projects: [integration boundaries](.agents/architecture/integrations.en.md).
- Before a workflow, read its project skill: [development](.agents/skills/ai-terminal-development/SKILL.en.md), [testing](.agents/skills/ai-terminal-testing/SKILL.en.md), [packaging](.agents/skills/ai-terminal-packaging/SKILL.en.md), or [Git collaboration](.agents/skills/ai-terminal-git-workflow/SKILL.en.md).

## Source and compatibility

- Production source is `src/`, `vendor/egui_term/src/`, and `vendor/egui-winit/src/`; fonts are in `assets/fonts/`. Do not rearrange production source for governance.
- `src/bin/ai-terminal-hook.rs` must stay independent and lightweight, without initializing fonts or the GPU.
- Derive status only from Agent lifecycle events and process exit; do not infer completion from silence, a bell, or process existence.
- Selecting an active session preserves its process; an unopened session opens its shell directly and clears unread state. A green dot marks the end of a turn, not project acceptance.
- Consume status in background logic. Hidden terminals must continue draining output without repainting.
- Preserve bounded queues, 5,000 lines of history, coalesced wakeups, and viewport snapshots; avoid full-history copying, polling scans, and idle redraws.
- Inject hooks only for the session. Do not change user-level configuration, save prompts, results, or credentials, or automatically trust projects or hooks.
- Retain vendored provenance and licenses; update the affected vendor `UPSTREAM.md` when patching it.
- Keep font provenance and SHA-256 current. The application and probes share `src/appearance.rs`.

## Git and files

- Use `develop` for daily work; `main` was established from the same initial baseline. Independent features use `feature/<name>` and fixes use `fix/<name>`.
- Changes enter `main` through review. Commit, remote, tag, PR merge, and release actions follow the user's current authorization separately.
- Do not track real `.env` files, private keys, user configuration, databases, output, runtime data, or handoff archives. Optional machine-local `.tools/env.ps1` and `.tools/cargo-network.toml` remain ignored.
- Use relative paths or environment variables in documentation; do not include developer usernames or personal workspace paths.
- Maintain current project-authored documentation in Chinese and English, with reciprocal links and governance checks as described in the [bilingual policy](docs/i18n.en.md); the ledger records historical translation progress.
- Do not stop user-owned terminals. Tests manage only their own processes and isolated data directories.
- Do not create extra worktrees or start subagents unless the user explicitly requests parallel work.

## Authoritative commands

See [scripts/README.en.md](scripts/README.en.md). Windows:

```powershell
.\scripts\windows\project.ps1 governance
.\scripts\windows\project.ps1 check
.\scripts\windows\project.ps1 test
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 package
```

The POSIX counterpart is `sh scripts/posix/project.sh <action>`. Native builds currently support Windows only; unsupported macOS/Linux native actions must fail explicitly. Static governance and ZIP verification can run cross-platform. Use UTF-8 text I/O in PowerShell; save `.ps1` files containing Chinese with a UTF-8 BOM. Put common commands first in CLI help.

## Completion

There is no other source-repository dependency. Codex, Claude Code, and PowerShell are external CLIs; user-selected working directories and adjacent projects are not automatically in scope. Document the change and run governance, fmt, Clippy, and relevant tests. Terminal, UI, and lifecycle changes additionally require native smoke and actual screenshots. Distinguish real Agent checks from synthetic events. Update affected architecture, data, protocol, scripts, and project skills. Testing does not replace visual and input acceptance; do not invent a release workflow.
