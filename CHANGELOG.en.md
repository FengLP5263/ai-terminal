# Changelog

[简体中文](CHANGELOG.md) | English

Record user-visible additions, improvements, fixes, and compatibility changes for each release; omit categories that do not apply. Review both languages together, use the same version entry in GitHub Releases, and ship both files in the ZIP. Historical summaries follow existing GitHub release notes and UTC publication dates; development versions without a separate release do not receive invented release dates.

## [0.11.0] - 2026-11-13

### Added

- Session groups: create, join, leave, rename and dissolve teams from session context menus. Monochrome badges identify members while preserving session order, directories and colors.
- The floating bar collapses each team into its first avatar with a simplified badge. Hover to see “Group name × N”, member avatars, titles and individual states; click a member to open its terminal. Pending and unread member states remain visible.
- The floating bar has an enabled-by-default outline that can be toggled and saved immediately, independently of opacity and floating scale.

### Changed

- Name badges fit their text and truncate long names on one line. Member cards keep session colors and clear spacing; order and a neutral divider distinguish the first member without a role marker.
- Session and group context menus retain tidy, single-line commands; hover over truncated group names to see them in full.
- Floating background transparency now spans 0%–100%. Avatars, state indicators and the enabled outline remain clear and clickable at full transparency.
- README includes bilingual group guidance, current native screenshots and updated asset provenance.

### Compatibility and boundaries

- Older workspaces load with no groups. Group metadata and the outline preference use compatible defaults and atomic saves, preserving existing sessions, colors, directories, fonts, scales and transparency.
- Grouping and dissolving only change navigation: terminals are not restarted, histories are not combined, and member states do not imply team completion. Removing the first member selects the next avatar in session order.
- Windows x64 support is unchanged. Screenshots and lifecycle probes use isolated data and synthetic events, without paid model requests or automatic trust actions.

## [0.11.0-preview.6] - 2026-11-13

### Changed

- Floating member cards have six-point gaps, with a twelve-point gap and neutral divider after the first session. Panel height includes the spacing, and longer lists remain scrollable.

## [0.11.0-preview.5] - 2026-11-13

### Improved

- Group name badges fit their text with small, equal horizontal padding; long names remain truncated on one line.
- Grouped floating avatars share the collapsed sidebar’s white badge with a black group-name initial. Hover headers show member counts as “Group name × N”.

### Fixed

- All hover-list members retain their session background colors. Normal, hovered and selected surfaces match the sidebar, including the background behind status indicators.

## [0.11.0-preview.4] - 2026-11-13

### Changed

- Group badges use simple white rectangles with black names, removing colored dots/outlines and all lead markers or labels. Order and a neutral divider distinguish the first member; floating counts and panel headings use neutral colors.
- Floating background transparency now spans 0%–100%. At full transparency, avatars, status indicators and the enabled outline remain visible and avatars stay clickable. Saved opacity supports 0.0–1.0.

## [0.11.0-preview.3] - 2026-11-13

### Added

- Grouped sessions show name badges with stable colors and a separate directory line. The first member has a lead marker and divider, also represented in the collapsed sidebar and hover member panel.
- A thin floating-bar outline is enabled by default, with an immediately applied, saved settings toggle. It is independent of background opacity and preserves window position, avatar hitboxes and native frame behavior.

## [0.11.0-preview.2] - 2026-11-13

### Fixed

- Keep session context-menu and group submenu items on single rows with a consistent width. Long names are truncated with full hover text; the ungroup action keeps session-preservation details in its tooltip, preventing narrow multiline commands.

## [0.11.0-preview.1] - 2026-11-13

### Added

- Session groups: create, join, leave, rename or dissolve teams from the session context menu. The sidebar shows group names while preserving session order.
- The floating bar collapses each group to its first session avatar with a member count. Hover to see member avatars, titles and individual states; click a member to restore the main window and its existing terminal. Pending/unread states remain visible without restarting terminals or merging histories.
- Group metadata is saved atomically with the workspace. Old workspaces remain ungrouped; closing the first member promotes the next session avatar.

## [0.10.0] - 2026-11-11

### Added

- Floating background transparency from 0% to 80%, applied and saved immediately in Settings. Avatars, status badges and hints remain clear; existing settings default to opaque.
- Muted names/icons and hover explanations for Git-ignored files and folders. Entries remain expandable and readable; nested rules, negations, repository/global excludes and tracked-file semantics follow Git.

### Changed

- Shared lightweight Tabler outline file-type icons across the tree, Git changes/commits and file titles. Python, HTML, Rust, JS/TS, scripts, media and documents have distinct icons, rasterized at final DPI with bounded caches. The `.gitignore` icon is neutral gray.
- Header order is project folder, Git summary, status, zoom. Project entries show a name and full-path tooltip. Remove duplicate root headings and extra dividers, retaining narrow-window truncation and click behavior.
- File and diff title icons stay beside left-aligned names; read-only indicators and close buttons align to the right edge.
- Settings, rename, session-close and quit dialogs share compact desktop typography, 36-point controls and coordinated spacing. Application zoom, font choices and immediate saving are retained.
- Refresh bilingual native README captures and ship the original Tabler MIT license, pinned source and hashes.

### Fixed

- Explicitly enable DWM desktop alpha composition for Windows/WGL floating bars and restore it after composition changes, fixing transparency sliders that changed without revealing the desktop.
- Remove the floating window’s system border, shadow and extra client insets while preserving smooth application corners, eliminating light-background outlines and gaps.

### Compatibility and limits

- Existing sessions, directories, fonts, zoom and history-recovery settings are retained. Missing `ui.floating_opacity` defaults to opaque; no migration is needed.
- Ignore classification runs in background batches only on directory loads or on-demand refreshes, with existing cache/cancellation/Git-query limits. Ordinary directories and missing Git remain browsable. No periodic scan or file-content persistence is added.
- Windows x64 support remains unchanged. No model proxy, paid model requests or automatic trust actions are introduced.

