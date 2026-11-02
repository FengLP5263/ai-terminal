# Governance and initial acceptance

[简体中文](governance.md) | English

## Pre-adoption baseline

2026-10-30, version 0.1.2. A separate Git repository had been initialized but `develop` had no commits; no remote, additional worktree, tags, or cloud publishing were configured. Production source, Cargo.lock, input/DPI regressions, font provenance, and licenses existed; 36 historical Rust tests passed. The user authorized project governance and the first local commit, not pushing, tags, online publishing, or product features.

## Findings and response

| Gap | Action at that time |
| --- | --- |
| Collaboration rules were only an overview | Merge AGENTS with authority, visual acceptance, source boundaries, and completion criteria. |
| No architecture/data/integration map | Add four `.agents/architecture` records. |
| Workflows depended on conversations and temporary scripts | Add local development, testing, and artifact skills. |
| No paired entry points; platform support unclear | Windows/POSIX project entries; unsupported native actions explicitly fail. |
| No repeatable ZIP delivery | `package.py` builds, inventories licenses/dependencies, records identities and verifies ZIP, tampering, and traversal. |
| Incomplete ignore rules | Exclude runtime data, real configs, keys, databases, archives and caches. |
| Local documentation paths and historical Git status | Use relative paths/environment variables and date historical records. |
| Risk of accidental real-model checks | Require `AllowModelRequest`; default CI does not incur model costs. |
| Missing script help/exit boundaries | Command-first help, UTF-8 BOM, unique output, screenshot/cleanup and failure propagation. |

## Scope and acceptance

No production source was moved/renamed; `src/`, vendor source, font bytes, UI, protocols, data formats, and runtime behavior were preserved. Original scripts and local `.tools` location remained; user terminals and other repositories were untouched. Stable plus Cargo.lock records the actual compiler, not bit-reproducibility. Automated license collection has a defined scope and is not a full legal review.

Windows was the verification platform; Git Bash represented Windows shell behavior, not macOS/Linux native execution. Model verification checked help and explicit refusal without authorization, without another paid turn. No remote existed then, so cloud CI, PR protection, signing, auto-update and remote release were not claimed.

## Final acceptance (2026-10-30, Windows)

- Windows CI entry: governance, fmt, all-target Clippy `-D warnings`, and 36 Rust tests passed. Three Python artifact regressions passed: tampering, traversal, and 1970 license timestamps.
- PowerShell AST/help checks passed; Git Bash syntax, governance and format-check ran from outside the repo. Native release smoke verified JSON, GPU PNG, and exit of three owned PTYs. Idle measurement ran; data represented only a short manager-process window.
- `package -AllowDirty` built Windows x64 with dependency/license lists and read-back ZIP verification; it was explicitly dirty, not a clean release. Timestamp and cross-platform dependency parsing issues were fixed; `.partial` is not a finished ZIP.
- Real-model command checked help/refusal only, without a model call. Ignore checks excluded local configs, artifacts, runtime data and credentials while keeping font source tracked.
- Production Rust, Cargo, and font SHA-256 matched the pre-governance 0.1.2 input: `2c325e0788a497d0270f3aa8c4615fbdc410ffe4ae3cb09a1efac398dba7c9e7`.

Local JSON/PNG/ZIP evidence was in ignored `output/`, summary `output/governance-20261030.json`, reproducible through project scripts. The interactive `run` window was not reopened and user sessions were not changed. The initial local commit put main/develop on the same baseline; no remote was configured/pushed and no tag created then.

The local dependency inventory listed 171 packages, with license text for 13 not found in local source directories; the list was retained for release review and was not a complete compliance assessment.
