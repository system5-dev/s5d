# S5D

> **Status: alpha / experimental**

A thin decision-and-validation layer for changes in a repository with AI participation.

Not a methodology. Not a replacement for git, tests, or planning tools. Four things on top of normal development:

1. **Explicit choice** — compare alternatives before committing to one.
2. **Reuse architecture** — describe changes in terms of the existing codebase.
3. **Record decisions** — write down what was decided, with integrity.
4. **Verify in code** — check that the code still matches the decision. Roll back when it doesn't.

Optional on top of that: a workflow shell for teams that want S5D to support an existing process without replacing it. Specs can now carry workflow mode, role map, phased execution, review policy, and telemetry-backed outcome verdicts.

## How it works

Two files per change: a **spec** (what you intend) and a **record** (what happened). The spec can also carry optional workflow support for phases and roles; the record can close the loop with telemetry refs and an explicit verdict. The CLI enforces a SHA256 chain between them:

```
spec → preview → approve → import → drift-check
                                         ↓
                                    reconcile / rollback
```

## Quick start

```bash
# Install
git clone https://github.com/system5-dev/s5d.git
cd s5d
./install.sh

# Initialize in your repo
s5d init
# If the repo has .git/, init installs a Rust-backed pre-commit hook.
# Manual hook entrypoint: s5d hook pre-commit

# Check or apply S5D binary/skill updates
s5d update check
s5d update apply

# Optional: keep codebase ownership coverage current
s5d codebase sync
s5d codebase check

# Create a spec
s5d new feat.my-feature --product myapp

# Edit the spec YAML, then:
s5d validate .s5d/packages/feat.my-feature__*.s5d.yaml
s5d preview .s5d/packages/feat.my-feature__*.s5d.yaml
s5d approve .s5d/packages/feat.my-feature__*.s5d.yaml --reviewer reviewername

# Implement your code, then:
s5d run-gates .s5d/packages/feat.my-feature__*.s5d.yaml
s5d import .s5d/packages/feat.my-feature__*.s5d.yaml --verified-by verifiername

# Optional: run a bounded workflow phase with Ralph
s5d phase list .s5d/packages/feat.my-feature__*.s5d.yaml
s5d phase start .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype
s5d execute loop .s5d/packages/feat.my-feature__*.s5d.yaml --phase prototype --engine ralph
s5d phase run .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --engine local-engine
s5d phase accept .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --reviewer yourname

# Later: close the loop with telemetry-backed outcome
s5d reflect .s5d/packages/feat.my-feature__*.s5d.yaml \
  --summary "Telemetry stayed inside target bounds" \
  --verdict confirmed \
  --measurement-window "7d post-ship" \
  --telemetry "grafana://my-dashboard" \
  --heuristic "Keep rollout verdicts tied to explicit telemetry refs"

# Later: verify nothing drifted
s5d drift-check
```

## Workflow Shell

When a team already has its own delivery/discovery process, S5D can support it instead of replacing it.

- `s5d phase list/start/run/accept` manages the active workflow phase in `.record.yaml`
- `s5d phase run --engine <name>` executes an approved command template from `.s5d/config.yaml`, captures stdout/stderr under `.s5d/runs/`, and records the output hash in `.record.yaml`
- `s5d execute loop --engine ralph [--mode init|bugfix]` emits a bounded task package for the active phase only
- each `execute loop` call persists the package under `.s5d/tasks/`
- engine completion does not accept the phase; human `phase accept` remains explicit
- `ralph-init` warms repo context from docs, tests, environment setup, and current test results
- `ralph-bugfix` enforces regression-first bugfix execution with explicit root-cause evidence
- `s5d reflect --verdict --measurement-window --telemetry` records outcome evidence after rollout

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

Schema, graph, and architecture gates run built-in validation. Add `architecture` to a spec to check component paths and declared source dependencies. Use `s5d codebase sync/check` when you track `.s5d/codebase/*` coverage snapshots. Add `lint`, `test`, `contract` gates to your spec when you've configured commands for them in `.s5d/config.yaml`.

`install.sh` must be run from a checked-out repository copy. `curl | bash` is intentionally unsupported.

## Documentation

- `skills/s5d/SKILL.md` — command reference and flow
- `skills/s5d/metamodel.md` — artifact definitions and validation rules
- `skills/s5d/session-protocol.md` — WAL format, conflict resolution
- `docs/s5d-v2-workflow-spec.md` — workflow-shell target state and rollout plan

## License

MIT