## [0.10.0-preview.5] - 2026-11-11

### Changed

- Use compact typography across settings, rename and confirmation dialogs: 18-point titles, 16-point section headings, 14-point body/buttons, 12-point supporting text and 36-point controls. Adjust spacing and confirmation width to match the main interface, retaining application zoom, font choices and immediate saving.

## [0.10.0-preview.4] - 2026-11-11

### Changed

- Move the project folder entry before the Git branch summary. Header order is project, Git, status, zoom, retaining truncation and click behavior in narrow windows.

## [0.10.0-preview.3] - 2026-11-11

### Added

- Project files and folders excluded by Git use muted names and icons with an ignore tooltip. They remain expandable and readable; tracked files and negated patterns retain their normal appearance.

### Fixed

- The transparent floating bar disables its own system border, shadow and extra client insets while retaining application corners. Native captures check enlarged edges and avatar clicks over light and gray backgrounds.

## [0.10.0-preview.2] - 2026-11-11

### Fixed

- Explicitly enable DWM transparency for the Windows/WGL floating bar and restore it after composition changes, fixing a black background despite slider changes. Verify actual desktop pixels over an owned white backdrop, rather than framebuffer alpha alone.
- Use a neutral gray ignored-file icon for `.gitignore` to reduce visual emphasis.

## [0.10.0-preview.1] - 2026-11-11

### Added

- Floating background transparency (0%–80%) in Settings, applied immediately and saved automatically. Avatars, status badges and hover text stay clear; old settings default to opaque.

### Fixed

- Align read-only labels and close buttons with the content area's right edge in files and Git diffs, keeping filenames beside their type icons on the left.

## [0.9.1-preview.4] - 2026-11-11

### Fixed

- File types use lightweight Tabler outline SVGs, including outline language symbols such as Python. Tree, Git lists and viewer titles share final-DPI rasterization with bounded caching and original MIT provenance/license.
- File-title icons and names are adjacent and left-aligned, correcting the gap caused by centered names.
- Header order is Git information, project folder, session status, zoom. Existing status sources, sessions and terminal processes are preserved.

## [0.9.1-preview.3] - 2026-11-11

### Fixed

- The file tree, Git changes/commit files and read-only titles share Seti type icons for Python, HTML, Rust, JS/TS, React, Vue, CSS, scripts, media, documents and more. Common project filenames take precedence; unknown types retain plain-file/binary fallbacks.
- Icons use the original upstream MIT font and palette, embedded and rasterized at the current size and zoom. No system-font installation is needed; terminal fonts, sessions and directories are unchanged. Packages include provenance, SHA-256 and the original license.

## [0.9.1-preview.2] - 2026-11-11

### Fixed

- Remove the repeated dividers and gap between the Git panel tabs and Changes/History, retaining the divider above the file list.

## [0.9.1-preview.1] - 2026-11-11

### Fixed

- The project-files entry now shows a folder icon and project name, using the same root as the file tree. Long names are shortened; hover shows the full path.
- The shared Files/Git panel removes the repeated centered root name. Collapse-all, refresh and close share the tab row, with tab text shortened to fit narrow windows.
- Existing sessions, working directories, fonts, zoom and terminal processes are preserved. No settings or assets are added.

## [0.9.0] - 2026-11-10

### Added

- A project file tree shares the collapsible, resizable right panel with Git. Open it from the folder button; folders load on demand and sort first, with refresh and collapse-all actions. Ordinary directories are supported too.
- Click a file to read its full content with line numbers, file-type syntax colors, indentation guides and color emoji. UTF-8 with or without BOM and UTF-16 with BOM are supported; the open file stays marked.
- The header entry, tab and directories use standard Codicons folder icons. File-type icons are shared with Git and follow the application's font, size and zoom.

### Compatibility and boundaries

- Closing a file returns to the original terminal, keeping its process and background output connected. Bounded background work handles directory reads and syntax preparation on demand; file bodies and directory snapshots are not persisted.
- Preview limits are 2 MiB, 20,000 lines and 4,000 characters per line; directories show up to 1,000 entries each, with at most 64 expanded directories cached. Size, permission, encoding, binary and link boundaries have explicit messages; links and Windows junctions are not followed automatically.
- The new `ui.project_files_open` preference defaults to off for old configurations. Upgrades preserve sessions, working directories, fonts, zoom and existing Git preferences.

## [0.9.0-preview.2] - 2026-11-10

### Fixed

- The Project files header entry and Files tab now share the standard Codicons folder glyph to represent project-directory browsing.

## [0.9.0-preview.1] - 2026-11-10

Local preview awaiting project-file browsing acceptance.

### Added

- A project file tree shares the collapsible right panel with Git, opened by a standard header icon. Folders load lazily and sort first; refresh, collapse all and ordinary folders are supported.
- Click files to view full contents read-only, with a single line-number column, syntax colors, indent guides and color emoji. UTF-8/BOM and BOM-marked UTF-16 are supported. The current file stays marked; closing returns to the original terminal.
- Icons use Microsoft Codicons, sharing file-type glyphs with Git and inheriting app font, size and zoom.

### Compatibility and limits

- Bounded background work handles directory reads and syntax preparation with on-demand refresh. Existing terminal processes/output remain; file bodies and directory snapshots are not persisted.
- Previews are limited to 2 MiB, 20,000 lines and 4,000 characters per line. Each directory has at most 1,000 entries, with 64 expanded directories cached. Limits, access errors, invalid encodings, binary files and links are explicit; links and Windows junctions are not followed automatically.
- The new `ui.project_files_open` preference defaults off for legacy settings, preserving Git preferences, sessions, directories and fonts.

## [0.8.0] - 2026-11-10

### Added

- Separate saved scales for the floating bar and main window. Adjust the bar in Settings, with Ctrl+wheel, Ctrl plus/minus, or Ctrl+0 over the bar. Legacy settings inherit the previous scale once, preserving sessions, directories and fonts.
- Broader read-only diff syntax highlighting for PowerShell, CMD/Bash, real TS/TSX, Dockerfile, TOML and other common types, distinguishing keywords, variables, strings and comments. Special filenames and extensionless script shebangs are recognized.
- `--settings` opens Settings at startup.

