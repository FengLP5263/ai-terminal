# Installation and font selection

[简体中文](installation.md) | English

## Installed builds

Choose `*-setup-*.exe` from the matching release assets and follow the wizard. It installs for the current user, defaulting to `%LOCALAPPDATA%/Programs/AI Terminal`, without administrator privileges. The Start menu contains launch/uninstall entries; the desktop shortcut is optional. Launch after setup is unchecked by default. The portable ZIP remains available; extract it completely before running.

Both formats share sessions/settings at `%LOCALAPPDATA%/ai-terminal`. Application files and data stay separate. Setup does not install Agent CLIs, edit their configuration or install system fonts. Busy files stop upgrade/uninstall and ask the user to close the corresponding program; tasks are never terminated automatically. Uninstall deletes listed application files and retains sessions, settings and unrelated user files. Nonempty unrecognized folders, the session-data root and directory links are rejected as installation targets.

Installed builds carry `.ai-terminal-installed` in their program directory and leave desktop-shortcut ownership to Setup; ZIP builds retain their original first-run behavior. Upgrades reuse the registered directory by default; an explicit `/D=` override takes priority. Installation records use only the current user's Windows uninstall registry. No background service is required.

Version 0.4.10 introduces installed builds alongside the portable ZIP. Public availability follows actual GitHub Release assets; automatic update checking and code signing are not integrated.

## Terminal fonts

Choose Settings → Terminal font. Changes apply to all sessions immediately and persist without restarting terminals or Agents.

| Font | Source | Notes |
| --- | --- | --- |
| JetBrains Mono | Four original embedded styles | Default, v2.304, SIL OFL 1.1 |
| Cascadia Mono | Four original embedded styles | Matching 0.4.9 |
| Hack | Four original embedded styles | Matching 0.4.8 |
| Consolas | Installed Windows font | Disabled when unavailable |
| Courier New | Installed Windows font | Disabled when unavailable |

Chinese keeps system Microsoft YaHei UI and emoji keeps the system color font. This setting changes terminal text and, from 0.5.0, the terminal header and Git panel. Interface language and other controls remain unchanged. System fonts are not copied into application/installer files. New workspaces, older settings without a font field and unknown future font IDs use JetBrains Mono. Existing saved choices are preserved, and unavailable fonts from another PC safely fall back to the default. Known system files are checked once; no complete font scan or per-frame disk reads are used.

## Local build and acceptance

See the [script catalog](../scripts/README.en.md) for commands and pinned NSIS preparation. `installer` first builds and verifies a ZIP, or accepts an existing ZIP through `-Path`; dirty previews require explicit `-AllowDirty`. The installer also has an EXE digest, checksum file and JSON record identifying the ZIP source, installer scripts and compiler. `verify-installer` checks only the EXE digest; it does not execute Setup or claim signature verification.

`-Acceptance` uses separate registry/shortcut identities. `installer-smoke` accepts only these builds and tests fresh install, busy-file rejection, upgrade, installed six-PTY smoke, data-preserving uninstall and its own registry cleanup under unique `output/` paths containing Chinese and spaces. Build old/new packages with the same `-AcceptanceId` and pass `-Previous` for a real version transition. Failed artifacts remain available; failed screenshots or incomplete uninstalls do not count as passing evidence.
