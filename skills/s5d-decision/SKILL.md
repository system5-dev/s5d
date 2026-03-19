---
name: s5d-decision
description: "S5D decision path — structured architecture decisions with evidence, hypotheses, and human-in-the-loop approval. Steps 1-6 of the S5D process."
argument-hint: "[architecture question or problem to decide]"
---

> **DEPRECATED.** This skill is superseded by the unified `/s5d` flow. Use `/s5d` instead.


# S5D Decision

Requires: `/fpf` for reasoning vocabulary (WLNK, F-G-R, ADI, CL).

Structured path for architecture decisions on existing codebases. Produces a Durable Rationale Record (DRR) with framed problem, hypotheses, evidence, and human-confirmed outcome.

---

## When to invoke

- Software architecture decision on an existing codebase
- Choice is hard to reverse or has high blast radius
- Multiple stakeholders or bounded contexts are affected
- Acceptance criteria are unclear and must be designed
- You need a durable rationale record (DRR)

Do NOT invoke for local edits with explicit acceptance and blast radius of one file.

---

## WAIVER protocol

The only legitimate way to not execute a step. No step may be silently skipped.

```
WAIVER: [step or gate name]
Reason: [why this step cannot be executed as specified]
Condition to lift: [what would make this step required and executable]
Approved by: [name — or "self" if solo author. Self-issued waivers are valid but must be stated explicitly.]
```

A WAIVER is a conscious, documented decision — not a shortcut. Record it in the CLI:
```
s5d waiver <spec> --gate <name> --reason "..." --condition "..." --approved-by "<name>"
```

Without a WAIVER record in the spec's `.record.yaml`, a skipped step is an **incomplete process**, not an exception.

**Waiver lifecycle:** There is currently no CLI command to revoke/lift a waiver once its condition is met. Waivers are append-only in the record. To "lift" a waiver, configure the gate command and re-run `run-gates` — the new `passed` result supersedes the waiver.

**Gate state machine — complete unambiguous path:**

| Gate has command? | Waiver recorded? | `run-gates` result | `import` accepts? |
|---|---|---|---|
| Yes | — | `passed` (exit 0) or `failed` | `passed` only |
| Yes | — | `timeout` | No — fix or waiver |
| No | No | `failed` (exit 1) | No — blocked |
| No | Yes (before run-gates) | `waived` | Yes |
| No | Yes (after run-gates) | re-run → `waived` | Yes after re-run |

**Rule:** Record waiver BEFORE `run-gates` to avoid re-run. `run-gates` is idempotent — safe to re-run after adding a waiver. `import` accepts `passed` or `waived`. Anything else blocks.

**Implementation note:** Currently `import` checks that each declared gate's latest result has `status == "passed"`. Gates with no configured command get `status: "skipped"` which does NOT satisfy `import`. To proceed: either configure a command for the gate, or remove the gate from spec.gates if it doesn't apply. Waiver-aware gate status (`waived`) is a planned feature — until then, removing the gate kind from the spec is the practical workaround.

**Non-waivable gates:** The following gates cannot be waived under any circumstances:
- `decide --confirmed-by` — human confirmation of decision
- `approve --reviewer` — human approval of feature spec

These gates exist to guarantee human agency over irreversible decisions. No WAIVER, no `--force`, no workaround.

---

## The path

Every step defines: **INPUT** (what you receive), **OUTPUT** (typed schema you must produce), **VALIDATE** (checks before proceeding), **CLI** (commands to run), **PRESENT** (structured block the agent MUST display to the user). Output of step N = input of step N+1.

**Presentation rule:** After completing each step, the agent MUST display the PRESENT block to the user before proceeding. No silent steps — every step produces a visible, structured artifact. The PRESENT block is not optional; skipping it = skipping the step.

### 1. Preflight

**Precondition:** the task is anchored in an existing repository and codebase. No codebase → outside S5D scope.

**INPUT:** User's question, feature request, or problem description.

**OUTPUT:**
```json
{
  "lifecycle_stage": "Explore | Shape | Evidence | Operate",
  "target_system": "what must work in operation",
  "creator_system": "who builds/changes it",
  "target_architecture": "desired steady-state design",
  "delivery_strategy": "migration sequence to reach target architecture",
  "roles": ["analyst", "implementer", "reviewer", "operator"],
  "object": "what decision or feature this is about",
  "owner": "whose problem"
}
```

