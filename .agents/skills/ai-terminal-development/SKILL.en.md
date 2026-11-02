# Local development

[简体中文](SKILL.md) | English

Read root [AGENTS.en.md](../../../AGENTS.en.md) and the architecture records. Windows x64 requires Rust stable MSVC and C++ Build Tools/SDK; governance and packaging need Python 3.11+. Prefer PATH; the ignored `.tools/env.ps1` is optional. Do not record personal tool paths in tracked files.

```powershell
.\scripts\windows\project.ps1 governance
.\scripts\windows\project.ps1 build
.\scripts\windows\project.ps1 run
```

The POSIX counterpart is `sh scripts/posix/project.sh <action>`. Git Bash can delegate to Windows; unsupported macOS/Linux native actions fail explicitly. OpenGL is the default renderer; use DX12 only for explicit A/B measurements and do not mix old binary results.

Preserve source layout, the independent helper, font provenance, and session-scoped hooks. Normal exit manages only its own sessions; do not kill existing user CLIs. For fonts, input, or rendering, perform actual screenshot and interaction acceptance; test counts alone do not prove appearance or input compatibility. Follow the [testing skill](../ai-terminal-testing/SKILL.en.md); update architecture, scripts, and vendor `UPSTREAM.md` when affected.

All visible debug windows, floating windows, and example EXEs use the selected B icon in `assets/branding`, not framework defaults or early icons. Check both window icon and EXE resources; the hook helper has no GUI icon.
