# Version and release rules

[简体中文](versioning.md) | English

Starting with 0.5.0, this project chooses versions using [SemVer 2.0.0](https://semver.org/). Classify the complete change range since the previous stable release tag. Mixed changes take the highest level; the last fix commit does not determine the release level alone.

| Change scope | Version change | Example |
| --- | --- | --- |
| Compatible bug/security fixes and corrections to existing display/performance, without a new user feature | Increment patch | 0.5.0 → 0.5.1 |
| Backward-compatible features, including accompanying fixes | Increment minor and reset patch | 0.4.11 → 0.5.0 |
| Incompatible changes to a stable public contract, from 1.0.0 onward | Increment major and reset minor/patch | 1.2.3 → 2.0.0 |
| Local previews before a stable release | Append -preview.N to the target version | 0.5.0-preview.1 |

Public compatibility covers documented user features, launch arguments, saved-session/settings restoration and external hook metadata interfaces. Ordinary visual corrections are patch-level. The first Git workspace view is a new feature, with the color/header corrections included in 0.5.0.

SemVer treats 0.y.z as initial development without a stable public API promise. The project still distinguishes compatible features from fixes in this phase. An incompatible change must at least advance minor and document migration, backup and rollback. Version 1.0.0 establishes a stable public contract; incompatible changes thereafter advance major. Initial development does not permit discarding sessions or changing data directories to avoid compatibility work.

Before release, record the previous stable tag, full scope, change level and target version. Cargo.toml, Cargo.lock, bilingual CHANGELOG, installer version, artifact names and app-v<version> tags must agree. Run existing governance, tests and archive verification, and build from a reviewed clean mainline commit. GitHub Releases reuse that entry plus actual validation and asset hashes. Permissions and PR flow remain governed by [Git workflow](../.agents/skills/ai-terminal-git-workflow/SKILL.en.md).

Do not overwrite or rename published versions, tags or artifacts. Fix issues in a new version. Historical public releases and delivered local previews retain their original versions. The local 0.4.12-preview.N development is consolidated into stable 0.5.0; no stable 0.4.12 is published.

Starting with 0.17.0, a stable release includes both its clean Windows x64 ZIP and the generated `ai-terminal-windows-x64-update.json` under their original filenames. After publishing, read the fixed attachment on the latest stable GitHub release and verify its version, ZIP filename, size, and SHA-256.


## Release display titles

GitHub Release display titles use `AI Terminal <SemVer>`, for example `AI Terminal 0.13.0`, with fixed project-name capitalization and one space before the version. Keep feature summaries in the bilingual Chinese/English body rather than the title.

Display titles are presentation metadata and historical formatting may be normalized at the user's request. Published version numbers, `app-v<SemVer>` tags, commit targets, asset filenames and contents remain subject to the immutable identity rules above. Verify the display title, corresponding tag and bilingual body after publication.
