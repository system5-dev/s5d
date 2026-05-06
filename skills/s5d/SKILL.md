---
name: s5d
description: "S5D — control plane for agentic repo changes. Target state → run evidence → decision record → trace → verify."
argument-hint: "[question, feature request, or problem description]"
---

# S5D

A control plane for changes in a repository with AI participation. Not a methodology. The machine core tracks desired state, actual code state, transitions, evidence, invariants, and violations; the human layer renders that state in plain language. Four things on top of normal development:

1. **Target state** — describe what the system should become and which existing architecture it reuses (DDD decomposition: domains, capabilities, components).
2. **Explicit choice** — compare alternatives before committing to one (>=3 hypotheses, different in kind).
3. **Run evidence** — record what agents/tools actually produced, with integrity (approval chain, SHA256 binding, ledger).
4. **Verify in code** — check that the code still matches the decision, and roll back when it doesn't (drift-check, reconcile, rollback).

Human-facing scope, ETA, phase names, and status summaries are derived explanations, not the source of truth.

If a term doesn't serve one of these four, it doesn't belong. If an artifact isn't read by a human or a gate, it doesn't belong. If a simple change can't pass through almost in a straight line, the system is lying about its simplicity.

For non-trivial work, describe the target system state before process details: what should exist, why, and how code will prove it. Do not substitute scopes, ETA, phase labels, or status rituals for target-state reasoning.

The flow sequences these four:

```
Route → Target → Decide → Spec → Run → Verify → Ship → Learn
```

Applies only to work grounded in an existing repository. No codebase, no S5D.

**Reference docs** (read when needed, not upfront):
- [metamodel.md](metamodel.md) — abstract metamodel: concepts, relations, enums, constraints
- [domain-capability-mode.md](domain-capability-mode.md) — Product Intent → domain/capability design → implementation scope mode
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts, effectiveness metrics
- [references/fpf/](references/fpf/) — FPF (First Principles Framework) modular corpus. Load via `references/fpf/agent/load-policy.md`: start with `entrypoints.yaml` / `glossary.yaml` / `query-index.jsonl`, then load 1–5 cards from `cards/`, expand to a full `modules/*.md` only when exact wording is needed. Cite by module id + source span (e.g. `B.5.2:13.3`).

---

## Scope

**Out of scope:** bugfix <30 LOC, config-only, docs-only. Just do them.

| Tier | Steps | Waivers |
|---|---|---|
| Lightweight | Route → Target → Spec → Run → Verify → Ship → Learn | Steps can be waived |
| Standard | Full flow | Steps can be waived |
| Decision | Full flow | Steps can be waived |
| High | Full flow | No waivers allowed |

---

## Agent Modes

S5D is one skill with internal agent modes. Domain-capability design is not a separate skill or methodology; it is the S5D shaping mode used when raw product intent must become an architecture-aware spec.

Canonical product path:

```
Product Intent → Domain-Capability Design → S5D Spec → Run → Verify → Learn
```

Use [domain-capability-mode.md](domain-capability-mode.md) inside Target/Spec when input is a feature request, product note, Jira/Linear ticket, design, transcript, project discovery task, or implementation brief that needs domain boundaries. The mode produces S5D artifacts: domains, capabilities, entities, use cases, components, contracts, edges, gates, allowed changes, and forbidden changes.

Keep discovery state in the current repository by default. Use the existing `.s5d/` package/record flow for decisions and specs; only split into a monorepo or separate architecture repository after the current repo boundary becomes a proven bottleneck.

Do not create or invoke a separate `domain-capability-design` skill. Keep the output in S5D terms and validate with S5D commands.

---

## Cross-cutting

**WAL** — see [session-protocol.md](session-protocol.md). WAL saves are local writes (no permission needed). Ship commits require human permission.

**spec:// URI** — `spec://<module>/<document>#<section>`. Use in WAL, commits, REVIEW markers.

**Conflicts** — see [session-protocol.md](session-protocol.md). Priority: Human > Spec > Code > Tests.

**Metamodel** — enforced by `s5d verify validate` (legacy alias: `s5d validate`). Lightweight specs require capabilities; standard/high specs require domains, capabilities, and components.

---

## Execution Surfaces

Every active spec has two mutable surfaces:

- `.s5d/packages/<spec-id>__<date>.s5d.yaml` — authored intent: problem, artifacts, links, contracts, gates.
- `.s5d/records/<spec-id>__<date>.record.yaml` — runtime state: preview diff, approvals, gate results, import fingerprint, decision record, reflection, outcome verdict.

`s5d_new` creates both. `s5d_add_hypothesis` / `s5d_add_evidence` edit spec YAML. Lifecycle commands such as preview, approve, gates, import, phase, and reflect update the record file; read-only commands do not.

---

## Code Decision & Change Tracking

S5D's job is to answer two questions for code:

1. **Why is this code shaped this way?** Follow `component.paths` → component → capability/domain → decision hypothesis/record.
2. **Did the code drift from the approved architecture?** Use discovery/codebase snapshots, gates, import hashes, and drift-check.

Use `s5d trace <path>` / `s5d_trace` when the question starts from code. Trace is read-only and shows the specs, components, capabilities, and decision records that claim a source path.

Decision records are not generic notes. A decision that changes code must bind to:
- a problem signal and competing hypotheses;
- the winning hypothesis and its linked feature spec (`spec_ref`);
- affected domains, capabilities, components, contracts, and `component.paths`;
- evidence/gates that justify the change;
- the record decision fields written by `s5d decision decide`.

Code tracking uses one path:

```text
spec artifacts -> preview diff -> human approval -> code change -> gates -> import hash -> drift-check -> reflect
```

Use `s5d discover sync/check` for a repository evidence graph and `s5d codebase sync/check` for source coverage. These snapshots support investigation; the `.record.yaml` approval/import chain remains the runtime truth.

Effective gates are part of the code decision contract. If `gates:` is empty, CLI/MCP use tier defaults from the Rust core (`decision: review`, `lightweight: schema`, `standard: schema+graph`, `high: schema+graph+review`). Explicit `gates:` override the defaults.

---

## Skill Runtime Contract

This skill is the human-facing conductor for the S5D CLI. It is not a second state machine.

**Source of truth:**
- CLI commands perform durable state transitions.
- `.s5d/packages/` stores authored intent.
- `.s5d/records/` stores runtime truth.
- `.s5d/config.yaml` stores approved local engine command templates.
- `.s5d/runs/` stores external agent-run artifacts.
- `.s5d/harness/` stores operational worktree journals and heartbeat/status only.
- `.s5d/discovery/` stores stack-agnostic discovery artifacts: file index, evidence JSONL, graph JSON, and metamodel projection.

**The skill may:**
- route the request and choose the next S5D command;
- help draft or edit spec YAML;
- read run artifacts and summarize them for the human;
- normalize engine outputs into hypotheses/evidence/spec edits through S5D commands;
- recommend run acceptance when evidence is sufficient.

**The skill must not:**
- store its own run state outside S5D files;
- treat harness journal state as run truth;
- treat an engine run as accepted evidence;
- call Claude/Codex/Gemini directly for S5D execution when `s5d run exec` applies;
- approve an engine that is not configured as `approved: true` in `.s5d/config.yaml`;
- bypass preview/approve/import hash checks.

If work needs an AI engine inside S5D, use:

```bash
s5d run start <spec> --id <work-state>
s5d run exec <spec> --id <work-state> --engine <approved-engine>
# read .s5d/runs/.../stdout.txt
s5d run accept <spec> --id <work-state> --reviewer <name>
```

For multi-engine hypothesis generation, run multiple approved engines for the same active work state, then merge only the artifacts:

```bash
s5d run exec <decision-spec> --id frame --engine codex-high
s5d run exec <decision-spec> --id frame --engine claude-sonnet
s5d run exec <decision-spec> --id frame --engine gemini-pro
```

After reading the three outputs, deduplicate and add hypotheses/evidence with `s5d decision add-hypothesis` / `s5d decision add-evidence` or by editing the decision spec followed by `s5d verify validate`. Legacy hidden aliases such as `s5d add-hypothesis` remain callable for scripts.

---

## Command Contract

### Core Commands

