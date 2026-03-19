---
name: s5d
description: "S5D — unified SDLC. Bootstrap → Frame → Decide → Spec → Build → Verify → Ship → Learn."
argument-hint: "[question, feature request, or problem description]"
---

# S5D

One flow. Eight steps.

```
Bootstrap → Frame → Decide → Spec → Build → Verify → Ship → Learn
```

Applies only to work grounded in an existing repository. No codebase, no S5D.

**Reference docs** (read when needed, not upfront):
- [fpf.md](fpf.md) — FPF reasoning vocabulary (WLNK, F-G-R, ADI, CL) + RAG search
- [metamodel.md](metamodel.md) — artifact graph, DDD decomposition, validation rules
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts

---

## Scope

**Out of scope (no S5D needed):** bugfix <30 LOC, config-only, docs-only. These are too small for the framework — just do them.

**In scope — pick tier:**

| Situation | Tier | Steps |
|---|---|---|
| Feature touching 1 domain | Lightweight | Bootstrap → Frame → Spec → Build → Verify → Ship → Learn |
| Feature touching 2+ domains | Standard | Full flow |
| Architecture decision | Decision | Full flow |
| Security / payment / auth | High | Full flow, no waivers |

Steps within flow can be skipped only via waiver (see Waiver section). Routing from Bootstrap to Step 3 is an auto-waiver for Steps 1-2.

---

## Cross-cutting

**WAL** — see [session-protocol.md](session-protocol.md) for full WAL rules, format, and recovery. WAL saves are local writes — no human permission needed. Ship commits (Step 6) require explicit human permission.

**spec:// URI** — `spec://<module>/<document>#<section>`. Use in WAL, commits, REVIEW markers. No prose references when URI exists.

**Conflicts / REVIEW markers** — see [session-protocol.md](session-protocol.md). Priority: Human > Spec > Code > Tests.

**Metamodel** — enforced by `s5d validate` gate. See [metamodel.md](metamodel.md). Spec without domains/capabilities/components = validation error.

---

## Execution Surfaces

Every active spec has two mutable surfaces:

- `.s5d/packages/<spec-id>__<date>.s5d.yaml` — authored intent: problem, artifacts, links, contracts, gates.
- `.s5d/records/<spec-id>__<date>.record.yaml` — runtime state: preview diff hash, approvals, gate results, import fingerprint, decision record, verifier, reflection.

`s5d_new` creates both files. `s5d_add_hypothesis` and `s5d_add_evidence` edit the spec YAML. `s5d_preview`, `s5d_approve`, `s5d_run_gates`, `s5d_import`, `s5d_decide`, and `s5d_reflect` update the record file. If approval/decision/gate state looks "missing", inspect the `.record.yaml` file first.

---

## Command Contract

These are the commands that define the flow. Use them as contracts, not as vague hints.

| Action | MCP | CLI | Writes | Hard preconditions / common stop |
|---|---|---|---|---|
| Bootstrap workspace | `s5d_init` | `s5d init` | `.s5d/` directories and project MCP config | Safe to re-run. If `.s5d/` is missing, stop here first. |
| Draft domain map / index | `s5d_analyze`, `codebase_index`, `codebase_update` | `s5d analyze`, `s5d codebase index`, `s5d codebase update` | codebase index + optional YAML draft | `s5d_analyze` drafts architecture; it does **not** create a spec or record unless you save/output it yourself. |
| Create scaffold | `s5d_new` | `s5d new` | spec YAML + matching record file | Scaffold only. It does **not** populate problem/artifacts/hypotheses for you. CLI `--hypothesis-id` auto-links `hypotheses[].spec_ref`; MCP `s5d_new` does not. |
| Add decision options | `s5d_add_hypothesis` | `s5d add-hypothesis` | spec YAML | Decision tier only. Duplicate hypothesis IDs stop the command. |
| Add decision evidence | `s5d_add_evidence` | `s5d add-evidence` | spec YAML | Decision tier only. `formality` is **1–5** on the command surface. Passing `6-9` here is an error even though raw YAML may store finer-grained values. |
| Validate structure | `s5d_validate` | `s5d validate` | read-only | Spec file must exist and parse. Do not preview before validate is green. |
| Validate graph | `s5d_graph_check` | `s5d graph-check` | read-only | Use after structural validation. Cycles/layering errors block preview/approve. |
| Preview import | `s5d_preview` | `s5d preview` | record preview block + `previewed` status | Spec file must exist. Preview hash becomes stale as soon as the spec changes. |
| Record approval | `s5d_approve` | `s5d approve --reviewer <name>` | record approvals + `approved` status | Spec must already be `previewed`. Approval binds both `spec_sha256` and `diff_sha256`. |
| Run gates | `s5d_run_gates` | `s5d run-gates` | record gate results | Gate kinds must already be declared in the spec. Any failed gate blocks import. |
| Waive a gate | `s5d_waiver` | MCP only today | record gate result with `waived` status | Gate kind must already exist in `spec.gates`. This is a **gate waiver**, not `links.edges[].waiver`. |
| Import approved state | `s5d_import` | `s5d import [--verified-by <name>] [--force]` | alias table, ledger, record verifier | Requires approved state, unchanged spec hash, unchanged preview diff hash, and the latest result for every declared gate = `passed` or `waived`. `--force` only overrides methodological checks, never structural ones. |
| Record decision | `s5d_decide` | `s5d decide --confirmed-by <name>` | record decision block | Decision tier only. Needs human confirmation. Winner hypothesis should already have `spec_ref` or the CLI will stop unless forced. |
| Verify trace/health | `trace_build`, `trace_check`, `s5d_check` | `s5d trace build`, `s5d trace check`, `s5d check --threshold <n>` | trace outputs + health snapshot | Run after import. Trace gaps usually mean missing links, annotations, or components. |
| Close lifecycle | `s5d_reflect` | `s5d reflect --summary ... --heuristic ...` | record reflection + `operated` status | Use after real verification/operation evidence exists. |

