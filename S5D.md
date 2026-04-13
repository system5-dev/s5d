---
name: s5d
description: "S5D — thin decision-and-validation layer for repo changes with AI. Think → record → verify → trace to code → roll back."
argument-hint: "[question, feature request, or problem description]"
---

# S5D

A thin decision-and-validation layer for changes in a repository with AI participation. Not a methodology. Four things on top of normal development:

1. **Explicit choice** — compare alternatives before committing to one (>=3 hypotheses, different in kind).
2. **Reuse architecture** — describe changes in terms of the existing codebase, not a new vocabulary (DDD decomposition: domains, capabilities, components).
3. **Record decisions** — write down what was decided and why, with integrity (approval chain, SHA256 binding, ledger).
4. **Verify in code** — check that the code still matches the decision, and roll back when it doesn't (drift-check, reconcile, rollback).

If a term doesn't serve one of these four, it doesn't belong. If an artifact isn't read by a human or a gate, it doesn't belong. If a simple change can't pass through almost in a straight line, the system is lying about its simplicity.

The flow sequences these four:

```
Route → Frame → Decide → Spec → Build → Verify → Ship → Learn
```

Applies only to work grounded in an existing repository. No codebase, no S5D.

**Reference docs** (read when needed, not upfront):
- [metamodel.md](metamodel.md) — artifact graph, DDD decomposition, validation rules
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts

---

## Scope

**Out of scope:** bugfix <30 LOC, config-only, docs-only. Just do them.

| Tier | Steps | Waivers |
|---|---|---|
| Lightweight | Route → Frame → Spec → Build → Verify → Ship → Learn | Steps can be waived |
| Standard | Full flow | Steps can be waived |
| Decision | Full flow | Steps can be waived |
| High | Full flow | No waivers allowed |

---

## Cross-cutting

**WAL** — see [session-protocol.md](session-protocol.md). WAL saves are local writes (no permission needed). Ship commits require human permission.

**spec:// URI** — `spec://<module>/<document>#<section>`. Use in WAL, commits, REVIEW markers.

**Conflicts** — see [session-protocol.md](session-protocol.md). Priority: Human > Spec > Code > Tests.

**Metamodel** — enforced by `s5d validate` gate. Spec without domains/capabilities/components = validation error.

---

## Execution Surfaces

Every active spec has two mutable surfaces:

- `.s5d/packages/<spec-id>__<date>.s5d.yaml` — authored intent: problem, artifacts, links, contracts, gates.
- `.s5d/records/<spec-id>__<date>.record.yaml` — runtime state: preview diff, approvals, gate results, import fingerprint, decision record, reflection.

`s5d_new` creates both. `s5d_add_hypothesis` / `s5d_add_evidence` edit spec YAML. All other commands update the record file.

---

## Command Contract

### Core Commands

| Action | MCP | CLI | Hard preconditions |
|---|---|---|---|
| Bootstrap workspace | `s5d_init` | `s5d init [--claude] [--all]` | Safe to re-run. |
| Create scaffold | `s5d_new` | `s5d new` | Scaffold only — does not populate problem/artifacts. CLI `--hypothesis-id` auto-links `spec_ref`. |
| Quick note | `s5d_note` | `s5d note` | Shorthand for `s5d new note.<slug> --tier note`. |
| Add hypothesis | `s5d_add_hypothesis` | `s5d add-hypothesis` | Decision tier only. Duplicate IDs rejected. |
| Add evidence | `s5d_add_evidence` | `s5d add-evidence` | Decision tier only. `formality` 1–5 on command surface. |
| Validate | `s5d_validate` | `s5d validate` | Read-only. Must pass before preview. |
| Graph check | `s5d_graph_check` | `s5d graph-check` | Cycles/layering errors block preview. |
| Preview | `s5d_preview` | `s5d preview` | Records `previewed_spec_sha256`. Stale after spec change. |
| Approve | `s5d_approve` | `s5d approve --reviewer <name>` | Must be `previewed`. Binds `spec_sha256` + `diff_sha256`. |
| Run gates | `s5d_run_gates` | `s5d run-gates` | Schema/graph run built-in if no external command. Failed gate blocks import. |
| Waive gate | `s5d_waiver` | MCP only | Gate kind must exist in spec. |
| Import | `s5d_import` | `s5d import [--verified-by] [--force]` | Requires: approved, spec hash match, diff hash match, all gates passed/waived. |
| Decide | `s5d_decide` | `s5d decide --confirmed-by <name>` | Decision tier. Winner must have `spec_ref`. Human confirmation required. |
| Reflect | `s5d_reflect` | `s5d reflect --summary ... --heuristic ...` | Writes to record only (not spec). |
| Route | `s5d_route` | `s5d route` | Classifies into tier + mode + entry point. |

