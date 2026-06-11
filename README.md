# S5D

> **Status: alpha / experimental**

S5D is a control plane for repository changes with AI participation.

Not a replacement for git, tests, or planning tools. Its machine core tracks desired state, actual code state, transitions, evidence, invariants, and violations; its human layer renders that state in plain language. Four things on top of normal development:

1. **Target state** — describe what the system should become and which existing architecture it reuses.
2. **Explicit choice** — compare alternatives before committing to one.
3. **Run evidence** — record what agents/tools actually produced.
4. **Verify in code** — check that code matches the decision; trace, reconcile, or roll back when it doesn't.

Human-facing scope, ETA, phase names, and status summaries are derived explanations, not the source of truth.

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
s5d admin update check
s5d admin update apply
# Sessions self-update: the plugin's SessionStart hook runs `s5d update auto`,
# which applies in the background when the checkout is clean and on the default
# branch (otherwise it prints the prompt). Opt out with S5D_AUTO_UPDATE=0.
# Trust note: self-update builds and installs whatever origin's default branch
# serves (ff-only) — enable it only for checkouts whose origin you trust.

# Optional, recommended on a repo S5D hasn't seen: build the discovery index
# (file index, evidence, dependency graph under .s5d/discovery/) so specs and
# trace queries can link to real code. Agent flows run a fuller Discover stage.
s5d discover sync

# Create a spec — the scaffold validates out of the box; edit the generated
# placeholder architecture (domain/capability/component, TODO paths) to match reality
s5d new feat.my-feature --product myapp

# Edit the spec YAML, then:
s5d verify validate .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state preview .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state approve .s5d/packages/feat.my-feature__*.s5d.yaml --reviewer reviewername

# Implement your code, then:
s5d verify run-gates .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state import .s5d/packages/feat.my-feature__*.s5d.yaml --verified-by verifiername

# Optional: run a bounded agent task and record evidence
s5d run list .s5d/packages/feat.my-feature__*.s5d.yaml
s5d run start .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype
s5d run task .s5d/packages/feat.my-feature__*.s5d.yaml --phase prototype --engine ralph
s5d run exec .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --engine local-engine
s5d run accept .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --reviewer yourname

# Later: close the loop with telemetry-backed outcome
s5d state reflect .s5d/packages/feat.my-feature__*.s5d.yaml \
  --summary "Telemetry stayed inside target bounds" \
  --verdict confirmed \
  --measurement-window "7d post-ship" \
  --telemetry "grafana://my-dashboard" \
  --heuristic "Keep rollout verdicts tied to explicit telemetry refs"

# Later: verify nothing drifted
s5d state drift-check
```

## Agent Run Control

S5D records agent execution as evidence against a desired system state. It can render that evidence for humans, but the durable state remains machine-readable.

- `s5d run list/start/exec/accept` manages active work state in `.record.yaml`
- `s5d run exec --engine <name>` executes an approved command template from `.s5d/config.yaml`, captures stdout/stderr under `.s5d/runs/`, and records the output hash in `.record.yaml`
- `s5d run task --engine ralph [--mode init|bugfix]` emits a bounded task package for the active work state only
- each `run task` call persists the package under `.s5d/tasks/`
- engine completion does not accept the work; human `run accept` remains explicit
- `s5d run harness start`, `s5d run harness status`, and `s5d run harness exec` add the operational layer: isolated git worktree, clean preflight, heartbeat/status, timeout, and journal under `.s5d/harness/`
- harness state is not run truth; `.record.yaml` remains authoritative for active work state, evidence, gates, and approvals
- `s5d discover sync/check` builds `.s5d/discovery/*`: file index, evidence JSONL, graph JSON, and a metamodel projection. The core is stack-agnostic; language parsers can be added later as optional evidence providers.
- `ralph-init` warms repo context from docs, tests, environment setup, and current test results
- `ralph-bugfix` enforces regression-first bugfix execution with explicit root-cause evidence
- `s5d state reflect --verdict --measurement-window --telemetry` records outcome evidence after rollout

Legacy aliases such as `s5d validate`, `s5d preview`, `s5d apply preview`, `s5d phase run`, `s5d execute loop`, `s5d harness start`, `s5d update check`, and `s5d install` remain callable for existing scripts, but the public surface shown in `s5d --help` uses `verify`, `state`, `run`, and `admin`.

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

## CI enforcement

```bash
s5d ci init            # GitHub Actions (default); --gitlab / --all for more
```

Generates a self-contained PR pipeline that installs the pinned s5d release binary and runs `s5d ci exec` — built-in read-only checks only: spec validation, architecture check for specs that declare the gate, and drift-check. Configured command gates (test/lint/contract) **never execute in generated CI** — a fork PR must not run repo-configured commands on the runner. The generated files carry a version marker; `s5d ci check` reports stale or user-modified config.

## Tiers

| Tier | When | Default gates |
|------|------|---------------|
| Note | Capture context | None |
| Decision | Compare alternatives | Review |
| Lightweight | Simple feature | Schema |
| Standard | Regular feature | Schema + Graph |
| High | Auth / payment / PII | Schema + Graph + Review |

The review gate is satisfied by recorded review evidence: `s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass ...` (works on decision- and high-tier specs).

Schema, graph, and architecture gates run built-in validation. Add `architecture` to a spec to check component paths and declared source dependencies. Use `s5d codebase sync/check` when you track `.s5d/codebase/*` coverage snapshots, and `s5d discover sync/check` when you track `.s5d/discovery/*` discovery artifacts. Add `lint`, `test`, `contract` gates to your spec when you've configured commands for them in `.s5d/config.yaml`.

`install.sh` must be run from a checked-out repository copy. `curl | bash` is intentionally unsupported.

## Documentation

- `skills/s5d/SKILL.md` — command reference and flow
- `skills/s5d/metamodel.md` — artifact definitions and validation rules
- `skills/s5d/session-protocol.md` — WAL format, conflict resolution

## License

MIT
