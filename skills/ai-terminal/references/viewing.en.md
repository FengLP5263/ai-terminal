# Files, Git and media viewing

[简体中文](viewing.md) | English

Requires native_controls. These commands address the GUI's selected session/project. Inspect active using `workspace show`, then explicitly `session activate <UUID>` if needed. `current` still means the caller and does not implicitly select a project for file commands.

```powershell
ai-terminal-cli project show
ai-terminal-cli file list --path src --offset 0 --limit 100
ai-terminal-cli file expand --path src
ai-terminal-cli file collapse --path src
ai-terminal-cli file collapse-all
ai-terminal-cli file refresh
ai-terminal-cli file open --path README.md
ai-terminal-cli file show
ai-terminal-cli file read --offset 0 --limit 100
ai-terminal-cli file source
ai-terminal-cli file preview
ai-terminal-cli file next
ai-terminal-cli file previous
ai-terminal-cli file first
ai-terminal-cli file reload
ai-terminal-cli file close
```

Paths are relative to project show's root; absolute paths inside that root are accepted. Parent traversal, outside paths and links/junctions are rejected. Viewed scripts are never executed. Open/refresh/page commands enqueue work; query show/list after loading=true settles, without recreating objects. Check error: finished loading is not necessarily success.

Directory caches hold at most 1,000 entries per level and 64 directories. Respect partial and next_offset; do not claim a complete project enumeration when truncated. file read returns only the current bounded text page's lines. --offset/--limit address page-local indices, with next_offset for response pagination. Responses hold at most 200 items and about 256 KiB of serialized items. Use next/previous for larger files; show exposes byte offset, first line, total bytes and page availability. Do not repeatedly read entire files to poll status. source/preview toggles Markdown source/rendered mode only.

```powershell
ai-terminal-cli image fit
ai-terminal-cli image original
ai-terminal-cli image zoom --percent 200
ai-terminal-cli media status
ai-terminal-cli media volume --percent 0
ai-terminal-cli media play
ai-terminal-cli media pause
ai-terminal-cli media seek --seconds 1.5
```

Open the appropriate image/audio/video first. Image zoom is 5–800%. Media initially pauses; wait while duration_seconds is null. Volume is 0–100; seek cannot exceed duration. Playback uses verified local paths and the native Windows player, without remote URLs, playlists or a browser. Missing decoders fail explicitly. Closing a file or switching sessions releases playback resources. Managers with session preview restoration return paused at the saved position. `media status` may report `loading=true` while restoring.

```powershell
ai-terminal-cli git status
ai-terminal-cli git files --offset 0 --limit 100
ai-terminal-cli git history --offset 0 --limit 50
ai-terminal-cli git more
ai-terminal-cli git inspect --commit <FULL_SHA>
ai-terminal-cli git commit
ai-terminal-cli git open-diff --path src/main.rs
ai-terminal-cli git open-diff --path src/main.rs --commit <FULL_SHA>
ai-terminal-cli git diff --offset 0 --limit 100
ai-terminal-cli git close-diff
ai-terminal-cli git refresh
```

These share existing read-only background Git snapshots, without committing, checking out branches, writing the index or contacting remotes. SHAs require all 40/64 hex characters. inspect/open-diff return queued; inspect commit/diff loading/error next. Missing inspection objects do not fabricate data. History next_offset paginates the current snapshot; explicitly more when more_history=true. Existing partial/omitted_files limits remain. Commit messages beyond 32,768 characters set message_truncated=true; use Git itself for raw access. Distinguish not_repository from an empty repository.
