# S5D — Software Architecture Decision & Feature Framework

Three layers. One path. Modular skills.

S5D is an opinionated software-architecture profile built on FPF. It applies to decisions and features grounded in an existing repository. No codebase, no S5D.

**Layer 1 — FPF**: thinking discipline. Frame before solving. Evidence before claiming.
**Layer 2 — Reasoning cycle**: Frame → Hypothesize → Evidence → Audit → Decide.
**Layer 3 — Metamodel**: architectural decomposition. Domains → Capabilities → Entities → Components.

## Skills

| Skill | What it does |
|-------|-------------|
| `/s5d` | Unified SDLC — decisions, features, traceability, gates, lifecycle |
| `/fpf` | FPF reasoning — problem framing, variants, evidence, ADI cycle |

## Install

### Claude Code
```
/plugin install s5d@system5-dev/s5d
```

### Gemini CLI
```bash
gemini extensions install https://github.com/system5-dev/s5d
```

### OpenAI Codex
Add to `.agents/plugins/marketplace.json`:
```json
{"plugins": [{"name": "s5d", "source": "github:system5-dev/s5d"}]}
```

### Universal (all agents)
```bash
curl -fsSL https://raw.githubusercontent.com/system5-dev/s5d/main/install.sh | bash
```

## Usage

```
/s5d should I use PostgreSQL or SQLite?
/s5d add authentication to the API
/fpf evaluate these three deployment strategies
```

Use S5D when the question is architectural and tied to a real codebase.

## The pendulum

Descend from the reasoning cycle into Metamodel to decompose hypotheses into domains/capabilities, return with decomposition AS evidence, then decide.

## Feature lifecycle

```
Spec → Preview → Approve → Test Contract → Execute → Verify (gates) → Reflect
```

## Tiers

| Tier | When | Gates |
|------|------|-------|
| Note | Trivial, reversible | None |
| Decision | Hard to reverse, trade-offs | N/A |
| Lightweight | Simple feature | Schema |
| Standard | Regular feature | Schema + Graph + Lint |
| High | Critical feature | Schema + Graph + Lint + Test + Contract |

## Based on

- [FPF — First Principles Framework](https://github.com/ailev/FPF) by Anatoly Levenchuk
- S5D Metamodel by Roman Voronin + Ivan Podobed

## Requirements

- Claude Code, Gemini CLI, Codex, or any agent runtime that supports skills
- Existing repository and codebase context
- `s5d` CLI binary (installed automatically)