### Recovery Commands

| Situation | Command | Effect |
|---|---|---|
| Check lifecycle state | `s5d_status` / `s5d status` | Lists spec status and sync state. |
| Inspect spec | `s5d_show` / `s5d show` | Spec shape. For approval/gate truth, check `.record.yaml`. |
| Verify applied state | `s5d_drift_check` / `s5d drift-check` | Compares live state to last applied fingerprint. |
| Fix drifted state | `s5d_reconcile` / `s5d reconcile` | Re-imports without re-approval. |
| Undo import | `s5d_rollback` / `s5d rollback` | Tombstones last import for that spec. |

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
s5d validate <feature-spec>
s5d graph-check <feature-spec>
s5d decide <decision-spec> --title "Use server-side rotation" \
  --winner server-side-rotation --confirmed-by Roman \
  --context "Revocation correctness > token statelessness" \
  --decision "Adopt server-side rotation" \
  --rationale "Best revocation/complexity balance" \
  --consequences "Needs persistent token-family store"
s5d preview <feature-spec>
# → WAL: status=AWAITING_HUMAN, pending=approve

# 5. After human approval: approve → build → gates → import → verify
s5d approve <feature-spec> --reviewer Roman
# implement code
s5d run-gates <feature-spec>
s5d import <feature-spec> --verified-by Roman

# 6. Ship (explicit human permission for push/deploy)

# 7. Learn
s5d reflect <feature-spec> --summary "Shipped cleanly" \
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
- "Evaluate/compare" → `prepare` (analyze + frame, stop for human)
- "Implement X" with clear architecture → `execute` (auto-waiver Frame+Decide)
- No signal → `prepare`

Emit routing explicitly:
```
Route: tier=standard, mode=prepare, entry=Frame
Reason: touches auth + payments, needs framing
```

`s5d_init` if no `.s5d/` directory. Proceed to entry point.

---

## Frame

State what's anomalous. Define acceptance BEFORE options.

`s5d_new` (tier: decision) creates skeleton. Fill problem card and add hypotheses/evidence via YAML or `s5d_add_hypothesis`/`s5d_add_evidence`.

---

## Decide

>=3 hypotheses, different in kind. For each: predictions, decomposition, F-G-R, WLNK.

**Challenge probes before `s5d_decide`:**
- Lightweight: 1 probe — strongest counter-argument
- Standard/High: 5 probes — counter-argument, tail failure, evidence weakness, WLNK, SoTA check

If a probe reveals a fatal flaw, stop — revisit hypotheses.

**Human confirms (non-waivable).** WAL: `status=AWAITING_HUMAN`. If winner needs implementation, create linked feature spec first (`s5d new <id> --hypothesis-id <winner>`).

---

## Spec

Problem → acceptance scenarios (>=3 GWT) → implementation hypotheses (>=2) → winner → DO/DON'T.

`s5d_new` creates scaffold. Write YAML with metamodel artifacts. `s5d_validate` + `s5d_graph_check`.

---

## Build

`s5d_preview` → `s5d_approve` (**human name required, non-waivable**).

WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop.

After approval: `s5d_run_gates` → implement. Local commits allowed. REVIEW markers for non-obvious decisions.

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
Non-waivable: Decide human confirmation, Build approve.
Route-to-Spec is an auto-waiver for Frame and Decide. Record it explicitly.
