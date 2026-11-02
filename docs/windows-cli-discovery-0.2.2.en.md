# 0.2.2 Windows CLI discovery fix

[简体中文](windows-cli-discovery-0.2.2.md) | English

0.2.1 did not embed the developer's personal path, but searched only for `codex.exe`/`claude.exe` and assumed a fixed npm-internal binary layout. That missed common `codex.cmd` installations, custom npm prefixes, other package-manager launchers, and stale PATH in desktop processes. The reported user's specific device was not examined; this records a confirmed code defect and isolated reproduction, not its sole proven cause.

## Fix

- Read current PATH and Windows registry system/user PATH, retaining directory priority. Search public exe/cmd/bat entries and user installation locations instead of npm private vendor paths.
- Give launchers and Node the same child PATH without changing the manager/user environment, executing profiles, or scanning disks.
- Launch cmd/bat through the lightweight helper's `--launch-cli` using Rust Command's batch escaping. Preserve terminal I/O, exit codes, and Job Object ownership. The console-subsystem helper inherits the terminal, preserves the metadata hook protocol, and initializes neither fonts nor GPU. Distribution still contains two EXEs.
- Explain discovery scope, custom directories, and WSL limitations when a CLI is not found.

## Isolated verification

Tested different user roots, custom prefixes, directory precedence for exe/cmd, persistent PATH supplementation, and empty/relative directory filtering. Real cmd in Chinese paths containing spaces, quotes, `&`, `%`, and `!` preserved complex hook arguments, environment characters and quotes, and passed exit code 37 through unchanged. A real ConPTY launched cmd → a native test program, accepted Chinese input, and reaped only the test's process tree. The installed Codex public npm entry accepted session-scoped hook arguments and `features list` in an isolated `CODEX_HOME` without model calls or trust/auth changes.

## Final checks and local artifact (2026-10-30)

- Governance, fmt, Clippy with warnings denied, 51 Rust and 3 Python tests passed (54 total).
- Optimized real Codex at `output/codex-v022-trusted-20261030-182148`: launched through public npm entry and helper, visible Chinese input, reviewed PNG, clean exit and owned PTY cleanup; no Enter/model request. Earlier probes of an untrusted directory were blank/not ready and did not pass. Existing trust was read-only checked before the trusted-directory probe; no automatic trust was granted. A screenshot counts only when input is actually visible.
- Optimized native smoke at `output/smoke-20261030-182211-115` passed real PowerShell, synthetic status, screenshot, exit and owned PTY cleanup.
- Local ZIP `output/packages/ai-terminal-v0.2.2-windows-x64-20261030T102221Z-1a7291.zip`: 317 members, read-back verified; SHA-256 `3e13c96e7dc1007740cfea1f5d66e223514724eff245e3476692aefe121e566d`. Main EXE 8,908,288 bytes, helper 292,352 bytes; helper initializes neither fonts nor GPU.
- The fix was uncommitted on `fix/windows-cli-discovery` when the test package was built; its `dirty=true` describes that build and cannot be changed by a later commit. This record did not create a 0.2.2 tag or remote release; `app-v0.2.1` and its asset were unchanged. Production source identity: `95a7f86505fec9dcfb1b045b6e54e3627dfa271730025e45a590347e04d60965`.

References: [npm folders/prefix](https://docs.npmjs.com/cli/v11/configuring-npm/folders/), [Windows environment inheritance](https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables), [Rust Command Windows arguments](https://doc.rust-lang.org/std/process/struct.Command.html).