| Action | MCP | CLI | Hard preconditions |
|---|---|---|---|
| Bootstrap workspace | `s5d_init` | `s5d init [--claude] [--all]` | Safe to re-run. |
| Rust pre-commit hook | — | `s5d hook pre-commit` hidden alias | Read-only. Runs on staged specs/source. Installed by `s5d init` when `.git/` exists. |
| Self-update check | — | `s5d admin update check` | Read-only. Plugin SessionStart may still run legacy `s5d update check --hook`. |
| Self-update apply | — | `s5d admin update apply` | Fast-forwards the S5D checkout, relinks skills, replaces installed binary. Legacy alias: `s5d update apply`. |
| Create scaffold | `s5d_new` | `s5d new` | Scaffold only — does not populate problem/artifacts. CLI `--hypothesis-id` auto-links `spec_ref`. |
| Quick note | `s5d_note` | `s5d new note.<slug> --tier note` | Hidden shorthand alias: `s5d note`. |
| Add hypothesis | `s5d_add_hypothesis` | `s5d decision add-hypothesis` | Decision tier only. Duplicate IDs rejected. Legacy alias: `s5d add-hypothesis`. |
| Add evidence | `s5d_add_evidence` | `s5d decision add-evidence` | Decision tier only. `formality` 1–5 on command surface. Legacy alias: `s5d add-evidence`. |
| Validate | `s5d_validate` | `s5d verify validate` | Read-only. Must pass before preview. Legacy alias: `s5d validate`. |
| Graph check | `s5d_graph_check` | `s5d verify graph-check` | Cycles/layering errors block preview. Legacy alias: `s5d graph-check`. |
| Architecture check | — | `s5d verify check <spec>` | Read-only. Validates component paths and declared source dependencies for specs with architecture ownership. Legacy alias: `s5d check`. |
| Codebase coverage | — | `s5d codebase sync` / `s5d codebase check` | Maintains `.s5d/codebase/*` from source files and component paths. Pre-commit checks it when the snapshot exists. |
| Discovery graph | — | `s5d discover sync` / `s5d discover check` | Maintains `.s5d/discovery/*`: stack-agnostic file index, evidence JSONL, graph JSON, and metamodel projection. |
| Preview | `s5d_preview` | `s5d state preview` | Records `previewed_spec_sha256`. Stale after spec change. Legacy aliases: `s5d preview`, `s5d apply preview`. |
| Approve | `s5d_approve` | `s5d state approve --reviewer <name>` | Must be `previewed`. Binds `spec_sha256` + `diff_sha256`. Legacy aliases: `s5d approve`, `s5d apply approve`. |
| Run gates | `s5d_run_gates` | `s5d verify run-gates` | Runs effective gates. Built-ins: schema, graph, review, architecture. `review` requires ≥1 `evidence_type=gate:review` with `verdict=pass`. Legacy alias: `s5d run-gates`. |
| Waive gate | `s5d_waiver` | MCP only | Gate kind must exist in the effective gate contract. |
| Import | `s5d_import` | `s5d state import --verified-by <name> [--force]` | Requires: explicit verifier, approved spec, spec hash match, diff hash match, all gates passed/waived. Legacy aliases: `s5d import`, `s5d apply import`. |
| Decide | `s5d_decide` | `s5d decision decide --confirmed-by <name>` | Decision tier. Winner must have `spec_ref`. Human confirmation required. Legacy alias: `s5d decide`. |
| Reflect | `s5d_reflect` | `s5d state reflect --summary ... --heuristic ... [--verdict ...] [--measurement-window ...] [--telemetry ...]` | Writes to record only (not spec). Legacy aliases: `s5d reflect`, `s5d apply reflect`. |
| Route | `s5d_route` | `s5d route` hidden alias | Classifies into tier + mode + entry point. |

### Agent Run Commands

