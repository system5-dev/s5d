---
name: s5d
description: "S5D — control plane for agentic repo changes. Target state → run evidence → decision record → trace → verify."
argument-hint: "[question, feature request, or problem description]"
---

# S5D

A control plane for changes in a repository with AI participation. Not a methodology. Four things on top of normal development:

1. **Target state** — describe what the system should become and which existing architecture it reuses.
2. **Explicit choice** — compare alternatives before committing to one (>=3 hypotheses, different in kind).
3. **Run evidence** — record what agents/tools actually produced, with integrity (approval chain, SHA256 binding, ledger).
4. **Verify in code** — check that the code still matches the decision, and roll back when it doesn't (drift-check, reconcile, rollback).

If a term doesn't serve one of these four, it doesn't belong. If an artifact isn't read by a human or a gate, it doesn't belong. If a simple change can't pass through almost in a straight line, the system is lying about its simplicity.

The flow sequences these four:

```
Route → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn
```

`Discover` runs once per project (or when the architecture map is stale); later tasks skip it. Applies only to work grounded in an existing repository. No codebase, no S5D.

**Reference docs** (read when needed, not upfront):
- [metamodel.md](metamodel.md) — artifact graph, DDD decomposition, validation rules
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts, effectiveness metrics

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

## Cross-cutting

**WAL** — see [session-protocol.md](session-protocol.md). WAL saves are local writes (no permission needed). Ship commits require human permission.

**spec:// URI** — `spec://<module>/<document>#<section>`. Use in WAL, commits, REVIEW markers.

**Conflicts** — see [session-protocol.md](session-protocol.md). Priority: Human > Spec > Code > Tests.

**Metamodel** — enforced by `s5d verify validate` (legacy alias: `s5d validate`). Spec without domains/capabilities/components = validation error.

---

## Execution Surfaces

Every active spec has two mutable surfaces:

- `.s5d/packages/<spec-id>__<date>.s5d.yaml` — authored intent: problem, artifacts, links, contracts, gates.
- `.s5d/records/<spec-id>__<date>.record.yaml` — runtime state: preview diff, approvals, gate results, import fingerprint, decision record, reflection, outcome verdict.

`s5d_new` creates both. `s5d_add_hypothesis` / `s5d_add_evidence` edit spec YAML. Lifecycle commands such as preview, approve, gates, import, run, and reflect update the record file; read-only commands do not.

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
| Architecture check | `s5d_check` | `s5d verify check <spec>` | Read-only. Validates component paths and declared source dependencies for specs with architecture ownership. Legacy alias: `s5d check`. |
| Codebase coverage | `s5d_codebase_sync` / `s5d_codebase_check` | `s5d codebase sync` / `s5d codebase check` | Maintains `.s5d/codebase/*` from source files and component paths. Pre-commit checks it when the snapshot exists. |
| Discovery graph | `s5d_discover_sync` / `s5d_discover_check` | `s5d discover sync` / `s5d discover check` | Maintains `.s5d/discovery/*`: stack-agnostic file index, evidence JSONL, graph JSON, and metamodel projection. |
| Preview | `s5d_preview` | `s5d state preview` | Records `previewed_spec_sha256`. Stale after spec change. Legacy alias: `s5d preview`. |
| Approve | `s5d_approve` | `s5d state approve --reviewer <name>` | Must be `previewed`. Binds `spec_sha256` + `diff_sha256`. Legacy alias: `s5d approve`. |
| Run gates | `s5d_run_gates` | `s5d verify run-gates` | Runs effective gates. Failed gate blocks import. Legacy alias: `s5d run-gates`. |
| Waive gate | `s5d_waiver` | MCP only | Gate kind must exist in the effective gate contract. |
| Import | `s5d_import` | `s5d state import --verified-by <name> [--force]` | Requires: explicit verifier, approved spec, spec hash match, diff hash match, all gates passed/waived. Legacy alias: `s5d import`. |
| Decide | `s5d_decide` | `s5d decision decide --confirmed-by <name>` | Decision tier. Winner must have `spec_ref`. Human confirmation required. Legacy alias: `s5d decide`. |
| Reflect | `s5d_reflect` | `s5d state reflect --summary ... --heuristic ...` | Writes to record only (not spec). Legacy alias: `s5d reflect`. |
| Route | `s5d_route` | `s5d route` hidden alias | Classifies into tier + mode + entry point. |

### Agent Run Commands