**VALIDATE:** No field is empty or "TBD". If any field is fuzzy → frame first, do not proceed.

**CLI (MANDATORY):**
1. Check if `.mcp.json` exists BEFORE running init.
2. `s5d init` — ensure `.s5d/` exists + registers `.mcp.json`. Idempotent.
3. **If `.mcp.json` did NOT exist before init → STOP.** Tell user to restart agent session, then re-run. MCP tools won't be available until restart. Do NOT proceed.
4. `s5d analyze . --product <product> -o .s5d/packages/analysis__<date>.s5d.yaml` — scan codebase. Skip ONLY if `.s5d/packages/` already has a recent analysis spec.
5. `s5d new decision.<id> --tier decision --question "<object>" --product <product>` — create the decision spec.

Agent MUST run `init` and `analyze` before `new`. Do not skip.

**PRESENT:**
```
┌─ S5D | PREFLIGHT | 1/6 ─────────────────────────────────────┐
│                                                              │
│  CLI results:                                                │
│    init:    ✓ .s5d/ ready                                    │
│    analyze: ✓ <n> domains / <n> entities / <n> components    │
│    new:     ✓ decision.<id> created                          │
│                                                              │
│  Object:       <what decision or feature this is about>      │
│  Owner:        <whose problem>                               │
│  Stage:        <Explore | Shape | Evidence | Operate>        │
│                                                              │
│  Target system:       <what must work in operation>          │
│  Creator system:      <who builds/changes it>                │
│  Target architecture: <desired steady-state design>          │
│  Delivery strategy:   <migration sequence>                   │
│                                                              │
│  Roles: <analyst, implementer, reviewer, operator>           │
│                                                              │
│  Codebase snapshot (from analyze):                           │
│    <key findings — domains, entities, edges discovered>      │
│                                                              │
│  Explanation:                                                │
│  <1-3 sentences: why this decision is needed, what           │
│   triggered it, what's at stake>                             │
│                                                              │
│  → Next: Frame — define acceptance criteria                  │
└──────────────────────────────────────────────────────────────┘
```

---

### 2. Frame (Abduction)

State what's anomalous. What doesn't fit. What needs deciding.

**INPUT:** Preflight output.

**OUTPUT:**
```json
{
  "signal": "What is anomalous — the triggering observation",
  "acceptance": {
    "optimization_targets": ["target_1", "target_2"],
    "hard_constraints": ["must hold — binary pass/fail"],
    "resource_limits": [],
    "observation_indicators": ["monitor but don't optimize"],
    "characteristic_space": { "dimension": "how measured" },
    "selection_policy": "declared BEFORE seeing results"
  }
}
```

**VALIDATE:**
- ≤3 optimization targets
- Hard constraints are binary (pass/fail), not fuzzy
- Resource limits listed ONLY if user explicitly stated them as axes
- Selection policy declared before hypotheses exist

**CLI:** None — Frame is recorded as spec `context` field. Acceptance criteria feed Steps 3–5.

**Why:** Define acceptance criteria BEFORE generating options. Separate target architecture from delivery strategy. Fix the role set during framing.

**PRESENT:**
```
┌─ S5D | FRAME | 2/6 ─────────────────────────────────────────┐
│                                                              │
│  Signal: <what is anomalous — the triggering observation>    │
│                                                              │
│  Acceptance criteria:                                        │
│    Optimize:    <target_1>, <target_2>                       │
│    Hard:        <binary pass/fail constraints>               │
│    Limits:      <resource limits, if any>                    │
│    Observe:     <monitor but don't optimize>                 │
│                                                              │
│  Characteristic space:                                       │
│    <dimension_1>: <how measured>                             │
│    <dimension_2>: <how measured>                             │
│                                                              │
│  Selection policy: <declared BEFORE seeing results>          │
│                                                              │
│  Explanation:                                                │
│  <Why these targets matter. What trade-offs are inherent.    │
│   Why the selection policy is appropriate.>                  │
│                                                              │
│  → Next: Hypothesize — generate ≥3 distinct approaches      │
└──────────────────────────────────────────────────────────────┘
```

---

### 3. Hypothesize (Abduction)

**INPUT:** Frame output (acceptance criteria, constraints, targets).

**OUTPUT:**
```json
{
  "hypotheses": [
    {
      "title": "Descriptive name — genuinely different approach",
      "content": "Why plausible — mechanisms, not opinions",
      "scope": "What part of the system this covers",
      "kind": "system"
    }
  ]
}
```

