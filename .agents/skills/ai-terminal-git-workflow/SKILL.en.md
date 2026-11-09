# Git collaboration and releases

[简体中文](SKILL.md) | English

Use this workflow for this repository's branches, commits, PRs, tags, and releases. See the development, testing, and packaging skills for their respective workflows. Read root [AGENTS.en.md](../../../AGENTS.en.md) and [CONTRIBUTING.en.md](../../../CONTRIBUTING.en.md). Inspection alone does not authorize commit, push, merge, publication, or deletion; honor existing task authorization without repeated questions. Do not bypass branch protection or review, rewrite shared history, or force-push without explicit authorization.

## Branches and daily work

- `main` is the public release line; `develop` is the integration line and may be ahead. Branch independent features from current `develop` as `feature/<name>` and fixes as `fix/<name>`; small changes may be made on `develop`. External contributors may open PRs from a fork to `develop`.
- Before work, check `git status --short --branch`, `git worktree list`, and upstreams. Fetch when remote collaboration is needed and record exact source/target SHAs. Preserve uncommitted work and do not switch another person's workspace.
- After feature/fix commits, check for an existing PR before opening one against `develop`. Describe visible changes, compatibility, tests actually run, and gaps. Test according to scope. PR creation, merge, and deletion require the relevant user authorization and repository permissions.
- Feature branches are temporary. After merge, inspect the PR, target code, unique commits, other open PRs, and worktree use. Delete only with authorization; remote deletion uses a lease on the exact old SHA. Do not automatically turn a rejected `git branch -d` into `-D`. Rebase/squash changes SHAs; equal patches or trees alone do not prove the original commit has another durable reference. Check and report recovery evidence; do not promise unreferenced objects will persist forever.

## Merge strategy and history

Preserve existing commits and clear future PRs; do not force branches into apparent alignment. Check protection, available merge methods, exact SHAs, and ancestry before following the PR review process. Prefer an actual fast-forward when available within that process; never bypass required review or protection by directly pushing a branch.

Promote `develop` to `main` through a release PR. A platform rebase may rewrite commit IDs, and squash creates a new commit; neither is automatically a SHA-preserving fast-forward. If an actual reviewed fast-forward is unavailable, prefer a PR merge commit to preserve identity and avoid duplicate copies of the same work on long-lived branches. Then, subject to permissions and authorization, fast-forward `develop` to updated `main`; if a PR is required, use a merge that preserves parents. Necessary merge commits are preferable to rewriting shared history for cosmetic linearity. Repository protection and review rules still apply.

After merge, refresh and verify the result rather than trusting a button name or PR status:

```text
git fetch origin
git merge-base --is-ancestor origin/main origin/develop
git rev-list --left-right --count origin/main...origin/develop
git diff --stat origin/main origin/develop
git log --graph --oneline --decorate -n 20 --all
```

`merge-base --is-ancestor` returns 0 if `main` is an ancestor of `develop`, 1 if not, and other codes for errors. `rev-list` compares history and `diff` compares final trees: an empty diff does not imply identical commit identities. `develop` may contain unpublished work; require an empty tree diff only when claiming both contain the same code. A GitHub PR's three-dot diff is relative to the merge base, not a direct comparison of both branch tips.

For divergence, remote movement, or non-equivalent changes, preserve both refs and inspect SHAs, graph, patches, source, and related PRs. Do not automatically rebase shared branches, force-push, or overwrite commits. With authorization use a history-preserving sync, then verify ancestry and code; stop and reassess if the target changes or conflicts remain.

## Release delivery

Choose the version from the entire range since the previous stable tag using [Version and release rules](../../../docs/versioning.en.md): features advance minor, compatible fixes advance patch, and mixed scopes take the highest level. Check 0.x and stable-contract conventions; do not simply strip a preview suffix for a stable version. Never overwrite older tags/assets.

Before releasing, review the scope against the previous release tag and update the current-version entries in root CHANGELOG.md/CHANGELOG.en.md with actual additions, improvements, fixes, upgrade guidance, and unverified cases. Dates must match. Base historical summaries on existing releases without inventing unreleased versions. Generate GitHub Release notes from the same entries, then add validation and asset hashes. Packaging/governance reject missing, duplicate, empty, or date-mismatched entries. Verify both the online notes and both changelog files in the downloaded package.

When authorized, build from the reviewed target commit. Run the relevant testing skill checks and native acceptance; use the packaging skill to build and verify a clean-source ZIP, never a dirty preview as a clean release. Check version, existing tag, and target commit before creating and pushing an annotated `app-v<SemVer>` tag and, when authorized, a GitHub Release. Verify the remote tag, assets, and downloaded hashes. Report tag, package, PR, and synchronization results separately; do not overwrite running binaries, close user sessions, or alter their data.