| Action | MCP | CLI | Hard preconditions |
|---|---|---|---|
| List work states | `s5d_phase_list` | `s5d run list <spec>` | Spec must have a `workflow` block and an existing `.record.yaml`. Legacy alias: `s5d phase list`. |
| Start work state | `s5d_phase_start` | `s5d run start <spec> --id <state>` | Spec must be approved or later. No other state may already be active. Legacy alias: `s5d phase start`. |
| Run external engine | — | `s5d run exec <spec> --id <state> --engine <name>` | State must be active. Engine must be approved in `.s5d/config.yaml`. Captures stdout/stderr under `.s5d/runs/` and records output hash in `.record.yaml`. Does not accept the run. Legacy alias: `s5d phase run`. |
| Accept run evidence | `s5d_phase_accept` | `s5d run accept <spec> --id <state> --reviewer <name>` | State must already be active. Human reviewer required. Legacy alias: `s5d phase accept`. |
| Emit Ralph task package | `s5d_execute_loop` | `s5d run task <spec> --phase <id> --engine ralph [--mode init\|bugfix]` | State must be active. Workflow engine must match and currently only `ralph` is supported. Each run persists a task artifact under `.s5d/tasks/`. Legacy alias: `s5d execute loop`. |
| Start operational harness | — | `s5d run harness start <spec> --phase <id> --name <id>` | Requires clean source worktree unless `--force`. Creates an isolated git worktree, starts the state there, and writes `.s5d/harness/<id>.yaml`. Legacy alias: `s5d harness start`. |
| Harness status | — | `s5d run harness status <id>` | Shows worktree, branch, active state, heartbeat freshness, current command, and last event. Harness status is operational visibility only. Legacy alias: `s5d harness status`. |
| Harness command | — | `s5d run harness exec <id> --timeout-s <n> -- <cmd> ...` | Runs an argv command in the harness worktree, captures stdout/stderr under `.s5d/harness/<id>/commands/`, and records timeout/failure/completion in the journal. Legacy alias: `s5d harness exec`. |

### Recovery Commands

| Situation | Command | Effect |
|---|---|---|
| Check lifecycle state | `s5d_status` / `s5d status` | Lists spec status and sync state. |
| Inspect spec | `s5d_show` / `s5d show` | Spec shape. For approval/gate truth, check `.record.yaml`. |
| Verify applied state | `s5d_drift_check` / `s5d state drift-check` | Compares live state to last applied fingerprint. Legacy alias: `s5d drift-check`. |
| Fix drifted state | `s5d_reconcile` / `s5d state reconcile` | Re-imports without re-approval. Legacy alias: `s5d reconcile`. |
| Undo import | `s5d_rollback` / `s5d state rollback` | Tombstones last import for that spec. Legacy alias: `s5d rollback`. |

---

## Worked Example

Decision-first, feature-second path:

```bash
# 1. Bootstrap + decision skeleton
s5d init
s5d new decision.refresh-rotation --tier decision --product auth \
  --question "How should refresh tokens rotate?"

# 2. Hypotheses + evidence
s5d add-hypothesis <spec> --title "Server-side rotation" \
  --content "Rotate on every refresh, persist token family state" --scope "auth boundary"
s5d add-evidence <spec> --hypothesis-id server-side-rotation \
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
- "Evaluate/compare" → `prepare` (analyze + target framing, stop for human)
- "Implement X" with a confirmed decision record or stated existing architecture → `execute` (enters at Spec; records the Target+Decide auto-waiver)
- Too vague to tell the domain count → run Discover / Domain-Capability mapping first, then classify
- No signal → `prepare`

Emit routing explicitly:
```
Route: tier=standard, mode=prepare, entry=Target
Reason: touches auth + payments, needs target-state framing
```

Cross-check with `s5d_route` (deterministic keyword classifier) when unsure. On disagreement — especially non-English requests or "X and Y" multi-domain, where the classifier is weaker — trust your semantic read and note the divergence.

`s5d_init` if no `.s5d/` directory. Proceed to entry point.

---

## Target

State what's anomalous. Define acceptance BEFORE options.

`s5d_new` (tier: decision) creates skeleton. Fill problem card and add hypotheses/evidence via YAML or `s5d_add_hypothesis`/`s5d_add_evidence`.

---

## Decide

>=3 hypotheses, different in kind. For each: predictions, decomposition, rigor rating, weakest-link analysis.

**Challenge probes before `s5d_decide`:**
- Lightweight: 1 probe — strongest counter-argument
- Standard / High / Decision: 5 probes — counter-argument, tail failure, evidence weakness, weakest link, existing alternatives

If a probe reveals a fatal flaw, stop — revisit hypotheses.

**Human confirms (non-waivable).** WAL: `status=AWAITING_HUMAN`. If the winner needs implementation, create the linked feature spec **before** `s5d_decide` (`s5d new <id> --hypothesis-id <winner>`) — the winner must carry a `spec_ref` — then confirm.

---

## Spec

Problem → acceptance scenarios (>=3 GWT) → implementation hypotheses (>=2) → winner → DO/DON'T.

`s5d_new` creates scaffold. Write YAML with metamodel artifacts. `s5d_validate` + `s5d_graph_check`.

---

## Run

`s5d_preview` → `s5d_approve` (**human name required, non-waivable**).

WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop.

After approval: optionally use `s5d run start` / `s5d run exec` for bounded engine work, then implement → run local tests → `s5d_run_gates`. Local commits allowed. REVIEW markers for non-obvious decisions.

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

**High-tier "no waivers" ≠ the Target/Decide auto-waiver.** "No waivers" governs *assurance gates* (schema, graph, review, contract, privacy, human approval) — never waivable, any tier. The Target+Decide auto-waiver is *not* a gate waiver: it points at a **prior confirmed decision** (winner + `confirmed_by`), it does not skip one. A high-tier feature with no prior decision record cannot auto-waive Decide — frame it first.