**VALIDATE:**
- ≥3 items, different in KIND (not degree)
- Three database configs = one hypothesis with parameters, NOT three hypotheses
- Three fundamentally different storage strategies = three hypotheses
- `kind`: `system` (how to build) or `episteme` (how to understand)
- No two hypotheses share >50% of the same components

**CLI (MANDATORY):** For each item → `s5d add-hypothesis <spec> --title "..." --content "..." --scope "..." --kind "..."`

Agent MUST run this CLI command for every hypothesis. Do not just display — record in the spec.

**PRESENT:**
```
┌─ S5D | HYPOTHESIZE | 3/6 ────────────────────────────────────┐
│                                                               │
│  H1: <title>                                                  │
│    Why plausible: <mechanisms, not opinions>                   │
│    Scope: <what part of the system>                           │
│    Kind: <system | episteme>                                  │
│                                                               │
│  H2: <title>                                                  │
│    Why plausible: <mechanisms>                                │
│    Scope: <scope>                                             │
│    Kind: <kind>                                               │
│                                                               │
│  H3: <title>                                                  │
│    Why plausible: <mechanisms>                                │
│    Scope: <scope>                                             │
│    Kind: <kind>                                               │
│                                                               │
│  Explanation:                                                 │
│  <How these hypotheses differ in KIND (not degree).           │
│   Why each is a genuinely distinct approach.>                 │
│                                                               │
│  CLI: ✓ add-hypothesis called for each                       │
│  → Next: Evidence — predictions + decomposition               │
└───────────────────────────────────────────────────────────────┘
```

**Pruning rule:** A hypothesis may be eliminated ONLY after Step 4a reveals a prediction violating a hard constraint, OR after Step 4b evidence returns unanimous contra-verdict. Pruning before evidence = unrecorded assumption → record a WAIVER.

Parity: same depth of analysis for each surviving hypothesis.

---

### 4. Evidence

#### 4a. Deduction — predictions before gathering

**INPUT:** Hypotheses from Step 3.

**OUTPUT:** Per surviving hypothesis:
```json
{
  "hypothesis_id": "h1-name",
  "predictions": {
    "must_observe": ["What MUST be observable if this hypothesis is true"],
    "must_not_observe": ["What MUST NOT be observable if this hypothesis is true"]
  }
}
```

**VALIDATE:**
- Every surviving hypothesis has predictions
- Predictions are falsifiable (can be checked against codebase or evidence)
- No induction without prior deduction — predictions are test targets, not post-hoc rationalization

**CLI (MANDATORY):** For each hypothesis → `s5d add-evidence <spec> --hypothesis-id <id> --evidence-type prediction --content "<predictions JSON>" --verdict pass|fail`

Agent MUST run this CLI command for every hypothesis prediction set. Do not skip.

**PRESENT:**
```
┌─ S5D | EVIDENCE 4a: PREDICTIONS | 4/6 ──────────────────────┐
│                                                               │
│  H1: <title>                                                  │
│    Must observe:     <falsifiable predictions>                │
│    Must NOT observe: <falsifiable anti-predictions>           │
│                                                               │
│  H2: <title>                                                  │
│    Must observe:     <predictions>                            │
│    Must NOT observe: <anti-predictions>                       │
│                                                               │
│  H3: <title>                                                  │
│    Must observe:     <predictions>                            │
│    Must NOT observe: <anti-predictions>                       │
│                                                               │
│  Explanation:                                                 │
│  <What makes these predictions falsifiable. Which predictions │
│   directly test the acceptance criteria from Frame.>          │
│                                                               │
│  CLI: ✓ add-evidence --evidence-type prediction called        │
│  → Next: Decomposition — metamodel analysis per hypothesis    │
└───────────────────────────────────────────────────────────────┘
```

---

#### 4b. Deduction — Metamodel decomposition

**INPUT:** Hypotheses + predictions from Steps 3–4a.