### Changed

- Settings apply immediately and save automatically, without Done. Appearance/Floating bar groups share consistent typography, control heights, spacing and alignment; narrow windows stack fields and allow scrolling. Rename, close-session and quit confirmations use the same form styling.
- PowerShell, Bash and CMD use standard Microsoft Codicons avatars consistently across creation menus, the sidebar and floating bar.
- The Git panel combines the repository name and actions. Branch and change totals appear only in the terminal header; untracked files use green U with a full hover explanation, and non-repository headers hide the Git placeholder.
- Packages include complete upstream syntax notices and provenance, checking locked versions, Cargo archives and file hashes.

### Fixed

- Floating icons render at their final physical size, removing blur from a second layer scaling pass when main/bar scales differ. Clicks, dragging, status badges and hints follow the independent scale.

### Upgrade notes

- Portable and current-user installer builds retain the existing data location, sessions, working directories and preferences. Fonts need no reconfiguration.

## [0.8.0-preview.3] - 2026-11-10

### Fixed

- Rasterize floating terminal glyphs and agent textures at their final pixel size, removing blur from whole-layer resampling. Clicks, dragging, badges and hints retain independent scaling.

### Changed

- Add `--settings` to open Settings directly. Remove Done from Settings; edits apply immediately and save automatically. Appearance/floating groups share aligned fields, control heights, typography, spacing and dark surfaces, stacking and scrolling on narrow windows.
- Rename, close-session and quit dialogs share the same form style, with explicit save and confirmation semantics.

## [0.8.0-preview.2] - 2026-11-10

### Added

- Use the two-face 0.5.2+bat-0.26.1 extended syntax set for PowerShell, CMD/Bash, real TS/TSX, Dockerfile, TOML and other common formats. Variables have a distinct color; source text, indentation and red/green diff backgrounds remain intact.
- Recognize common shell startup files, Dockerfile/environment variants and in-memory shebangs for extensionless scripts. Unknown formats retain plain text.

### Changed

- PowerShell, Bash and CMD use standard Microsoft Codicons terminal avatars consistently in creation menus, the sidebar and floating bar.
- Package the full upstream syntax license texts and provenance, checking the locked version, Cargo archive identity and file hashes.

## [0.8.0-preview.1] - 2026-11-10

Local preview for independent floating scaling and Git panel acceptance.

### Added

- The floating bar has its own saved scale, adjustable in Settings or with Ctrl+wheel / Ctrl plus/minus and Ctrl+0 over its window. Legacy settings inherit the former shared scale once, then adjust independently.
- Native bar size, avatars, clicks/dragging and hover hints follow the separate scale. Changing the bar does not resize or restart the main terminal.

### Improved

- Removed the standalone Git title and placed the repository name beside Refresh/Close. Branch and line totals appear only in the terminal header; the panel retains file/history controls.
- Untracked files use a green U, with an “Untracked (not added to Git)” tooltip that distinguishes merge conflicts.
- Non-Git directories no longer show a repository placeholder in the terminal header; explicit panel messages and manual refresh remain available.

## [0.7.0] - 2026-11-10

### Added

- A right-side terminal history scrollbar supports thumb dragging and track clicks for long output, retaining the 5,000-line limit and staying hidden on alternate screens.
- Open committed files or files in Changes in a complete-file read-only diff with old/new line numbers, red/green backgrounds, syntax highlighting, indent guides and native color emoji. The right panel marks the current file; closing returns to the original terminal.
- Working files show staged and unstaged net changes against HEAD. Untracked/unborn files show full additions, and removed files retain old content. Untracked labels and descriptions distinguish staging and merge conflicts.

### Improved

- JSON, Markdown and common code use file-type syntax colors independently of diff backgrounds. Source indentation/tab stops are preserved; raw @@ ranges and mode rows are hidden, with metadata on title hover.
- Queries, syntax preparation and cancellation run in the background, with only visible rows laid out. Repeated clicks reuse results; repository events/manual refresh update working files, without added polling or idle repaint.
- Refreshed both READMEs and native workspace/floating/Git-history screenshots, with a new read-only diff showcase. English examples use English text.

### Fixed

- File views render color emoji as complete graphemes, fixing monochrome outlines and syntax splits inside composite sequences. Explicit text variants retain text presentation.
- Strict UTF-8 validation and first-line BOM handling report invalid encoding rather than replacing bytes. Already-corrupted historical text is retained faithfully. Slow queries no longer trigger an immediate duplicate fallback refresh.

### Compatibility and verification

- Sessions, directories, ordering, fonts, the default data location and running terminals are preserved. There is no editing, index/configuration write, disk diff cache, external diff, textconv or content-filter execution.
- Git commands retain 6-second/2-MiB limits; file views show at most 20,000 lines/4,000 characters per line with explicit limit notices. Binary files have no text view; working-file links/junctions are not followed. Recreated staged-deletion paths show complete old/new sides.
- Compatible features since 0.6.0 raise the minor version to 0.7.0. The user accepted preview.6. Offline regressions, native scale checks, six-terminal smoke and isolated installer upgrades are recorded separately from real model rounds, with no model API requests.

## [0.7.0-preview.6] - 2026-11-10

Local preview for working-file diff acceptance.

### Added

- Files in Changes open a read-only diff of the current file against HEAD, combining staged and unstaged changes. Untracked files and unborn repositories show additions; removed files show deletions. The full-file line numbers, syntax highlighting, color emoji and red/green diff backgrounds are shared, without editing.
- Working and historical files have distinct source identities and persistent selection indicators; closing returns to the original terminal. The bounded background request/result slot is reused. Repository events or manual refresh results trigger rereads; repeated clicks reuse results, and directory switches cancel stale requests.

### Improved

