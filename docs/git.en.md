# Git panel

[简体中文](git.md) | English

Requires an installed Git for Windows. The name, path and Git summary share one header row, following the font selected in ai-terminal settings and current zoom; metadata shortens when space is limited. The branch icon uses Microsoft Codicons at the same text size. The summary shows the branch, `+added lines` and `−deleted lines` for the repository containing the current directory. Click the branch to toggle the right panel; drag its left edge to resize it. The workspace saves the open preference, and panel operations keep the terminal running.

Changes lists two-column status codes, relative paths and per-file line totals. The columns describe the index and working tree; `??` means untracked. History shows commits reachable from the current HEAD, including merged history, and follows branch changes. Read 50 commits initially, then use Load more commits up to 300. Click a commit for its full ID; merge commits also show their parent count.

Line totals are net changes against HEAD, combining staged and unstaged changes without double counting. New untracked text contributes additions. An unborn repository counts current file contents as additions. Binary files have no line counts. Files with content-filter attributes (such as Git LFS) do not run their filters; their counts are unavailable and totals are marked partial. Untracked files over 1 MiB each, beyond an 8 MiB read budget, or not regular files are marked unavailable. The file list is capped at 500 entries; exceeding it marks totals partial, while tracked-file totals still come from Git. Changes inside submodules are excluded. These are neither per-commit statistics nor comparisons with remote branches.

Windows directory notifications trigger coalesced background refreshes. Git-ignored build files do not trigger a full reread. Changing the terminal directory, returning to a Shell prompt, refocusing the app and clicking Refresh also update the view. Non-repositories, missing Git, unavailable directory monitoring and query failures have explicit messages. Retained stale data after failure can be refreshed manually. There is no periodic scan or continuous idle repaint.

This version reads local branches, status and history. Staging, committing, fetching, pushing and switching branches remain terminal operations. Queries do not access the network, store file contents or credentials, modify Git configuration, bypass repository ownership checks, or execute repository hooks, external diff, textconv or content filters. Lazy fetching of missing objects and transport protocols are disabled. Each Git command has a six-second timeout and a 2 MiB output cap; exceeding either reports failure while the terminal stays usable.

Implementation references: [Git status](https://git-scm.com/docs/git-status), [Git diff](https://git-scm.com/docs/git-diff), [Git log](https://git-scm.com/docs/git-log).