**OUTPUT:** Per surviving hypothesis:
```json
{
  "hypothesis_id": "h1-name",
  "decomposition": {
    "domains": [
      { "id": "payment", "classification": "core", "name": "Payment Processing" }
    ],
    "capabilities": [
      { "id": "authorize-payment", "domain": "payment", "name": "AuthorizePayment" }
    ],
    "entities": [
      { "id": "transaction", "domain": "payment", "name": "Transaction" }
    ],
    "use_cases": [
      { "id": "uc-pay", "name": "Process payment", "feature": "feat.x" }
    ],
    "components": [
      { "id": "payment-service", "domain": "payment", "name": "payment_service.rs", "paths": ["src/payment/service.rs"] }
    ],
    "containers": [
      { "id": "api-server", "name": "API Server", "system": "main" }
    ],
    "edges": [
      { "from": "payment", "to": "identity", "archetype": "customer_supplier", "transport_ref": "rest" }
    ],
    "transports": [
      { "id": "rest", "kind": "rest", "serialization": "json" }
    ]
  },
  "verification": {
    "scenarios": [
      { "title": "Happy path", "given": "...", "when": "...", "then": "..." }
    ],
    "invariants": ["Property that must ALWAYS hold"],
    "boundaries": ["OUT_OF_SCOPE: what is explicitly excluded"]
  },
  "blast_radius": {
    "domain_count": 3,
    "new_components": 5,
    "affected_services": 2,
    "cross_domain_edges": 2
  },
  "f_g_r": {
    "formality": 2,
    "claim_scope": ["latency", "consistency"],
    "reliability": 0.8
  }
}
```

**VALIDATE:**
- `classification`: `core` | `supporting` | `generic` only
- `archetype`: `ohs` | `customer_supplier` | `acl` | `conformist` | `shared_kernel` | `published_language` | `partnership` | `separate_ways`
- Capability names: VerbNoun format (AuthorizePayment, TrackShipment)
- No cycles in edges (lower classification must not depend on higher)
- `formality`: 1–5, `reliability`: 0.0–1.0
- Scenarios have all Given/When/Then fields with concrete data, not prose
- Parity: ALL surviving hypotheses decomposed to same depth

**CLI (MANDATORY):** Serialize decomposition as `--content`, call `s5d add-evidence <spec> --hypothesis-id <id> --evidence-type decomposition --content "<decomposition JSON>" --verdict pass`

Agent MUST run this CLI command for every hypothesis. Do not skip.

**Why decomposition:** It is structured deduction, not opinion. "A touches 3 domains, adds 2 cross-domain edges, requires new gRPC transport" is a derived consequence, not a feeling.

**PRESENT:**
```
┌─ S5D | EVIDENCE 4b: DECOMPOSITION | 4/6 ────────────────────┐
│                                                               │
│  H1: <title>                                                  │
│    Domains:    <count> (<core/supporting/generic breakdown>)  │
│    Components: <count> (new: <n>, modified: <n>)             │
│    Edges:      <cross-domain edge count + key archetypes>    │
│    Blast radius: <domain_count>d / <components>c / <edges>e  │
│    F-G-R: F=<1-5> G={<scope>} R=<0-1>                       │
│                                                               │
│  H2: <title>                                                  │
│    Domains:    ...                                            │
│    Components: ...                                            │
│    Edges:      ...                                            │
│    Blast radius: ...                                          │
│    F-G-R: ...                                                 │
│                                                               │
│  H3: <title>                                                  │
│    Domains:    ...                                            │
│    Components: ...                                            │
│    Edges:      ...                                            │
│    Blast radius: ...                                          │
│    F-G-R: ...                                                 │
│                                                               │
│  Explanation:                                                 │
│  <Key structural differences between hypotheses.              │
│   Which has smallest blast radius. Which introduces           │
│   most cross-domain coupling and why.>                        │
│                                                               │
│  CLI: ✓ add-evidence --evidence-type decomposition called     │
│  → Next: Audit — compare hypotheses on acceptance axes        │
└───────────────────────────────────────────────────────────────┘
```

---

### 5. Audit (Induction)

**INPUT:** Decompositions + F-G-R + predictions from Steps 4a–4b.

**OUTPUT:**
```json
{
  "acceptance_axes": ["consistency", "latency_p99", "operational_complexity"],
  "comparison": [
    {
      "hypothesis_id": "h1-name",
      "scores": {
        "consistency": "ACID guarantees — full serializable isolation",
        "latency_p99": "< 50ms reads, < 200ms writes",
        "operational_complexity": "high — requires DBA, migration tooling"
      },
      "wlnk": {
        "component": "operational_complexity",
        "reason": "Requires 24/7 on-call DBAs, single point of failure for schema changes"
      }
    }
  ],
  "cl": {
    "level": 2,
    "justification": "All hypotheses address relational workloads, different consistency models"
  },
  "recommendation": {
    "winner": "h1-name",
    "rationale": "Best WLNK profile — weakest link acceptable given team expertise",
    "stepping_stone": {
      "id": "h2-name",
      "condition": "When write volume exceeds 10K TPS and eventual consistency becomes acceptable"
    }
  }
}
```

