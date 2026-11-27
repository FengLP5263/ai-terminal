# Project files

[简体中文](files.md) | English

Click the folder icon and project name in the terminal header to open the right file tree. Long names are shortened; hover shows the full root path. Files / Git share a resizable, collapsible panel.

Folders load lazily and sort before files. Click a file to view its contents read-only in the main area. The tree keeps the current file marked, and hovering a path shows its full location. Refresh updates the current panel; Files also supports Collapse all. Closing the document returns to the original terminal, keeping its process and background output connected.

Each session keeps its open file, page, scroll position, Markdown view mode, and image zoom for the lifetime of the app. Switching away releases content and textures. Returning reads the file again. Closing the preview, closing the session, or exiting clears this state.

Search from the top of the Files tab by filename or relative path, regardless of case. Results include files in unopened folders and files ignored by Git. Click a result to open its read-only preview, right-click to copy its full path, or clear the search to return to the tree. Search displays up to 200 results. When results are partial, specify more of the directory path.

## Projects and updates

The browser uses an available Git repository root, falling back to the current session's working directory. Ordinary folders need no Git installation. Switching projects clears the old view and stale query results. Moving within a repository in the same session retains its project root.

Directories load on first opening or expansion and remain cached for the current view. Refresh or window focus regain updates on demand, without periodic project scanning. Refresh retains existing directory contents while reading. Files load only on clicks or refresh; scripts, plugins and edits are not executed.

## Content and limits

- UTF-8 (with optional BOM) and BOM-marked UTF-16 LE/BE preserve valid CJK and emoji. Invalid encodings produce an explicit message without guessed repairs.
- Syntax colors, indent guides and native color emoji share the Git file renderer. Ordinary documents use a single line-number column; unknown types show plain text.
- Large text loads in pages of at most 2 MiB / 20,000 lines. Long lines continue in 4,000-character segments without omitting the remainder. Previous, Next and First navigate pages; changes between pages require reloading.
- PNG, JPEG, GIF, WebP, BMP, ICO, TIFF and SVG preview centered horizontally and vertically in the main area with fit, original-size and zoom controls, plus a checkerboard for transparency. Animations show the first frame; images above 4,096 pixels are downsampled. Encoded images are capped at 32 MiB; raster originals at 32 million pixels and 16,384 pixels per edge. SVG is capped at 4 MiB and does not load external image resources.
- Markdown renders headings, lists, tables, quotes and syntax-colored code blocks by default, with a Source toggle. The preview uses a centered reading width, separators under level-one/two headings, wrapped table columns and a separate horizontal scrollbar for wide tables. Common embedded HTML supports local `<img>` elements with `width`/`height`, `<div align="center">`, links and line breaks; scripts, styles and embedded pages do not run. Pages are capped at 256 KiB / 2,000 lines; complex structures fall back to source, and pagination can split structures across pages. HTML images with the same source and dimensions share decoding and texture resources instead of consuming the budget again. Up to 16 project-local images are downsampled to 2,048 pixels per edge with a total of 16 million pixels. Remote images never load automatically. Local file links use the same read-only viewer; web/mail links reach the system only on a click. Word, Excel, PowerPoint and similar documents are not previewed.
- Each directory reads at most 1,000 entries. The cache holds 64 expanded directories; the tree displays at most 20,000 expanded entries and 32 levels. Collapsing a folder releases its subtree cache.
- Binary, empty, deleted, inaccessible and oversized files have explicit states. Links and Windows junctions are marked and never followed or recursively expanded automatically.

Folders and actions use Microsoft Codicons. File-type icons use embedded Tabler SVG and are shared with Git, following app size and zoom. Closed/Files/Git panel state is saved per session; file bodies, directory snapshots and recent files are not persisted. Legacy global preferences migrate only to their original active session; other sessions start closed.

The search field sits below the tabs and above the files. Collapse all, refresh and close stay in the tab row.

Embedded Tabler SVG icons (MIT) distinguish Python, HTML, Rust, JS/TS, React, Vue, CSS, scripts, media, documents and more. Common project filenames take precedence over extensions; unknown types use plain-file or binary fallbacks. Classification does not read file content. The tree, Git changes/commit lists and read-only titles share the mapping; folders and actions still use Codicons.

Files and folders ignored by Git have muted names and icons and an “Ignored by Git” tooltip; they remain expandable and readable. Local Git classifies each directory in one batch when it loads or refreshes, including nested rules, negations, repository excludes and global ignores. Tracked files retain their normal style. Missing Git, failed queries and ordinary directories keep the normal style instead of guessing. No periodic scan is added.

Audio/video previews use the native Windows Media Foundation engine, with a centered, width-limited audio player and a centered video picture with a slim seek bar and compact playback and volume controls along its bottom edge. During playback the controls fade out over 280 ms after about 2.5 seconds of pointer inactivity and return when the pointer moves over the picture. They remain visible while paused or in use. Standard controls provide play/pause, elapsed/total time, a draggable seek bar, mute and volume. Unmute restores the prior volume, and Play restarts ended media. Narrow windows move the seek bar onto another row to keep controls usable. MP4/M4V/MOV/WebM/MKV/AVI/WMV/MPEG and MP3/WAV/FLAC/OGG/Opus/M4A/AAC/WMA/AIFF are recognized; decoding depends on installed system codecs. Unsupported or damaged files show a playback error. No browser runtime or bundled codec pack is added. Files are streamed by the engine rather than loaded into a text buffer, with no 2 MiB text-preview limit. Previews start paused. Switching sessions releases the player and frame textures. Returning restores position and volume, paused until you press Play. Switching files or projects, closing the preview or exiting clears the previous position. Background terminals keep running. Playlist and remote-URL files are not treated as playable media. Video preview frames are capped at 1,920 pixels per edge and 30 fps; source video over 16,384 pixels per edge or 64 million pixels is rejected.
