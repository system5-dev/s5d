# S5D Project

## Architecture decisions

Use `/s5d` for all architectural decisions within this project. S5D applies to itself — no exception.

Non-trivial changes (new skills, decomposition, plugin format, distribution) go through the full S5D path: Frame → Hypothesize → Evidence → Audit → Decide → Implement.

Trivial changes (typo fixes, <30 LOC refactors, config updates) skip S5D.

## Structure

- `skills/` — agent skills (SKILL.md files)
- `rust/` — CLI binary and MCP server
- `hooks/` — pre-commit validation, update checker, FPF sync
- `.claude-plugin/` — Claude Code marketplace manifest
- `.codex-plugin/` — OpenAI Codex marketplace manifest
- `gemini-extension.json` — Gemini CLI extension manifest
- `install.sh` — universal installer (all agent runtimes)

## Distribution

Repo: `system5-dev/s5d` (GitHub). Distributed via:
- Claude Code plugin marketplace
- Gemini CLI extensions (auto-indexed when public)
- Codex plugin system
- `install.sh` fallback