**VALIDATE:**
- `acceptance_axes` match axes from Frame (Step 2) — no new axes invented
- Every hypothesis scored on SAME axes (parity)
- `wlnk` required per hypothesis — weakest component identified with reason
- `cl.level`: 0–3 (0=opposed, 1=comparable, 2=translatable, 3=near-identity)
- If CL 0 → resolve frame conflict before selecting
- `stepping_stone.condition`: state-based, NEVER time-based ("in 3 months" is forbidden)
- Scores reference evidence data from Step 4, not gut feel

**CLI:** None — Audit is a presentation step. Evidence is already recorded.

**PRESENT:**
```
┌─ S5D | AUDIT | 5/6 ──────────────────────────────────────────┐
│                                                               │
│  Acceptance axes: <axis_1>, <axis_2>, <axis_3>               │
│                                                               │
│  ┌─────────────┬──────────────┬──────────────┬──────────────┐│
│  │ Axis        │ H1           │ H2           │ H3           ││
│  ├─────────────┼──────────────┼──────────────┼──────────────┤│
│  │ <axis_1>    │ <score>      │ <score>      │ <score>      ││
│  │ <axis_2>    │ <score>      │ <score>      │ <score>      ││
│  │ <axis_3>    │ <score>      │ <score>      │ <score>      ││
│  ├─────────────┼──────────────┼──────────────┼──────────────┤│
│  │ WLNK        │ <weakest>    │ <weakest>    │ <weakest>    ││
│  └─────────────┴──────────────┴──────────────┴──────────────┘│
│                                                               │
│  CL: <0-3> — <justification>                                │
│                                                               │
│  Recommendation: <winner> — <rationale in 1-2 sentences>     │
│  Stepping stone: <non-winner> — when <state-based condition> │
│                                                               │
│  Explanation:                                                 │
│  <Why the winner has the best WLNK profile. What trade-offs  │
│   are accepted. Why the stepping stone isn't chosen now       │
│   but could be right under different conditions.>             │
│                                                               │
│  → Next: Decide — present to human for confirmation          │
│  ⚠ STOP: Agent must NOT proceed without human confirmation   │
└───────────────────────────────────────────────────────────────┘
```

**F-G-R** (required per hypothesis):
- **F** (Formality): 1=opinion, 2=structured logic, 3=prototype, 4=experiment, 5=peer-reviewed. Min across chain.
- **G** (ClaimScope): set-valued — what's in scope, what's out. Not a score.
- **R** (Reliability): [0,1]. Min across chain. 0.5=coin flip, 0.7=confident, 0.9=near-certain.

**CL** (required before comparing):
- CL 0 = Opposed (contradictory frames — comparison invalid without bridging)
- CL 1 = Comparable (same domain, different frameworks)
- CL 2 = Translatable (systematic mapping exists)
- CL 3 = Near-identity (same framework, minor differences)

Evaluate target-state architecture and rollout strategy separately. Do not downgrade a target architecture just because migration is non-trivial unless migration risk is an explicit acceptance axis.

---

### 6. Decide (Human in the loop — mandatory, NON-WAIVABLE)

**INPUT:** Audit comparison from Step 5.

**OUTPUT:** Present recommendation to user. **STOP.** Do NOT call CLI until user confirms.

After human confirmation:
```json
{
  "title": "Decision title",
  "winner_id": "h1-name",
  "rejected_ids": ["h2-name", "h3-name"],
  "context": "Why this question arose — triggering anomaly",
  "decision": "What was decided — exact form",
  "rationale": "Comparison referencing evidence, WLNK, trade-off axes",
  "consequences": "Benefits, trade-offs, risks, impacted areas",
  "confirmed_by": "human name"
}
```

**VALIDATE:**
- `confirmed_by` is filled by the HUMAN, not the agent (non-waivable)
- `rationale` references evidence data from Step 4, not new arguments
- `consequences` names impacted areas explicitly

