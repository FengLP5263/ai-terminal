# Project files

[简体中文](files.md) | English

Click the folder icon and project name in the terminal header to open the right file tree. Long names are shortened; hover shows the full root path. Files / Git share a resizable, collapsible panel.

Folders load lazily and sort before files. Click a file to view its contents read-only in the main area. The tree keeps the current file marked, and hovering a path shows its full location. Refresh updates the current panel; Files also supports Collapse all. Closing the document returns to the original terminal, keeping its process and background output connected.

## Projects and updates

The browser uses an available Git repository root, falling back to the current session's working directory. Ordinary folders need no Git installation. Switching projects clears the old view and stale query results. Moving within a repository in the same session retains its project root.

Directories load on first opening or expansion and remain cached for the current view. Refresh or window focus regain updates on demand, without periodic project scanning. Refresh retains existing directory contents while reading. Files load only on clicks or refresh; scripts, plugins and edits are not executed.

## Content and limits

- UTF-8 (with optional BOM) and BOM-marked UTF-16 LE/BE preserve valid CJK and emoji. Invalid encodings produce an explicit message without guessed repairs.
- Syntax colors, indent guides and native color emoji share the Git file renderer. Ordinary documents use a single line-number column; unknown types show plain text.
- Each file preview is limited to 2 MiB, 20,000 lines and 4,000 characters per line. Display truncation is explicit.
- Each directory reads at most 1,000 entries. The cache holds 64 expanded directories; the tree displays at most 20,000 expanded entries and 32 levels. Collapsing a folder releases its subtree cache.
- Binary, empty, deleted, inaccessible and oversized files have explicit states. Links and Windows junctions are marked and never followed or recursively expanded automatically.

Folders and actions use Microsoft Codicons. File-type icons use embedded Tabler SVG and are shared with Git, following app size and zoom. Only the `ui.project_files_open` panel preference is saved; file bodies, directory snapshots and recent files are not persisted. Legacy settings default the file panel off and retain Git preferences and session directories.

The right panel omits the repeated root name. The tree follows the tabs and action row directly, with collapse-all, refresh and close on the same row.

Embedded Tabler SVG icons (MIT) distinguish Python, HTML, Rust, JS/TS, React, Vue, CSS, scripts, media, documents and more. Common project filenames take precedence over extensions; unknown types use plain-file or binary fallbacks. Classification does not read file content. The tree, Git changes/commit lists and read-only titles share the mapping; folders and actions still use Codicons.

Files and folders ignored by Git have muted names and icons and an “Ignored by Git” tooltip; they remain expandable and readable. Local Git classifies each directory in one batch when it loads or refreshes, including nested rules, negations, repository excludes and global ignores. Tracked files retain their normal style. Missing Git, failed queries and ordinary directories keep the normal style instead of guessing. No periodic scan is added.