## Recovery Commands

Use these when the happy path breaks:

| Situation | Command | What it tells you / does |
|---|---|---|
| Need current lifecycle state | `s5d_status` / `s5d status` | Lists spec status and sync state across the project. |
| Need spec details fast | `s5d_show` / `s5d show` | Shows the spec file shape. For approval/decision/gate truth, check the matching `.record.yaml`. |
| Import rejects because state drifted after apply | `s5d_drift_check` / `s5d drift-check` | Compares live state to the last applied fingerprint. |
| Applied state is drifted/degraded | `s5d_reconcile` / `s5d reconcile` | Re-imports desired state for already applied specs without re-approval. |
| Imported wrong state and must tombstone it | `s5d_rollback` / `s5d rollback` | Reverses the last successful import for that spec. |

## Worked Example

End-to-end example for the common "decision first, feature second" path:

1. Bootstrap and create a decision skeleton.

```bash
s5d init
s5d new decision.refresh-rotation --tier decision --product auth \
  --question "How should refresh tokens rotate?"
```

2. Add at least three genuinely different hypotheses and enough evidence to compare them.

```bash
s5d add-hypothesis .s5d/packages/decision.refresh-rotation__20260411.s5d.yaml \
  --title "Server-side rotation with revocation table" \
  --content "Rotate on every refresh and persist token family state" \
  --scope "auth boundary"

s5d add-evidence .s5d/packages/decision.refresh-rotation__20260411.s5d.yaml \
  --hypothesis-id server-side-rotation-with-revocation-table \
  --evidence-type internal \
  --content "Staging test shows revocation lookup stays <5ms at p95" \
  --verdict pass \
  --formality 4 \
  --claim-scope latency,revocation-correctness \
  --reliability 0.8
```

3. Before deciding, create the implementation spec and link it back to the winner hypothesis. The CLI can do this automatically.

```bash
s5d new feat.refresh-rotation --tier standard --product auth \
  --hypothesis-id server-side-rotation-with-revocation-table
```

4. Fill the generated feature YAML: `problem`, `artifacts`, `links`, `contracts` if needed, and `gates`. Then validate.

```bash
s5d validate .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
s5d graph-check .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
```

5. Record the decision, preview the feature diff, and stop for human build approval.

```bash
s5d decide .s5d/packages/decision.refresh-rotation__20260411.s5d.yaml \
  --title "Use server-side refresh rotation" \
  --winner server-side-rotation-with-revocation-table \
  --context "Auth revocation correctness matters more than token statelessness" \
  --decision "Adopt server-side rotation with token family tracking" \
  --rationale "Best balance of revocation guarantees and implementation complexity" \
  --consequences "Needs persistent token-family store and cleanup path" \
  --confirmed-by Roman

s5d preview .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
```

Write WAL before stopping:

```markdown
# WAL — auth
last_commit_sha: <HEAD>
updated_at: 2026-04-11T12:00:00Z
status: AWAITING_HUMAN
pending: approve feat.refresh-rotation__20260411.s5d.yaml
resume_from: Step 4 — Build
resume_when: Roman replies with reviewer name
```

6. After the human approves the previewed diff, approve, implement, run gates, import, and verify.

```bash
s5d approve .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml --reviewer Roman
# implement code here
s5d run-gates .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
s5d import .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml --verified-by Roman
s5d trace build .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
s5d trace check .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml
s5d check .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml --threshold 70
```

7. Ship only with explicit human permission, then close the loop.

```bash
s5d reflect .s5d/packages/feat.refresh-rotation__20260411.s5d.yaml \
  --summary "Rotation shipped cleanly; no revocation regressions in staging" \
  --heuristic "Link winner hypotheses to feature specs before decide so approval is not blocked later"
```

---

## Step 0 — Bootstrap

1. `s5d_init` MCP → `codebase_index`/`codebase_update` → `s5d_analyze`
2. If first session: collapse crates → domains, fill entities/capabilities/components/edges. Create `specs/WAL.md` with initial state.
3. Route:
   - Hard-to-reverse decision → Step 1 (Frame)
   - Architecture already decided → Step 3 (Spec). This IS a waiver: record `WAIVER: Step 1–2 | Reason: architecture decided | Approved: <name>`

---

## Step 1 — Frame

State what's anomalous. Define acceptance BEFORE options.

`s5d_new` MCP (tier: decision) creates the skeleton only. Then fill the problem card (`signal`, targets, hard constraints, selection policy) and add hypotheses/evidence via YAML edits or `s5d_add_hypothesis` / `s5d_add_evidence`.

---

## Step 2 — Decide

≥3 hypotheses, different in KIND. For each: predictions, decomposition, F-G-R, WLNK.
Audit: compare on Frame's axes.

**Human confirms (non-waivable).** Write to WAL: `status=AWAITING_HUMAN, pending=decision approval`. Stop. Resume when human responds.

If the winning hypothesis needs implementation modeling, create the linked feature spec first (CLI: `s5d new <feature-id> --hypothesis-id <winner>`; MCP: set `hypotheses[].spec_ref` manually). `s5d_decide` records the winner, rationale, and consequences in the **record file**, not in the spec YAML.

---

## Step 3 — Spec

Problem statement → acceptance scenarios (≥3 Given/When/Then) → implementation hypotheses (≥2, table: How/Pros/Cons/WLNK) → winner → DO/DON'T.

`s5d_new` MCP creates the scaffold. Then write YAML with metamodel artifacts and run `s5d_validate` + `s5d_graph_check`.

**Failure paths:**
- `s5d_validate` fails → fix spec, re-validate.
- `s5d_graph_check` finds cycles → resolve dependency, re-check.

---

## Step 4 — Build

`s5d_preview` (structural prerequisite) → `s5d_approve` (**human name required, non-waivable, requires preview**).

**Human approve (non-waivable).** Write to WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop. Resume when human responds.

After approval: `s5d_run_gates` → implement code. Local working commits allowed during implementation (no human permission needed). REVIEW markers for non-obvious decisions. `codebase_update` after code changes.

**Failure paths:**
- Approve without preview → structural error, cannot proceed.
- Gates fail → fix or record an approved gate waiver via `s5d_waiver`.
- Approve denied → revise spec, re-preview.

---

## Step 5 — Verify

Tests → `s5d_import` (SHA256 chain) → `trace_build` + `trace_check` → `s5d_check` (health).
Human reviews diff for semantic correctness.

**Failure paths:**
- Tests fail → fix code.
- `s5d_import` rejects (hash mismatch) → re-preview, re-approve.
- Trace coverage < threshold → add annotations or components.

---

## Step 6 — Ship

Push and deploy require explicit human permission. Commits during Build (Step 4) are local working commits — they don't need permission. Ship commits (push to remote) do.

Each action: push → confirm with human. Deploy → confirm with human.

---

## Step 7 — Learn

`s5d_reflect` MCP. Update WAL: collapse completed items. Record reusable heuristics.

---

## Waiver

Only way to skip a step:
```
WAIVER: <step> | Reason: <why> | Condition: <when required again> | Approved: <name>
```
Non-waivable: Step 2 human confirmation, Step 4 approve.

Routing from Bootstrap to Step 3 is an auto-waiver for Steps 1–2. Record it explicitly.