- `??` now reads “Untracked”, with a tooltip explaining that the file has not been added to Git. Other status descriptions distinguish staged, unstaged and merge conflicts.
- Reads do not modify files, the index or Git configuration, or execute external diff, textconv or content filters. UTF-8, 2 MiB, 20,000-line and 4,000-character limits remain. Linked/junction files report an error rather than following outside the repository.

## [0.7.0-preview.5] - 2026-11-10

Local preview for continued acceptance of emoji in file bodies.

### Fixed

- Git syntax-highlighted files use the existing Windows native color emoji renderer, fixing monochrome outlines for fire and prohibited signs while preserving Unicode source, syntax colors and diff backgrounds.
- Skin tones, ZWJ, flags, keycaps and variation sequences retain complete grapheme slots even across syntax-span boundaries. Explicit text variants keep the source font and color instead of forcing color presentation.

## [0.7.0-preview.4] - 2026-11-10

Local preview for continued file-selection and encoding acceptance.

### Fixed

- The active committed file has a persistent gray background and thin blue marker, identified by full commit ID plus path. Hover does not move it; file switches, close and repository changes synchronize it, including loading/error states.
- Committed files validate UTF-8 strictly rather than silently replacing invalid bytes. UTF-8 BOMs are handled at the first code line, preserving Chinese, accented text and emoji. Invalid UTF-8 has an explicit localized error; already-corrupted but valid UTF-8 history is retained without guessed conversion.
- The file-title hover describes original UTF-8 text, and truncation messaging clarifies that either the row or per-line limit may apply.

## [0.7.0-preview.3] - 2026-11-10

Local preview for continued file-reading acceptance.

### Added

- Syntect 5.3.0 default grammars color JSON, Markdown and common source code by file type; unknown types fall back to plain text. Source spaces remain intact, tabs use four-column stops, and thin indent guides follow existing indentation without formatting files.
- Syntax is prepared and cached in the Git file worker with separate old/new parse states. Diff status only changes the existing red/green backgrounds and markers; syntax colors stay consistent. Scrolling still lays out visible rows only, and close/directory changes cancel stale work.

### Changed

- The body shows actual file rows instead of raw hunk ranges (@@) or file modes. Mode metadata is available on the file-title hover.
- Syntax preparation has a two-second soft budget, 100,000-token cap and 256-span-per-line cap. Errors or exhausted budgets fall back to plain text while retaining complete text within existing read/display limits.

- Start fallback refresh cooldown when a slow Git result is consumed, so a query taking over two seconds does not immediately trigger another read of fresh results.

## [0.7.0-preview.2] - 2026-11-09

Local preview for continued file-diff acceptance.

### Fixed

- Added/deleted code keeps its normal body text color instead of turning green/red. Only row backgrounds and +/− markers use the application green/red; line numbers keep the secondary text color.

- File views include unchanged content from the first through the last line, retaining inline additions/deletions and old/new line numbers. Mode-only commits also show the complete body. Existing read/display limits remain explicit.

## [0.7.0-preview.1] - 2026-11-09

Local preview awaiting hands-on acceptance.

### Added

- A narrow right-side terminal history scrollbar supports thumb dragging and track clicks. The 5,000-line limit and viewport snapshots remain; a fixed gutter avoids CLI reflow when history appears. Scrollbar gestures never reach the terminal application, and alternate screens do not show local history.
- Click a committed file to open a read-only diff in the main area while retaining the commit panel. Old/new line numbers and the existing application green/red distinguish additions/deletions. Closing returns to the same terminal; background output continues draining.
- File diffs load on demand in the background and cancel stale requests without mixing in uncommitted content. Root and first-parent merge comparisons match the file list. Binary files have explicit messaging; display is bounded to 20,000 lines and 4,000 characters per line. Queries exceeding six seconds or 2 MiB fail explicitly.

### Compatibility

- No new workspace fields or changes to data directories, session identity, font choices or Agent lifecycle. Compatible features target 0.7.0 under SemVer; the stable 0.6.0 tag and assets remain unchanged.

### Validation scope

- Governance, formatting, Clippy, 235 Rust tests and 20 Python tests pass. Real isolated ConPTY regressions verify scroll positions, mouse-input isolation and terminal preservation; Git tests cover file diffs and bounded reads. Native screenshots and six-terminal smoke evidence are recorded separately and do not claim real Agent model-turn acceptance.

## [0.6.0] - 2026-11-09

### Added

- Show commit history as a compact dot-and-line timeline. Hover for author, UTC time, message and file/line totals; display a commit icon and seven-character ID, while row and card controls copy the full commit ID.
- Click a commit to expand changed files, then click again to collapse. Handle root, empty and merge commits, explicitly comparing merges against their first parent. Read details on demand in the background and cancel old requests when directories change.

### Changed

- Share compact file rows between Changes and commits and align expanded file icons with commit text. Distinguish Markdown, code, JSON, configuration, scripts, media, PDF, archives and binary files with Microsoft Codicons glyphs.
- Keep reading status inside the existing refresh button without adding a row or shifting the list.
- Let healthy directory monitoring own automatic updates, reusing snapshots on prompts, focus and panel opens. Missing-watch fallback is action-driven and limited to once every two seconds; explicit refresh remains available. Ignore Git reflog, lock and object writes and coalesce edits using a 250 ms quiet window, capped at one second during sustained changes.
- Simplify quit confirmation to “Terminal sessions will close. You can resume conversations next time.” in both languages, and update native README images.

### Compatibility

- Preserve workspaces, session directories/order, fonts and running terminals. Commit details stay in bounded memory, with no new persisted fields, repository writes or Git-configuration changes.
- Git for Windows is still required, with six-second/2 MiB per-command limits. Display at most 500 commit files while totals cover all queried results. Binary files have no line counts; renames appear as deletion and addition.
- Compatible features added since 0.5.0 advance the minor version to 0.6.0 under semantic versioning. Isolated preview data directories are local acceptance fixtures; the stable program keeps its original default data directory.

### Verification scope

