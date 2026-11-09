# Contributing

[简体中文](CONTRIBUTING.md) | English

Thank you for improving ai-terminal. Input, rendering, status reliability, performance, documentation, and reproducible reports are welcome.

## Issues and proposals

Search [existing issues](https://github.com/FengLP5263/ai-terminal/issues) first. Include the Windows, ai-terminal, and Agent CLI versions, reproduction steps, expected result, and actual result. For rendering/zoom, include screenshots, display scaling, and monitor configuration; for input, identify the IME. Never upload API keys, authentication files, private conversations, or real session data. Default checks do not need a model account.

Discuss major features or architecture changes in an issue first. Keep the project focused on multiple Agent sessions, a real terminal, reliable status, a simple interface, and reasonable resource use.

## Local development

Set up Windows, Rust MSVC, C++ Build Tools, Windows SDK, and Python as described in the [README](README.en.md). Read [AGENTS.en.md](AGENTS.en.md) and the [architecture overview](.agents/architecture/overview.en.md), then run:

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`main` is the public release branch; `develop` is for integration. Fork from `develop` into `feature/<name>` or `fix/<name>` and normally target `develop` in your PR. Keep commits focused and do not rewrite others' history. Maintainers review changes entering `main`. See the [Git workflow](.agents/skills/ai-terminal-git-workflow/SKILL.en.md) for branch synchronization, PR merges, and releases.

## Preserve these boundaries

- Do not add a browser runtime, implement a model proxy in the manager, or store API keys.
- Derive status from real lifecycle events, not silence or the presence of a process.
- Do not automatically accept CLI project trust, authorization, or model requests; do not change user-level CLI configuration.
- Do not kill terminals by process name. Tests use isolated data and manage only their own processes.
- Preserve bounded history and queues, background output draining, and on-demand redraw; monitor idle resource use.
- Update the [vendor patch record](vendor/egui_term/UPSTREAM.en.md) when changing `vendor/egui_term`; retain provenance, hashes, and licenses for assets.

## Pull requests

First classify the full scope since the previous stable tag with [Version and release rules](docs/versioning.en.md): compatible fixes advance patch, features advance minor and reset patch, and incompatible changes follow public-contract/development-phase rules.

Release changes must update [CHANGELOG.md](CHANGELOG.md) and [CHANGELOG.en.md](CHANGELOG.en.md) for the current Cargo version with matching dates, listing actual additions, improvements, fixes, upgrade notes, and verification limits. GitHub Releases reuse that entry; packaging checks the version entry and includes both files. Record ordinary development under Unreleased, then organize it into a version before release. Do not rewrite historical releases as current behavior.

Explain the problem, visible behavior, and validation. Keep small changes concise. Before submitting, run:

```powershell
.\scripts\windows\project.ps1 ci
```

For terminal display, input, UI, or lifecycle changes, also build an optimized binary and run native smoke:

```powershell
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
```

Include actual screenshots for visual changes; describe tested IMEs and operations for input changes; document build mode, scenario, and method for performance work. Do not present historical results as current checks. Default tests do not call model APIs or require contributors to spend model credits. Documentation and other low-impact changes need relevant focused checks; see [scripts](scripts/README.en.md).

Do not commit `target/`, `output/`, `.tools/`, session data, real configuration, credentials, or packaged files. Update affected docs for protocol, data, or command changes. Project code uses the [MIT License](LICENSE); retain third-party licenses and provenance. Maintain Chinese and English documentation together according to the [bilingual policy](docs/i18n.en.md).