| Action | MCP | CLI | Hard preconditions |
|---|---|---|---|
| List work states | `s5d_phase_list` | `s5d run list <spec>` | Spec must have a `workflow` block and an existing `.record.yaml`. Legacy alias: `s5d phase list`. |
| Start work state | `s5d_phase_start` | `s5d run start <spec> --id <state>` | Spec must be approved or later. No other state may already be active. Legacy alias: `s5d phase start`. |
| Run external engine | — | `s5d run exec <spec> --id <state> --engine <name>` | State must be active. Engine must be approved in `.s5d/config.yaml`. Captures stdout/stderr under `.s5d/runs/` and records the output hash in `.record.yaml`. Does not accept the run. Legacy alias: `s5d phase run`. |
| Accept run evidence | `s5d_phase_accept` | `s5d run accept <spec> --id <state> --reviewer <name>` | State must already be active. Human reviewer required. Legacy alias: `s5d phase accept`. |
| Emit Ralph task package | `s5d_execute_loop` | `s5d run task <spec> --phase <id> --engine ralph [--mode init|bugfix]` | State must be active. Workflow engine must match and currently only `ralph` is supported. Each run persists a task artifact under `.s5d/tasks/`. Legacy aliases: `s5d phase loop`, `s5d execute loop`. |
| Start operational harness | — | `s5d run harness start <spec> --phase <id> --name <id>` | Requires clean source worktree unless `--force`. Creates an isolated git worktree, starts the state there, and writes `.s5d/harness/<id>.yaml`. Legacy aliases: `s5d phase harness start`, `s5d harness start`. |
| Harness status | — | `s5d run harness status <id>` | Shows worktree, branch, active state, heartbeat freshness, current command, and last event. Harness status is operational visibility only. Legacy aliases: `s5d phase harness status`, `s5d harness status`. |
| Harness command | — | `s5d run harness exec <id> --timeout-s <n> -- <cmd> ...` | Runs an argv command in the harness worktree, captures stdout/stderr under `.s5d/harness/<id>/commands/`, records timeout/failure/completion in the journal. Legacy aliases: `s5d phase harness exec`, `s5d harness exec`. |

Ralph run modes stay runtime-only for now:
- `init` — warm up repository context from docs, tests, environment setup, and test-suite output
- `bugfix` — regression-first bugfix loop: failing test, minimal fix, targeted run, full suite, root-cause summary
- default with no `--mode` — inferred from the phase; `prototype` warms context, everything else stays generic

### Recovery Commands

| Situation | Command | Effect |
|---|---|---|
| Check lifecycle state | `s5d_status` / `s5d status` | Lists spec status and sync state. |
| Inspect spec | `s5d_show` / `s5d show` | Spec shape. For approval/gate truth, check `.record.yaml`. |
| Trace code path | `s5d_trace` / `s5d trace <path>` | Shows specs, components, capabilities, and decisions that claim a source path. |
| Verify applied state | `s5d_drift_check` / `s5d state drift-check` | Compares live state to last applied fingerprint. Legacy aliases: `s5d drift-check`, `s5d apply drift-check`. |
| Fix drifted state | `s5d_reconcile` / `s5d state reconcile` | Re-imports without re-approval. Legacy aliases: `s5d reconcile`, `s5d apply reconcile`. |
| Undo import | `s5d_rollback` / `s5d state rollback` | Tombstones last import for that spec. Legacy aliases: `s5d rollback`, `s5d apply rollback`. |

---

## Worked Example

Decision-first, feature-second path:

```bash
# 1. Bootstrap + decision skeleton
s5d init
s5d new decision.refresh-rotation --tier decision --product auth \
  --question "How should refresh tokens rotate?"

# 2. Hypotheses + evidence
s5d decision add-hypothesis <spec> --title "Server-side rotation" \
  --content "Rotate on every refresh, persist token family state" --scope "auth boundary"
s5d decision add-evidence <spec> --hypothesis-id server-side-rotation \
  --evidence-type internal --content "Revocation lookup <5ms at p95" \
  --verdict pass --formality 4 --claim-scope latency --reliability 0.8

# 3. Feature spec linked to winner
s5d new feat.refresh-rotation --tier standard --product auth \
  --hypothesis-id server-side-rotation

# 4. Validate + decide + preview
s5d verify validate <feature-spec>
s5d verify graph-check <feature-spec>
s5d decision decide <decision-spec> --title "Use server-side rotation" \
  --winner server-side-rotation --confirmed-by Roman \
  --context "Revocation correctness > token statelessness" \
  --decision "Adopt server-side rotation" \
  --rationale "Best revocation/complexity balance" \
  --consequences "Needs persistent token-family store"
s5d state preview <feature-spec>
# → WAL: status=AWAITING_HUMAN, pending=approve

# 5. After human approval: approve → build → gates → import → verify
s5d state approve <feature-spec> --reviewer Roman
# implement code
s5d verify run-gates <feature-spec>
s5d state import <feature-spec> --verified-by Diana

# 6. Ship (explicit human permission for push/deploy)

# 7. Learn
s5d state reflect <feature-spec> --summary "Shipped cleanly" \
  --heuristic "Link winner to feature spec before decide"
```