- Passed 229 Rust and 20 Python offline regressions, governance, formatting and Clippy. Coverage includes abbreviated display/full-ID copying, alignment at three zoom levels, stable loading layout, root/empty/merge commits, query limits, monitoring filters and refresh gating.
- Twelve native Git images cover Chinese/English, 100%/103%/114%/200%, JetBrains Mono/Hack, changes/expanded history/collapsed views and hover cards. Six-terminal smoke makes no model requests. The user accepted preview.3; see the release notes for stable installation-upgrade and artifact verification records.

## [0.6.0-preview.3] - 2026-11-09

Local preview for continued interface acceptance.

### Changed

- Show a Microsoft Codicons commit icon and seven-character abbreviated ID in commit hover cards; copy controls still copy the full commit ID.
- Simplify quit confirmation to “Terminal sessions will close. You can resume conversations next time.” with matching Chinese text.

## [0.6.0-preview.2] - 2026-11-09

Local preview for continued Git-panel acceptance.

### Changed

- Share compact file rows between Changes and expanded commits. Align file-icon left edges with commit text and distinguish Markdown, code, JSON, configuration, scripts, media, archives, PDF and binary files with Microsoft Codicons glyphs.
- Put reading status inside the existing refresh button instead of inserting a row or shifting the list.
- Let healthy directory monitoring own automatic refreshes. Prompts, focus and opening the panel no longer repeat full queries. If monitoring fails, action-driven fallback reads are limited to once every two seconds; explicit refresh remains available. Ignore Git reflog, lock and object writes and coalesce ongoing edits with a quiet window.

## [0.6.0-preview.1] - 2026-11-09

Local preview pending user acceptance; not a public release. Compatible features added since 0.5.0 target minor version 0.6.0.

### Added

- Show commit history as a compact dot-and-line timeline. Hover for author, UTC time, message and line totals; copy the full commit ID from the card or row.
- Click a commit to expand changed files, status and totals, then click again to collapse. Root, empty, merge and binary changes have explicit displays; merge changes compare against the first parent.
- Read details on demand in the background and cancel old requests on directory changes. Preserve terminals, working directories and font settings; reuse Microsoft Codicons for copy/file symbols.

### Verification scope

- Passed 226 Rust and 20 Python offline regressions, governance, formatting and Clippy, plus a final rerun of 16 Git-specific tests.
- The isolated native repository probe produces twelve Chinese/English captures covering zoom levels, changes/history/collapsed views and hover cards, with normal exit and owned PTY cleanup verified. Six-terminal smoke passed without model requests.

## [0.5.0] - 2026-11-09

### Added

- Add a Git workspace view for the active terminal directory: branch and line totals in the header, plus changed files and history reachable from the current HEAD in a collapsible, resizable right panel. Load 50 commits at a time, up to 300.
- Run read-only local Git queries in the background and coalesce Windows directory notifications, filtering ignored build files. Support prompt, focus and manual refresh, reject stale directory results on session changes and preserve running terminals during panel operations.

### Changed

- Share one header row for the name, path, branch and line totals. Follow the font selected in ai-terminal settings, inherit application text size and shorten long metadata to fit.
- Use the original Microsoft Codicons branch glyph, scaling with text. Preserve original font/mapping bytes, provenance hashes and the CC BY 4.0 license.
- Present red, green, yellow, blue, pink, purple and No color in one menu row. Draw a clear No color circle/slash; hovered and selected sessions share emphasized fills and text brightness, with no separate card outline in either sidebar layout.
- Establish ongoing semantic versioning. Compatible features added since 0.4.11 advance the minor version to 0.5.0; future releases classify their complete change range.

### Compatibility

- Keep the existing workspace, session names, directories, order and font preferences. Preserve readable/savable legacy teal/orange IDs while mapping them to blue/red. The Git panel defaults to collapsed for older records.
- Git views require an installed Git for Windows and display local data without modifying repositories, Git configuration or credentials. Binary, large and content-filter files and display limits have explicit counting boundaries. Content filters and missing-object downloads do not run automatically. See [Git panel](docs/git.en.md).
- Setup and portable ZIP continue sharing the same data directory. No system fonts are installed and user terminals are not stopped automatically. Existing public tags and release assets remain unchanged.

### Verification scope

- Automated regressions cover real Git net changes, untracked files, unborn/detached HEAD, branch history, pagination, notifications, filter isolation and output caps, plus session colors, single-row headers, settings-font inheritance and preserved ConPTY identity.
- Native Git captures check Chinese/English, 100%/103%/114%/200% zoom, JetBrains Mono/Hack, changes/history/collapsed panels and composite emoji. Six-terminal smoke uses synthetic lifecycle events without real model calls. See release notes for installer upgrades and final artifact digests.

## [0.4.11] - 2026-11-06

### Fixed

- Fix recursive launch when a public dispatcher such as Qoder resolves another alias through PATH and reenters the session shim. Exclude only this session's entry directory from CLI-child PATH, preserving the outer interactive shell, public launchers, original arguments, exit codes and a single lifecycle pair.
- Preserve user PATH, CLI installations and configuration. Hooks keep their existing absolute paths; running sessions are not terminated and trust is not accepted automatically.

### Verification scope

- A bounded regression reproduces recursion in old code and reaches the public alias after the fix. PATH coverage includes case, slashes, CJK/spaces and similar directories; both regressions run in normal test/ci. Governance, formatting, Clippy, 207 Rust and 20 Python tests pass.
- Both real qodercn/qoderclicn --version entries return 1.1.83, and native six-PTY smoke passes. These checks are separate from real model-turn acceptance; no model requests.

## [0.4.10] - 2026-11-06

### Added

- Add a per-user Windows Setup alongside the portable ZIP, with installation directory selection, Start menu entries, an optional desktop shortcut and uninstallation.
- Select five terminal fonts in Settings: JetBrains Mono (default), Cascadia Mono, Hack, and installed Consolas or Courier New. Options show only family names. Apply and save immediately; unavailable system fonts are identified and fall back to the default. Existing saved font choices are preserved.

