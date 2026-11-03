# Changelog

[简体中文](CHANGELOG.md) | English

Record user-visible additions, improvements, fixes, and compatibility changes for each release; omit categories that do not apply. Review both languages together, use the same version entry in GitHub Releases, and ship both files in the ZIP. Historical summaries follow existing GitHub release notes and UTC publication dates; development versions without a separate release do not receive invented release dates.

## [Unreleased]

No confirmed changes for the next version yet.

## [0.4.3] - 2026-11-03

### Added

- A bilingual changelog with published-release summaries. Packaging checks the current version entry so new archives cannot omit release notes.

### Changed

- Session avatars follow the active shell when entering or exiting nested CMD, PowerShell, and Git Bash sessions.
- Sidebar scrollbars stay thin beside the divider, changing opacity on hover or scroll. Panel resizing and scrollbar dragging no longer compete.

### Fixed

- Git Bash loads system and user startup configuration while preserving prompts, aliases, and completion. Portable/registered installations are discovered, with an explicit PowerShell/CMD fallback when unavailable.
- Codex/Claude Code text supports ordinary left-drag selection followed by Ctrl+C to copy. Without a selection, Ctrl+C still interrupts; clicks and application scrolling remain available.
- Bundled, pinned Microsoft-signed console components address missing fullscreen and mouse control information with the older Windows 10 console runtime.
- Native captions remain dark under a light system theme and retain that policy after theme notifications. [#13](https://github.com/FengLP5263/ai-terminal/issues/13)
- Discovery continues past standard npm launchers with missing targets to find a working Codex/Claude entry. Custom scripts retain precedence and global PATH is unchanged. [#11](https://github.com/FengLP5263/ai-terminal/issues/11)

### Upgrade and verification scope

- On Windows x64 (Windows 10 1809/build 17763 or later), extract the complete ZIP and keep `ai-terminal.exe`, `ai-terminal-hook.exe`, `conpty.dll`, and `OpenConsole.exe` together. Rust, Python, and Electron are not required to run it.
- Windows 10 22H2 and the local Windows environment passed native checks and user preview trials. Regression coverage includes 125%, 150%, and 151.13608% zoom, captions, selection, the sidebar, and the floating window.
- The original high-zoom composer trigger has not been reliably reproduced, although current regressions and user trials passed. [#12](https://github.com/FengLP5263/ai-terminal/issues/12) remains tracked; synthetic responses are not real multi-turn Agent acceptance. See the [verification record](docs/windows10-compatibility.en.md).

## [0.4.2] - 2026-11-02

### Changed

- Reuse Windows OpenGL buffers and reduce repeated rendering/input work during long sessions.
- Save and close owned sessions in the background to improve responsiveness when exiting with multiple sessions.

### Fixed

- Address memory growth and input lag on some devices, and stale composition state after IME focus changes.
- Refine settings/rename dialogs and record graphics-patch provenance and licenses.

## [0.4.1] - 2026-11-02

### Changed

- Improve layout when moving between displays with different scaling; refresh bilingual READMEs, screenshots, and setup guidance.

### Fixed

- Clear unread completion markers when viewed; reduce false Codex/Claude attention signals during automatic approval and related flows.
- Request dark native captions; further Windows 10 fixes are included in 0.4.3.

## [0.4.0] - 2026-11-01

### Added

- Settings for automatic system language or manual Simplified Chinese/English selection.
- An optional always-on-top session rail with avatars and status; ten visible sessions, with dragging to reach more.

### Changed

- Scale and center the floating window on the current display, handle different DPI settings, and use the approved application icon consistently.
- Derive state from lifecycle notifications and actual process exits, without guessing completion from quiet output.

## [0.3.7] - 2026-11-01 (prerelease)

### Fixed

- Restore Codex's native fullscreen TUI, startup screen, bottom composer, and task notifications; improve Claude application scrolling.
- Restore native Codex/Claude image paste, prevent inherited stale managed launchers, and include dependency/license inventories.

## [0.3.4] - 2026-11-01 (prerelease)

### Added

- Shell-first sessions with PowerShell, CMD, Git Bash, recent directories, and Agent avatars identified from launch commands.
- Session colors, dividers, colorful emoji names, and the B stacked-session application icon.

### Fixed

- Prevent wheel input from navigating input history and improve public CLI discovery across devices.

## [0.2.1] - 2026-10-30 (prerelease)

### Added

- First public preview: multiple Agent sessions, status indicators, reordering, renaming, collapsible sidebar, and whole-window zoom.
- Windows-native text, Chinese/color emoji, and saved session metadata and UI preferences.

[0.4.3]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.3
[0.4.2]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.2
[0.4.1]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.1
[0.4.0]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.4.0
[0.3.7]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.3.7
[0.3.4]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.3.4
[0.2.1]: https://github.com/FengLP5263/ai-terminal/releases/tag/app-v0.2.1