**CLI (MANDATORY — after human confirmation ONLY):** `s5d decide <spec> --title "..." --winner <id> --confirmed-by "<name>" --context "..." --decision "..." --rationale "..." --consequences "..." --rejected "<ids>"`

Agent MUST run this CLI command after and only after the user confirms. Do not skip.

**Agent instruction:** STOP execution here. Present your recommendation to the user. Do NOT run `s5d decide` until the user explicitly confirms in conversation. Proceeding without human input is a process violation — this gate is non-waivable.

**Stepping stone:** name 1 non-winning hypothesis — why not chosen now AND under what state-based condition it becomes the right call.

**PRESENT (before human confirmation):**
```
┌─ S5D | DECIDE | 6/6 ─────────────────────────────────────────┐
│                                                               │
│  ⚠ AWAITING HUMAN CONFIRMATION — do not proceed without it   │
│                                                               │
│  Recommendation: <winner title>                               │
│                                                               │
│  DRR (Durable Rationale Record):                             │
│    Problem:      <why this question arose>                    │
│    Decision:     <what is decided — exact form>               │
│    Rationale:    <comparison referencing evidence + WLNK>     │
│    Consequences: <benefits, trade-offs, risks, impacted areas>│
│                                                               │
│  Rejected: <H2 title>, <H3 title>                            │
│  Stepping stone: <non-winner> —                              │
│    condition: <state-based condition to reconsider>           │
│                                                               │
│  Waivers: <list or "none">                                   │
│                                                               │
│  Confirm? (name for --confirmed-by)                          │
└───────────────────────────────────────────────────────────────┘
```

**PRESENT (after human confirmation):**
```
┌─ S5D | DECIDED ✓ ────────────────────────────────────────────┐
│                                                               │
│  Decision: <title>                                           │
│  Winner:   <hypothesis title>                                │
│  Confirmed by: <human name>                                  │
│  CLI: ✓ s5d decide executed                                  │
│                                                               │
│  → Next steps: /s5d-feature to implement the winning approach│
└───────────────────────────────────────────────────────────────┘
```

---

## Status line

Shown at every step transition. Not machine-enforced.

```
S5D | [STEP NAME] | [step number]/7
Object: [what is being decided]
Need: [what is required to advance]
```

---

## Output template

```
S5D DECISION: <title>
Stage: <Explore | Shape | Evidence | Operate>
Question: <framed question>
Acceptance: <optimization targets + hard constraints>
Hypotheses: H1 <name> | H2 <name> | H3 <name>
Roles: <role map for analysis, implementation, verification, review, rollout, operation>
Evidence:
  H1: <domain count, components, blast radius, key data> | F-G-R: <assessments>
  H2: <domain count, components, blast radius, key data> | F-G-R: <assessments>
  H3: <domain count, components, blast radius, key data> | F-G-R: <assessments>
CL: <commensurability level + justification>
Weakest link: <which hypothesis has the worst weak point>
DRR:
  Problem frame: <why this question arose>
  Decision: <winning hypothesis + exact form>
  Rationale: <comparison referencing evidence, WLNK, trade-off axes>
  Consequences: <benefits, trade-offs, risks, impacted areas>
Stepping stone: <non-winning hypothesis — state-based condition to become right call>
Waivers: <list any waivers recorded, or "none">
```

---

## CLI Enforcement

**Every step with a CLI (MANDATORY) marker MUST execute the command before moving to the next step.** The agent must not:
- Display the PRESENT block without having run the CLI command first
- Claim a step is done without CLI output confirming execution
- Batch multiple steps' CLI calls — execute each step's CLI, verify output, PRESENT, then proceed

**Verification:** After each CLI call, check the exit code. If non-zero — diagnose and fix before proceeding. Do not WAIVER a CLI failure; fix the input and retry.

**Enforcement checklist per step:**
| Step | CLI required? | Tool call |
|------|--------------|-----------|
| 1. Preflight | Yes | `s5d init` → `s5d analyze .` → `s5d new decision.<id>` |
| 2. Frame | No | — (recorded in context) |
| 3. Hypothesize | Yes | `s5d add-hypothesis` × N |
| 4a. Predictions | Yes | `s5d add-evidence --evidence-type prediction` × N |
| 4b. Decomposition | Yes | `s5d add-evidence --evidence-type decomposition` × N |
| 5. Audit | No | — (presentation) |
| 6. Decide | Yes (after human confirm) | `s5d decide` |