### Changed

- Font changes preserve terminal and Agent processes. Chinese and emoji keep their system fonts; the four embedded styles and original native rendering parameters are retained.
- Installed builds continue using `%LOCALAPPDATA%/ai-terminal` for sessions/settings. Upgrades reuse the registered installation directory, with an explicit directory override taking priority. Uninstall deletes managed program files and preserves data and unrelated user files. Upgrade/uninstall checks busy files without terminating sessions automatically.
- Pin the NSIS toolchain source, version and digest, retain its original license, and use isolated registry keys, shortcuts and data directories for acceptance.
- Native smoke waits for all six fixed fixtures before capturing instead of counting an early blank screenshot as passing evidence.

### Compatibility and validation

- Pass governance, formatting, Clippy, 205 Rust and 20 Python regressions. The stable build runs isolated smoke with six real PowerShell PTYs and synthetic task events, without model requests.
- Inspect native captures of all five fonts at 100%/103%/110%/150%/200%. The user accepted font selection and the JetBrains Mono default. Existing sessions, working directories and explicitly saved font choices are preserved.
- Isolated installer acceptance covers fresh install, upgrade from 0.4.9, rejection of upgrade/uninstall while running, installed six-PTY smoke, uninstall preserving data/unrelated files, and registry cleanup. Page-by-page wizard interaction and cross-device upgrades still need practical acceptance.
- Automatic update checking and code signing are not integrated; existing Agent login, trust and permission boundaries remain.

## [0.4.9] - 2026-11-05

### Fixed

- Qoder automatic permission checks no longer falsely request attention. Correlate manual waits with tool IDs, reject delayed notices for settled tools, and preserve genuine questions and MCP waits.
- Reply correctly to private cursor-position queries so responses do not leak into TUI input. Use the correct inverted glyphs and paint order for wide-character block cursors.
- Full floating hints share native color-emoji rendering, with long-name wrapping, work-area positioning, no activation and mouse passthrough.
- Ignore temporary sizing, invisible, already discarded and unknown-screen layouts when resizing terminals; restrict main-terminal UI and background state processing to the root viewport.
- Fix intermittent stretched, overlapping or flashing text on Windows OpenGL by uploading the frame's vertices and indices before drawing, retaining buffer-capacity reuse.

### Changed

- Default to four original embedded Cascadia Mono faces: regular, bold, italic and bold-italic. CJK continues to use system Microsoft YaHei UI with natural native default rendering parameters.
- Bundle the original Cascadia SIL OFL 1.1 and font provenance, checking font identity and license completeness on build and archive verification. Include local protocol patches in source identity and third-party licensing records.

### Compatibility and validation

- Keep the fixed `%LOCALAPPDATA%/ai-terminal` session-data directory and existing names, directories, order and UI preferences. CLI login, trust and permission decisions remain with the user.
- Pass 201 Rust and 16 Python regressions, governance, formatting and Clippy; verify native six-PTY smoke, five typography scales and three full-hint scales.
- Capture 656 fixed-table frames at 100%/103%/110%/200% with identical fixed-region pixels and visible grids, including 512 consecutive captures at 103%. See the rendering-check document for reproduction images and scope.
- The user tested preview.5, reported no issues and authorized stable publication. Synthetic events, controlled graphics fixtures and real Agent use are recorded separately. Developer checks made no paid model requests and do not establish coverage of every CLI or long-duration interaction.

## [0.4.9-preview.5] - 2026-11-05

### Fixed

- Fix occasional stretched, overlapping or briefly flashing text on the Windows OpenGL path: upload the frame's vertices and indices before drawing in their original order, avoiding repeated overwrites of regions referenced by earlier meshes in the same paint call.
- Retain GPU buffer capacity reuse. Reusable CPU staging contains only current visible-frame geometry; textures, clipping, callback order and the selected fonts retain their behavior.

### Developer checks

- A fixed local file table and changing glyphs in another terminal region captured one corrupt frame on the old path; the same 48 comparisons on the new path had no pixel differences. Record this controlled reproduction separately from long-term acceptance in a real user session.
- Add regressions for cross-texture/callback index offsets, size overflow, repeated-frame clearing and raster independence from prior glyphs.

## [0.4.9-preview.4] - 2026-11-05

### Fixed

- Terminal sizing ignores measurement, invisible, already discarded and unknown-screen layouts; real window changes still resize ConPTY.
- Main-terminal drawing and background state processing are restricted to the root viewport, keeping floating/hint geometry and minimized state separate.
- Retain preview.3's selected Cascadia Mono plus Microsoft YaHei UI and native default font rendering parameters.

### Developer checks

- Add real-ConPTY layout-isolation regressions. Record single Qoder startup captures separately from full reproduction of the user's duplicate-page report; defensive regressions do not establish acceptance of every scenario.

## [0.4.9-preview.3] - 2026-11-05

### Changed

- Adopt the visually selected Cascadia Mono + Microsoft YaHei UI combination, retaining size and 0.4.8 native default rendering parameters; preview.1 GDI classic fitting stays reverted.
- Embed four original regular/bold/italic/bold-italic faces from Microsoft's official v2407.24 archive. No Latin font installation is needed; system CJK/emoji fonts are not copied.

### Packaging and checks

- Pin font archive, revision and per-file SHA-256; bundle the original SIL OFL 1.1 and provenance. Reject tampered fonts or declared fonts without provenance/licenses; old archives remain verifiable.
- Production UI and probes share the new defaults. Preference for the independent system-font sample and actual acceptance of this embedded version are recorded separately. Workspace directories and session format are unchanged.

## [0.4.9-preview.2] - 2026-11-05

### Fixed

- Revert preview.1 GDI classic stem fitting after visual user acceptance found it stiffer. Restore 0.4.8 native default rendering parameters; retain the accepted status, protocol, cursor and emoji fixes.

### Development checks

