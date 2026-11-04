# Changelog

[简体中文](CHANGELOG.md) | English

Record user-visible additions, improvements, fixes, and compatibility changes for each release; omit categories that do not apply. Review both languages together, use the same version entry in GitHub Releases, and ship both files in the ZIP. Historical summaries follow existing GitHub release notes and UTC publication dates; development versions without a separate release do not receive invented release dates.

## [Unreleased]

No confirmed changes for a subsequent version yet.

## [0.4.5] - 2026-11-04

### Changed

- Preserve Codex's shared server and `← for agents` by default. Explicit native OSC 7501 lifecycle from the current terminal updates status, including hidden sessions. Explicit `codex --no-daemon` retains the legacy hook integration.
- Show a full session card at the pointer's original grab point while the list makes space. Release commits the order; Escape or release outside cancels. Holding a card at an overflowing list's edge scrolls further. Hide hover tooltips during dragging so they do not cover the card.

### Fixed

- Treat failure red dots and completion green dots as unread badges: viewing clears them from the sidebar and floating rail while retaining the failed/completed state. Record unread state for startup failures and abnormal process exits too.
- Give adjacent narrow Claude mode badges equal visual widths, fixing mismatched double-triangle sizes. Preserve isolated badges, combined emoji and VS15 text presentation.

### Upgrade notes and verification limits

- Native lifecycle and agents navigation were checked with Codex 0.166.0. Older CLIs without native events can use `codex --no-daemon` and review compatibility hooks in `/hooks`; that mode has no shared agents entry. Existing CLIs must be exited and relaunched to use the new integration; the manager does not restart sessions or alter user configuration automatically.
- Codex owns shared background tasks. Closing the manager reaps only its terminals and helpers; tasks in the shared server may continue running.
- Passed 161 Rust and 13 Python regressions, governance, formatting, Clippy, Release smoke and native UI captures; the user accepted the local preview. Synthetic lifecycle, real CLI navigation and actual model tasks are recorded separately. No paid model turns, long-duration load measurements or remote Windows 10 retest were run for this release.

## [0.4.5-preview.2] - 2026-11-04

Local feature preview; not a published release. Includes preview.1's native Codex status and mode-badge fixes.

### Changed

- Show a full session card at the pointer's original grab point during reordering, with a live destination gap. Commit on release; Escape or release outside the list cancels. Holding a card at an overflowing list's edge scrolls further.

### Fixed

- Treat failure red dots and completion green dots as unread badges: viewing clears them from the sidebar and floating rail while retaining the failed status. Record unread state for startup failures and abnormal process exits as well.

## [0.4.5-preview.1] - 2026-11-03

Local feature preview; not a published release.

### Changed

- Preserve Codex's shared server and `← for agents` by default, updating status from explicit OSC 7501 lifecycle in the current terminal. Explicit `codex --no-daemon` retains the legacy hook integration. Codex owns shared background tasks.

### Fixed

- Use equal visual widths for adjacent narrow mode badges, fixing mismatched sizes in Claude's double-triangle indicator; isolated badges can still use following padding for a complete glyph.

## [0.4.4] - 2026-11-03

### Improved

- Prefilter, cache and pixel-align Agent avatars at their physical display size for sharper thin strokes and zoomed rendering. Refresh the README with native high-resolution captures using synthetic, private-data-free content.
- Refresh session colors, selected borders and status brightness. Match collapsed-rail settings/new/expand controls to session-avatar size and balance expanded-card side insets.

### Fixed

- Stop collapsed-sidebar jitter near the overflow threshold. Fixed square tiles keep row height independent of scrollbar visibility; the scrollbar disappears when the pointer leaves.
- Correct old-monitor nudges, flicker, abnormal resizing and mouse-anchor jumps during mixed-DPI main/floating-window dragging. Native movement follows the OS-suggested rectangle; extra floating-size repairs wait until release.
- Separate floating-position observations from placement commands to prevent snapback, stretched containers and clipped avatars, while retaining zoom centering, up to ten visible sessions and drag browsing.
- Fit complete native emoji tiles, improving Claude manual-mode badges and narrow-emoji clipping while preserving composed clusters and text-presentation selectors.

### Upgrade and verification scope

- Fully extract the Windows x64 ZIP and keep both EXEs with conpty.dll/OpenConsole.exe. Existing session data remains compatible and user-level CLI settings are unchanged.
- Offline regressions, native smoke and two-monitor DPI/zoom/sidebar checks passed; the user confirmed physical cross-monitor dragging locally. This release did not repeat remote Windows 10 hardware acceptance or real-model turns; older evidence is not presented as current testing.
- Retain Codex independent mode and accurate status indicators under the current tradeoff. The shared `for agents` entry remains deferred.

## [0.4.4-preview.4] - 2026-11-03

Local repair preview, not a stable release; the date identifies this preview build.

### Fixed

- Correct the winit 0.30.13 path that repeatedly pushes Windows native drags back across mixed-DPI monitors. Apply the OS-suggested rectangle directly for both main and floating windows, addressing flicker, resizing and mouse-anchor jumps.
- Pause additional floating-window size repairs during native movement and reconcile after release. Extend the two-window probe to keep move state active across DPI boundaries and compare suggested/actual rectangles and recursive transitions.
- Keep equal space on both sides of expanded-sidebar cards while retaining the scrollbar beside the divider and the compact collapsed rail.

## [0.4.4-preview.3] - 2026-11-03

Local repair preview, not a stable release; the date identifies this preview build.

### Fixed

- Separate floating-window position observations from placement commands so a cross-monitor move is not pulled back to an old position. Reconcile the client area from that window's native DPI/size events to prevent stretched containers and clipped avatars, without idle polling.
- Give collapsed-sidebar settings, new-terminal and expand controls the same circular surface, glyph size and square tile as session avatars, with tighter vertical spacing.
- Add native two-monitor movement, position retention after movement, oversized-client recovery, zoom and close/reopen checks. Automated probes do not replace physical mouse-drag acceptance.

## [0.4.4-preview.2] - 2026-11-03

Local repair preview, not a stable release; the date identifies this preview build.

### Fixed

- Stop continuous collapsed-sidebar jitter caused by row geometry and scrollbar visibility influencing each other. Session tiles remain 44×44 logical pixels; scrollbar visibility no longer changes row height.
- Add multi-frame, zoom, hover, and idle-repaint regressions near the overflow threshold, plus native-window geometry and screenshot checks.

## [0.4.4-preview.1] - 2026-11-03

Local preview, not a stable release; the date identifies this preview build.

### Improved

- Refresh session colors and selected borders, and brighten working, attention, failure, and unread-completion indicators.
- Prefilter Agent avatars at their physical display size and cache the results, preserving thin strokes when reducing large source images.

### Fixed

- Keep collapsed-sidebar selection tiles square; hide the scrollbar when the pointer leaves the sidebar without changing its reserved gutter.
- Fit complete emoji into terminal cells, improving Claude mode badges and narrow emoji clipping.
- Reapply floating-window dimensions and position conversions using its own DPI when moving between monitors.

### Known limitations

- Keep Codex independent mode and accurate state indicators per the current decision. Shared `for agents` integration is deferred and is not a fix delivered in this preview.

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
