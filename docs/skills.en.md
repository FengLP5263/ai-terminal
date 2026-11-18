# Agent Skill package

[简体中文](skills.md) | English

The user-facing [skills/ai-terminal](../skills/ai-terminal/SKILL.en.md) is separate from the source-development/testing skills under .agents/skills. It contains a standard SKILL.md, English counterpart, Agent display metadata, and references for sessions, groups, team plans, protocol and troubleshooting. It needs neither this repository's source nor an extra MCP service; commands are provided by the installed ai-terminal-cli.

## Install

Copy the entire ai-terminal folder into your Agent's skill directory, retaining references and agents. Agents supporting project-local .agents/skills can use `<project>/.agents/skills/ai-terminal/`; other Agents use their documented directory. Do not copy only the entry file or put it in AI Terminal's data directory. No user hooks, authentication or model configuration changes are needed.

The source distribution includes skills/ai-terminal. Build a standalone ZIP locally:

```powershell
python scripts/skill_bundle.py build
python scripts/skill_bundle.py verify --path output/skills/<skill-archive>.zip
```

It includes ai-terminal/, LICENSE.txt and a verifiable SHA-256 manifest. Full Windows software ZIPs also include the CLI and skill directory. Install a CLI-enabled version first if your old package lacks it; installing a Skill does not add an API to an older manager.

## Example request

> Use $ai-terminal to keep this session as coordinator, create frontend/backend sessions, form a Product Engineering group, order coordinator/frontend/backend, and launch Codex and Qoder respectively.

The Agent checks instance/caller identity, previews and commits metadata through the team reference, launches members separately, and reads back results. Ordinary changes use existing user authorization; closing a running session terminates its owned process tree. References also cover settings, viewing and terminal interaction. Content reads/task submission must fit user authorization; automatic trust approval is never authorized by this Skill.

See [CLI documentation](cli.en.md) and the [Skill entry](../skills/ai-terminal/SKILL.en.md).