Hidden legacy CLI aliases such as `s5d add-hypothesis`, `s5d validate`, `s5d apply preview`, `s5d phase run`, `s5d execute loop`, and `s5d harness start` remain supported for existing scripts, but the grouped forms above are the canonical surface.

---

## Route & Bootstrap

Classify before touching tools. First match wins.

**Out of scope (exit S5D):** bugfix <30 LOC, config-only, docs-only, status query (`s5d_status`/`s5d_show`).

**Tier:**
- Choice/tradeoff/architecture question → `decision`
- Feature, 1 domain, no auth/payment/security → `lightweight`
- Feature, 2+ domains → `standard`
- Touches auth/payment/security/PII/compliance → `high`
- Ambiguous → pick the higher tier.

**Mode:**
- Raw product intent / ticket / design / transcript → `prepare` (run Domain-Capability Design in Target/Spec)
- "Evaluate/compare" → `prepare` (analyze + target framing, stop for human)
- "Implement X" with clear architecture → `execute` (auto-waiver Target+Decide)
- No signal → `prepare`

Emit routing explicitly:
```
Route: tier=standard, mode=prepare, entry=Target
Reason: touches auth + payments, needs target-state framing
```

`s5d_init` if no `.s5d/` directory. Proceed to entry point.

---

## Target

State what's anomalous. Define acceptance BEFORE options.

`s5d_new` (tier: decision) creates skeleton. Fill problem card and add hypotheses/evidence via YAML or `s5d_add_hypothesis`/`s5d_add_evidence`.

For raw product intent, first run Domain-Capability Design mode: extract feature intent and use cases, discover current architecture, map impacted capabilities/entities/components/UX surfaces, then decide whether this is lightweight/standard/high. Use the resulting map to populate the feature spec; do not jump directly from product text to code.

When multiple engines are approved and the task benefits from independent generation, use `s5d run exec` for each engine and treat the outputs as evidence, not decisions. Do not invoke provider CLIs directly from the skill for S5D run work.

---

## Decide

>=3 hypotheses, different in kind. For each: predictions, decomposition, rigor rating, weakest-link analysis.

**Challenge probes before `s5d_decide`:**
- Lightweight: 1 probe — strongest counter-argument
- Standard/High: 5 probes — counter-argument, tail failure, evidence weakness, weakest link, existing alternatives

If a probe reveals a fatal flaw, stop — revisit hypotheses.

**Human confirms (non-waivable).** WAL: `status=AWAITING_HUMAN`. If winner needs implementation, create linked feature spec first (`s5d new <id> --hypothesis-id <winner>`).

---

## Spec

Problem → acceptance scenarios (>=3 GWT) → implementation hypotheses (>=2) → winner → DO/DON'T.

`s5d_new` creates scaffold. Write YAML with metamodel artifacts. For feature specs, trace every user-facing change as `Feature → UseCase → Capability → Component` and declare cross-domain edges/contracts explicitly. `s5d_validate` + `s5d_graph_check`.

---

## Run

`s5d_preview` → `s5d_approve` (**human name required, non-waivable**).

WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop.

After approval: optionally use `s5d run start` / `s5d run exec` for bounded engine work, then implement → run local tests → `s5d_run_gates`. Local commits allowed. REVIEW markers for non-obvious decisions.

Engine completion is only evidence. Human acceptance is explicit:

```bash
s5d run accept <spec> --id <state> --reviewer <name>
```

---

## Verify

Tests → `s5d_import` (SHA256 chain). Human reviews diff. Import rejects on hash mismatch → re-preview, re-approve.

---

## Ship

Push and deploy require explicit human permission per action.

---

## Learn

`s5d_reflect`. Update WAL. Record reusable heuristics.

---

## Waiver

Only way to skip a step:
```
WAIVER: <step> | Reason: <why> | Condition: <when required again> | Approved: <name>
```
Non-waivable: Decide human confirmation, Run approval.
Route-to-Spec is an auto-waiver for Target and Decide. Record it explicitly.
