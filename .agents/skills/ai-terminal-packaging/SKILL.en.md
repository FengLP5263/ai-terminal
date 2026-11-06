# Local artifacts

[简体中文](SKILL.md) | English

Read root [AGENTS.en.md](../../../AGENTS.en.md) and the [data and assets record](../../architecture/data-and-assets.en.md). Windows x64, Python 3.11+, Rust, and C++/SDK are required. Run CI and the relevant native smoke first; visual/input changes also need actual screenshot acceptance.

```powershell
.\scripts\windows\project.ps1 package
.\scripts\windows\project.ps1 verify-package -Path .\output\packages\<package>.zip
```

The POSIX companion can verify a ZIP cross-platform; native build/package require Windows. Package invokes Cargo release, creates a unique versioned folder and ZIP, records actual Git/dirty state, compiler, source inputs and hashes, archives manager/helper and licenses, then reads the ZIP back. The source identity includes `build.rs` and `assets/branding`; the icon is embedded. A clean tree is required by default. Explicit `-AllowDirty` is for local acceptance only and sets the manifest accordingly.

Do not overwrite a running preview, alter user data, create tags, or publish without authorization. Missing collected dependency licenses must be recorded; automated collection is not a complete legal review. Signing, public releases, and auto-update are not automatic. When packaging rules change, update Python regressions, both platform entry points, script documentation, README, and the data/asset record.

Packaging reads Cargo metadata's `target_directory`, respecting `CARGO_TARGET_DIR` / `build.target-dir`. If a running preview locks the default EXE, set a separate ignored target directory for the package command; never use an old binary by mistake.

Artifact source identity must also cover the local egui_glow Cargo manifest, Rust sources, and GLSL shaders; licenses are gathered through Cargo metadata.

After the native-console upgrade, new packages use format_version 2 and must include conpty.dll, OpenConsole.exe, Microsoft's original license and provenance, all covered by source identity. Verify pinned assets/windows/conpty hashes before copying; missing companions block delivery and cannot be filled from another preview. Preserve read-only verification of older ZIP formats.

Before building, require a unique, dated, categorized, nonempty current Cargo-version entry in both CHANGELOG.md/CHANGELOG.en.md, with matching dates. Reject missing notes, include both full changelogs in the ZIP, and cover their bytes in source identity. Existing governance, ci, and package entry points perform these checks; Windows/POSIX arguments are unchanged.

Source identity must also cover the local winit patch Cargo manifest, build.rs, Rust sources and embedded assets. Retain its upstream Apache-2.0 license and bilingual UPSTREAM provenance, and collect it in the dependency license inventory through Cargo metadata.

ZIPs include icon provenance and available original licenses in agent-icons/, plus bilingual THIRD_PARTY_NOTICES. Check license scope per SOURCE.json entry rather than assigning Lobe MIT to independent official brand assets; read back packaged provenance and license bytes. Windows/POSIX entry points and arguments are unchanged.

Include the 0.4.9 local alacritty_terminal/vte manifests and Rust sources in source identity; collect original Apache-2.0/MIT licenses through dependency inventory. Packaging regression must prove changes to either crate production source alter source_sha256.

0.4.9-preview.3 embeds four static Cascadia Mono faces by default and uses system Microsoft YaHei UI for CJK; preview.1 GDI classic fitting remains reverted. package checks all asset bytes/hashes against assets/fonts/SOURCE.json and copies the original Cascadia OFL/provenance. verify-package retains old-archive compatibility and requires provenance/license files when new fonts metadata is declared. Windows/POSIX entry points and arguments are unchanged.


0.4.10 installer entry: `project.ps1 installer`, optionally using -Path for a verified ZIP; without it, build the ZIP first. Dirty sources/payloads require explicit local -AllowDirty. EXE/.sha256/.json record ZIP, script and complete NSIS toolchain identities. Portable `verify-installer -Path <json>` checks EXE digests only, not installation/signing. -Acceptance/-AcceptanceId create isolated acceptance identities; native smoke refuses normal installers. System choices add no bundled font files; FONT-SOURCE.json carries the catalog with JetBrains Mono as the default, matching the runtime and manifest. New actions require matching Windows/POSIX entries, bilingual script guidance and installation docs; no automatic commits/publication.