- typography_probe adds fixed-size comparisons with original antialiasing: Hack/Consolas/Cascadia Mono and Microsoft YaHei UI/Microsoft YaHei/DengXian, with interactive switching. It uses local fonts and fixed samples, without changing daily font/session settings or installing/copying system fonts.
- Default terminal fonts remain Hack/Microsoft YaHei UI. Candidate shape and clarity require visual user choice; passing regressions does not establish better appearance.

## [0.4.9-preview.1] - 2026-11-05

### Fixed

- Qoder automatic approval no longer reports false attention. Correlate manual permission waits by tool ID, reject delayed notices for settled tools, and preserve genuine questions/MCP waits.
- Add private cursor-position queries with correctly prefixed responses, allowing TUIs to recognize them; ordinary responses are unchanged.
- Floating hints share production color emoji rendering while retaining long-title wrapping, work-area placement, mouse passthrough and no activation.
- Correct wide-character block-cursor inversion and drawing order so the final Chinese input character is not obscured by a black block or adjacent backgrounds.

### Improved

- Keep font files and adjust native physical-pixel stem fitting for ordinary terminal text to improve small-size/fractional-scale clarity; emoji retains its color parameters.

### Compatibility and acceptance

- Ordinary/private query regressions, synthetic hooks, native screenshots and real model tasks are recorded separately. This preview does not claim complete long-running Agent acceptance. No automatic trust approval or paid model request.
- Daily startup retains the fixed `%LOCALAPPDATA%/ai-terminal` data directory and existing session directories; isolated probes do not migrate user data.

## [0.4.8] - 2026-11-05

### Added

- Kimi Code 2.5.0 native OSC 7501 lifecycle uses explicit root-session events for working, interaction/login waits, completion and failure; child Agents, message bodies and stale invocations are ignored.
- TRAE CLI 0.213.1 receives lifecycle hooks through invocation-scoped `-c` options, preserving CLI trust review and later user overrides.
- Community zcode-app-cli 3.17.2-44 / runtime 0.17.1 receives start, tool and completion events through a credential-free basic-settings snapshot, leaving user files and authentication unchanged.

### Fixed

- Floating hints use an owned native window for full names, Agent and status text beyond capsule bounds, with monitor-aware placement, scaling, long-title wrapping, mouse passthrough and no focus activation.
- ZCode shares the explicit hook-event reducer: start/tool execution shows Working, questions show Needs attention, Stop shows Completed and explicit failure shows Failed. Remove preview.2’s unconfirmed downgrade.

### Changed

- README support tables and icons cover six Agents. Refresh four native workspace/floating captures in Chinese and English; English examples, names and controls use English.

### Compatibility and upgrade notes

- Restart existing CLIs in the new version. Users review login, directory and hook trust in the CLI; no automatic authorization or model request is performed.
- Existing ZCode hooks, custom lists, MCP/plugins, unknown settings or explicit `ZCODE_CLI_SETTINGS_FILE` preserve the original configuration and skip integration. Only credential-free basic options are copied; provider/credential files are never opened. Snapshot settings changes last only for that invocation.
- Status reflects the last explicit notification. If ZCode emits no failure/interruption event, the previous state remains; terminal text, silence and timeouts are not used to infer results.
- The current enterprise TRAE CLI requires an Enterprise flagship account and PAT. Personal subscriptions and third-party model API keys do not replace login; remote services cannot be assumed to execute local hooks.

### Verification

- 190 Rust and 14 Python regressions, governance, formatting and Clippy pass. The optimized release passes six-PTY smoke, owned-process cleanup, bilingual main-image captures and native floating-hint checks.
- Native hints cover 100%/142%/200% zoom, long titles, no activation, mouse passthrough and hiding on leave. Real startup checks cover Kimi directory-trust notifications, ZCode basic-settings snapshot/TUI and the TRAE login page. After actual ZCode use, the user requested direct hook-state display. Development made no model requests in this run and does not claim real failure/interruption-turn coverage for every CLI.

## [0.4.8-preview.2] - 2026-11-05

### Fixed

- Floating-bar hover hints use an owned native window to display the full name, Agent and status beyond capsule bounds, with monitor-aware placement, scaling and long-title wrapping.
- Community ZCode shows Status unconfirmed with a hollow gray marker after submission when reliable model-failure/interruption end events are unavailable, avoiding an indefinite working animation. Explicit completion, failure and process exit still update the result.

### Compatibility and limits

- Terminal error text, silence and timeouts are not used to infer results. Inspect errors in this ZCode version inside the CLI. Existing processes must be restarted in the new preview.
- Clarify the TRAE enterprise CLI’s flagship account/PAT prerequisite and bilingual status guidance. This local preview is not a stable publication.

### Verification and upgrade notes

- 191 Rust and 14 Python regressions, governance, formatting and Clippy pass. The final optimized build passes six-PTY smoke, owned-process cleanup and native screenshot checks.
- Native hints cover 100%/142%/200% zoom, full Chinese/English and long titles, mouse passthrough, no activation and hiding on leave. Missing ZCode end events use synthetic metadata; real error-turn acceptance remains with the user. No model request was made.

## [0.4.8-preview.1] - 2026-11-04

### Added

- Native OSC 7501 lifecycle for Kimi Code 2.5.0: explicit root events drive work, interaction/login waits, completion and failure. Child records, message bodies and stale invocations are ignored.
- Invocation-scoped `-c` lifecycle hooks for TRAE CLI 0.213.1, preserving CLI trust review and later explicit overrides.
- Start, tool and completion hooks for community zcode-app-cli 3.17.2-44 / runtime 0.17.1 through a credential-free basic settings snapshot, preserving original user files and authentication.

### Improved

- Refresh both native README workspace and floating-bar images with six Agents. The English README uses separate English UI, synthetic content and names.
- Status connection help explains each transport, configuration boundary and user trust step.

### Compatibility and limits

