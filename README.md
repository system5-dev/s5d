# S5D

> **Status: alpha / experimental**

A thin decision-and-validation layer for changes in a repository with AI participation.

Not a methodology. Not a replacement for git, tests, or planning tools. Four things on top of normal development:

1. **Explicit choice** — compare alternatives before committing to one.
2. **Reuse architecture** — describe changes in terms of the existing codebase.
3. **Record decisions** — write down what was decided, with integrity.
4. **Verify in code** — check that the code still matches the decision. Roll back when it doesn't.

## How it works

Two files per change: a **spec** (what you intend) and a **record** (what happened). The CLI enforces a SHA256 chain between them:

```
spec → preview → approve → import → drift-check
                                         ↓
                                    reconcile / rollback
```

## Quick start

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/system5-dev/s5d/main/install.sh | bash

# Initialize in your repo
s5d init

# Create a spec
s5d new feat.my-feature --product myapp

# Edit the spec YAML, then:
s5d validate .s5d/packages/feat.my-feature__*.s5d.yaml
s5d preview .s5d/packages/feat.my-feature__*.s5d.yaml
s5d approve .s5d/packages/feat.my-feature__*.s5d.yaml --reviewer yourname

# Implement your code, then:
s5d run-gates .s5d/packages/feat.my-feature__*.s5d.yaml
s5d import .s5d/packages/feat.my-feature__*.s5d.yaml

# Later: verify nothing drifted
s5d drift-check
```

## Non-goals

- Not a replacement for git
- Not a replacement for tests
- Not a planning or project management system
- Not required for small fixes (bugfix <30 LOC, config-only, docs-only — just do them)
- Not a code generator

## Agent integration

S5D works as an MCP server (experimental) for AI coding agents:

```bash
# Claude Code
s5d init --claude

# All supported agents
s5d init --all
```

Available as a plugin for Claude Code, Gemini CLI, and Codex.

## Tiers

| Tier | When | Default gates |
|------|------|---------------|
| Note | Capture context | None |
| Decision | Compare alternatives | None |
| Lightweight | Simple feature | Schema |
| Standard | Regular feature | Schema + Graph |
| High | Auth / payment / PII | Schema + Graph |

Schema and graph gates run built-in validation. Add `lint`, `test`, `contract` gates to your spec when you've configured commands for them in `.s5d/config.yaml`.

## Documentation

- `skills/s5d/SKILL.md` — command reference and flow
- `skills/s5d/metamodel.md` — artifact definitions and validation rules
- `skills/s5d/session-protocol.md` — WAL format, conflict resolution

## License

MIT