---

## Autonomy — Compute Before Asking

If a decision is **derivable from evidence**, compute it — don't ask. Human-in-the-loop ONLY for:
- `decide --confirmed-by` (non-waivable)
- Trade-offs reflecting values, not facts
- Genuinely ambiguous choices with no data to break the tie

Computable: domain classification, edge archetype, maturity level, WLNK (min), F-G-R scoring, product name (from repo), gate config (from tier).

**Default to reasonable choices. Infer. Derive. Ask only when genuinely stuck.**

---

## Do NOT

- Skip any step without a recorded WAIVER
- Route to "easy mode" based on decision type — one path for all decisions
- Compare without acceptance criteria defined first
- Claim "done" without a named output artifact
- Skip decomposition — it is the structured deduction that informs evidence
- Use time-based stepping stone conditions ("in 3 months") — state-based only
- Smuggle resource discomfort into architecture choice when time/headcount/budget were not declared as acceptance axes
- Confuse target architecture with delivery strategy; phased rollout is not a competing architecture by itself
- Leave roles implicit; if the role map is undefined, staffing and ownership will drift during execution
- Solve before framing — the bottleneck is problem quality, not solution speed
- Compare before characterizing — define the characteristic space first

---

## CLI reference — Decision path

```
s5d new decision.<id> --tier decision --question "<q>" --product <p>
s5d add-hypothesis <spec> --title "H1" --content "<why>"
s5d add-evidence <spec> --hypothesis-id h1 --evidence-type decomposition \
  --content "<data>" --verdict PASS|FAIL \
  [--formality 3] [--claim-scope "latency,throughput"] [--reliability 0.8]
s5d decide <spec> --title "<title>" --winner <id> --confirmed-by "<name>" \
  --context "<...>" --decision "<...>" --rationale "<...>" --consequences "<...>" --rejected "<ids>"
s5d waiver <spec> --gate implement --reason "<why>" --condition "<when>" --approved-by "<name>"
s5d show <spec>
s5d render <spec> --view decision
```

---

## Command details

### new (decision tier)
Creates spec + record from template. Requires `--question`. Files: `.s5d/packages/{id}__{YYYYMMDD}.s5d.yaml` + `.s5d/records/{id}__{YYYYMMDD}.record.yaml`.

### add-hypothesis
Adds hypothesis to decision-tier spec. Auto-generates slugified ID from title. Validates uniqueness. Starts at layer L0. Fields: title, content, scope, kind (system/episteme).

### add-evidence
Attaches evidence to hypothesis. Layer progression: L0 + pass → L1, L1 + pass → L2, any + fail → invalid. Supports F-G-R scoring: `--formality` (1-5), `--claim-scope`, `--reliability` (0.0-1.0). Evidence valid for 90 days.

`--evidence-type` is free-form. Common values: `decomposition`, `prediction`, `benchmark`, `prototype`, `research:paper`, `research:docs`, `test`, `external`.

### decide
Records decision in record file (Two-File Model). Requires: tier == Decision, winner hypothesis exists, `--confirmed-by` (non-waivable). Creates DecisionRecord with title, winner_id, rejected_ids, context, decision, rationale, consequences, confirmed_by, timestamp. Warns if winner is L1 (not L2).

### waiver
Records explicit waiver for a gate or step. Fields: gate name, reason, condition to lift, approved-by. Stored in record. Required for any skipped step — without waiver, gate = BLOCK.

### show
Formatted display. Decision specs: problem signal, hypothesis tree with evidence, winner marked, verdicts colored.

---

## Glossary

- **WLNK** — Weakest Link. System quality = min(component qualities).
- **WAIVER** — Explicit documented refusal of a step. Required to proceed without executing the step.
- **F-G-R** — Formality / ClaimScope / Reliability. Confidence assessment for evidence.
- **CL 0-3** — Commensurability level. How comparable two hypotheses are.
- **DRR** — Durable Rationale Record. Problem frame + decision + rationale + consequences.
- **ADI** — Abduction → Deduction → Induction. The reasoning cycle underlying all S5D steps.
- **MONO** — Adding a part cannot worsen the whole; but it adds a new potential weak link.
- **Characteristic space** — dimensions used to compare hypotheses.
- **Target system** — what must work in production after the decision is implemented.
- **Creator system** — who builds/changes/operates it.