- Existing ZCode hooks, custom lists, MCP/plugin definitions, unknown settings or explicit `ZCODE_CLI_SETTINGS_FILE` skip automatic integration and keep the original configuration. Changes within a basic settings snapshot last only for this invocation; providers, credentials and custom commands are not copied. Upstream lacks reliable approval-wait, failure and interruption events, so these states may not update immediately.
- Restart existing CLIs to apply integration. This preview does not log in, approve trust or request models automatically. Real account/model-turn acceptance remains with the user; this is not a stable release.

### Validation and upgrade notes

- 187 Rust and 14 Python tests, governance, formatting, Clippy, optimized build and six-PTY native smoke passed. All four final native Chinese/English workspace/floating-bar captures were inspected.
- Real Kimi reported native permission status on the workspace-trust page. The final ZCode build verified its basic settings snapshot, TUI startup and return to PowerShell. TRAE reached its login page and was cancelled. No model prompts or trust approvals were submitted; these checks are not real model-turn acceptance.

## [0.4.7] - 2026-11-04

### Added

- Recognize Kimi Code (`kimi`), ZCode (`zcode`) and TRAE CLI (`traecli`) public entries in managed shells, showing their avatars and restoring the current shell on exit. Discovery keeps the exe/cmd/bat order and stale standard-npm-launcher checks.
- On first launch with the default data directory, create `ai-terminal.lnk` on the Desktop targeting the current program. The app attempts once and startup continues if creation fails; `--data-dir` with a non-default directory, isolated smoke runs, or a failed workspace load never create it, and a user-deleted shortcut is not recreated.

### Changed

- Use verified official Kimi Code, ZCode and TRAE product icons, with pinned provenance and per-file license declarations. Packages include icon provenance, available original licenses and bilingual third-party notices.

### Fixed

- Release retired atlas pages even when later frames only hit the glyph cache, while preserving texture lifetimes for shapes recorded in the same frame.
- The native glyph atlas no longer keys cached tiles by foreground/background color. Animated TUI gradients, blinking and per-frame recoloring reuse the same glyphs instead of rapidly filling the atlas and rebuilding it wholesale; a full page now retires only the oldest page and keeps its texture handle until a later frame, reducing terminal garbling and flicker.

### Compatibility and boundaries

- Kimi Code, ZCode and TRAE CLI provide identity and launch/exit recognition only in this release; session-scoped task events are not integrated. The manager does not copy or edit user/project hook configuration or infer task status from output/process existence. ZCode was tested with the community zcode-app-cli 3.17.2-44 (runtime 0.17.1), not as acceptance of Z.ai's official standalone CLI.

### Verification and upgrade notes

- Passed 181 Rust and 14 Python regressions, governance, formatting, Clippy, the optimized build and six-session native smoke. Native captures verified actual startup, avatars and return to PowerShell for Kimi Code 2.5.0, community zcode-app-cli 3.17.2-44 / runtime 0.17.1, and TRAE CLI 0.213.1 public. Kimi was cancelled at directory trust and TRAE at login; no trust approval or model prompt was submitted.
- The bilingual README now lists six Agents, matching production-rendered icons, launch commands and task-status boundaries. Extract the whole new ZIP; it includes icon provenance, available original licenses and bilingual third-party notices.
- Actual first-run Desktop creation, real model tasks, Windows 10 and sustained workload remain outside this release's new checks. Startup captures and offline fixtures do not establish those acceptance results.

## [0.4.6] - 2026-11-04

### Added

- Support Qoder CN CLI's `qodercn`/`qoderclicn` entries, avatar, lifecycle hooks, conversation recovery, drag selection and image paste. Resolve native/npm installs and preserve explicit user `--settings`, global configuration and directory trust.

### Fixed

- Codex launched through an absolute path, alias or `--remote … resume …` can establish its avatar and update state from explicit native notifications. Restore shell identity at its prompt and support repeated invocations. Hidden sessions still consume notifications in background logic, without inferring state from content or changing resume arguments or saved conversation IDs.

### Upgrade notes and verification boundaries

- Use the updated manager binary for already open windows; the CLI still owns login, trust and models. Record Qoder's offline synthetic lifecycle, real startup and user preview acceptance separately. This release does not automatically accept trust or send paid model requests.
- Passed 168 Rust and 13 Python regressions, governance, formatting, Clippy, Release smoke, real Codex 0.166.0 absolute-path remote startup and seven synthetic Qoder state captures. A fake CLI in a real ConPTY covers resume arguments and repeated invocation. Startup captures are not real model-task acceptance; long-duration load and Windows 10 retesting were not run.
- The user separately confirmed that the Qoder CN preview and recovery of an existing Codex conversation work correctly and authorized release. User acceptance is recorded separately from the developer's synthetic/startup checks above.

## [0.4.6-preview.1] - 2026-11-04

Local feature preview; not a published release.

### Added

- Support Qoder CN CLI's `qodercn`/`qoderclicn` entries, avatar and explicit lifecycle status. Exit restores the current shell; keep native terminal interaction, drag selection and image paste.
- Inject metadata-only hooks with session-scoped `--settings`, preserving explicit user settings, global/project configuration and directory trust. Resolve native executables and npm launchers and preserve resume arguments.

## [0.4.5] - 2026-11-04

### Changed

- Preserve Codex's shared server and `← for agents` by default. Explicit native OSC 7501 lifecycle from the current terminal updates status, including hidden sessions. Explicit `codex --no-daemon` retains the legacy hook integration.
- Show a full session card at the pointer's original grab point while the list makes space. Release commits the order; Escape or release outside cancels. Holding a card at an overflowing list's edge scrolls further. Hide hover tooltips during dragging so they do not cover the card.

### Fixed

- Treat failure red dots and completion green dots as unread badges: viewing clears them from the sidebar and floating rail while retaining the failed/completed state. Record unread state for startup failures and abnormal process exits too.
- Give adjacent narrow Claude mode badges equal visual widths, fixing mismatched double-triangle sizes. Preserve isolated badges, combined emoji and VS15 text presentation.
- Normalize bilingual document digests as UTF-8/LF text, preventing false governance failures after Windows checkout or editor line-ending conversions.

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
