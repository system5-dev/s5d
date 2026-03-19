# FPF Reference (First Principles Reasoning)

This file is a STABLE REFERENCE. FPF updates independently via RAG — query the RAG for current artifact schemas and edge cases. This file teaches WHEN and HOW to use it.

**If RAG is unavailable** (`fpf_search` returns error), use the **Agent Operating Core** below as standalone fallback. It is self-sufficient for most decisions.

## Access

```
fpf_search MCP tool — query: "<concept>"              # quick lookup
fpf_search MCP tool — query: "<concept>", full: true  # full section
fpf_sync MCP tool                                      # update to latest upstream
```

CLI fallback:
```bash
s5d fpf search "<query>"
s5d fpf search "<query>" --full
s5d fpf sync
```

---

## Agent Operating Core

These are the 4 concepts used every session. Keep them in working memory.

### 1. Frame Before Solve

Bottleneck is problem quality, not solution speed. Do not generate solutions until the problem is framed.

- State the anomaly. Generate ≥3 genuinely distinct hypotheses (different in KIND, not degree).
- Name the trade-off axes explicitly.
- Define acceptance criteria: optimization targets (1–3 max), hard constraints (binary), observation indicators (monitor only — do not optimize, Goodhart's Law).

### 2. WLNK (Weakest Link)

System quality = min(component qualities). Identify the weakest link first. Improving strong components while a weak link persists is waste. Adding a new component adds a new candidate weak link — justify the cost explicitly (MONO).

### 3. F-G-R (Quick Version)

Three dimensions for assessing evidence quality:

- **F (Formality)** — ordinal 1–5: opinion → structured logic → prototype → controlled experiment → formal proof. Take the **minimum** across the full reasoning chain.
- **G (ClaimScope)** — SET-valued, e.g. {latency, consistency}. Never extend a claim beyond its declared scope.
- **R (Reliability)** — continuous [0,1]: 0.5 = coin flip, 0.7 = likely, 0.9 = near-certain, 1.0 = proof. Take the **minimum** across the chain.

**Concrete examples:**

| Scenario | F | G | R |
|----------|---|---|---|
| Bugfix | 3 (tested) | {correctness} | 0.9 (test passes) |
| Refactor | 2 (structured logic) | {maintainability, performance} | 0.7 (reviewed, not benchmarked) |
| Architecture decision | 4 (prototype + benchmark) | {latency, cost, reliability} | 0.8 (measured on staging) |

**F scale vs YAML:** FPF defines F as 1–5. Spec YAML accepts 0–9 for finer granularity. Mapping: FPF 1→YAML 1-2, FPF 2→YAML 3-4, FPF 3→YAML 5-6, FPF 4→YAML 7-8, FPF 5→YAML 9.

**Tooling contract:** `s5d add-evidence --formality` / `s5d_add_evidence.formality` accept **1–5 only**. Raw YAML may store 0–9 for manually authored or imported evidence, but command surfaces reject `6-9`. If you need the finer scale, edit YAML directly and re-validate.

### 4. ADI Cycle

Strict order: Abduction → Deduction → Induction. No shortcuts.

| Phase | What happens | Output |
|-------|-------------|--------|
| **Abduction** | Generate hypotheses, frame problems | Problem cards, candidate hypotheses |
| **Deduction** | Derive falsifiable predictions | Acceptance specs, what MUST / MUST NOT be observed |
| **Induction** | Test predictions, update confidence | Evidence records, corroboration/refutation |

No induction without prior deduction. No deduction without prior abduction. Long reasoning chains without intermediate verification increase hallucination risk — verify with tools at each phase boundary.

---

## Quick Mode (Default)

**Default: Quick. Escalate to Deep only when: hard to reverse, multiple stakeholders, problem framing unclear.**

1. **Frame** — one sentence: what is the problem and what matters?
2. **Variants table** — ≥3 options different in kind, not degree. Columns: option / key trade-offs / weakest link.
3. **Pick** — apply pre-declared selection policy. State which variant wins and why.

Output: 1–2 paragraphs + table. No full ADI cycle required.

### Quick → Deep Escalation

Escalate from Quick Mode to Deep Mode immediately when any of these becomes true:

- The choice is hard to reverse.
- The winner depends on an untested empirical claim.
- Two variants use incompatible frames or values.
- More than one domain or stakeholder group is materially affected.
- The selection policy is still ambiguous after the first variants table.

Do not stay in Quick Mode once the answer depends on evidence you have not yet gathered. That is exactly where agents start improvising.

### Quick Mode Micro-Example

Problem: "Refresh-token rotation is slow and correctness matters more than raw throughput."

| Option | Key trade-offs | Weakest link |
|--------|----------------|--------------|
| Stateless JWT refresh | Lowest DB load, weakest revocation guarantees | hard revocation |
| Server-side token family | Strong revocation, moderate storage cost | token-family table growth |
| Hybrid cache + DB | Better latency, highest implementation complexity | coherence bugs |

Selection policy: correctness > latency > implementation cost.

Pick: server-side token family. It wins on the primary axis and has a visible weakest link you can measure (`table growth`) instead of a hidden one (`silent revocation failure`).

---

## Full Algorithm (Deep Mode)

Use for architectural decisions, ambiguous problems, irreversible choices.

### Step 1 — Frame Before Solving

- State the anomaly. Generate ≥3 genuinely distinct hypotheses.
- Identify trade-off axes. Define acceptance criteria (targets / constraints / indicators).

### Step 2 — Characterize Before Comparing

- Define characteristic space before seeing results.
- Pre-register selection policy BEFORE running experiments. Post-hoc policy is data dredging.
- Ensure parity: identical inputs, scope, budget across all variants.
- Keep comparison multi-dimensional — never collapse to a single score without explicit justification.

### Step 3 — Generate Genuinely Distinct Variants

- ≥3 variants different in KIND not degree.
- Assess each as multi-dimensional quality (NQD). Identify WLNK per variant.
- Preserve 1–2 stepping stones — state-based condition only: "when X reaches Y". Never time-based: "in 3 months" (unfalsifiable).
- Prefer better scaling slope over hand-tuning (BLP) at comparable budgets.

### Step 4 — Select from Pareto Front

- Hold the Pareto front — do not discard non-dominated options prematurely.
- Apply the pre-declared selection policy. Record: non-dominated variants, why chosen wins.
- If two options are non-comparable (CL = 0): STOP. Resolve frame conflict before selecting.

### Step 5 — Test Against Reality (Evidence)

- Predict before testing: expected observation if hypothesis is correct AND if wrong.
- Record evidence: exact commands, outputs, interpretation.
- Assess confidence using F-G-R. Close the loop — evidence corroborates or refutes.
- Evidence record must contain: predicted, checked, observed, confidence change, unknowns, staleness date.

---

## Full Reference

### Lifecycle Stages

| Stage | Activity | ADI phase | Commitment state |
|-------|----------|-----------|-----------------|
| **Explore** | Generate possibilities, brainstorm, question assumptions | Abduction | Everything open, nothing committed |
| **Shape** | Select direction, define architecture, internal consistency | Deduction | Direction committed, alternatives cut |
| **Evidence** | Test against reality, validate claims, measure | Induction | Reality check, empirical data required |
| **Operate** | Deploy, monitor, maintain | Continuous induction | Production evidence, drift monitoring |

Always state which stage the current artifact is in. Don't skip stages. Artifacts progress through all four in order.

### Key Invariants

**MONO (Monotonicity):** Improving a part cannot worsen the whole. BUT: adding a NEW part adds a new potential weak link.

**IDEM (Idempotency):** Evaluating a single element in isolation must return it unchanged. No accidental upgrade or downgrade from the evaluation process.

**COMM (Commutativity):** For independent components, evaluation order doesn't matter. When dependencies exist, evaluate dependencies first.

**LOC (Locality):** For independent components, evaluation location doesn't matter. No hidden global state.

### F-G-R Chain Rule

For sequential reasoning (A supports B supports C):
- F_total = min(F_A, F_B, F_C)
- R_total = min(R_A, R_B, R_C)
- G_total = intersection(G_A, G_B, G_C)

### Commensurability (CL 0–3)

| Level | Name | Definition | Required action |
|-------|------|-----------|----------------|
| **0** | Opposed | Contradictory frames — comparison invalid | Must bridge or reframe before comparing |
| **1** | Comparable | Same domain, different frameworks | Compare with explicit translation |
| **2** | Translatable | Systematic mapping exists | Use bridge matrix |
| **3** | Near-identity | Same framework, minor differences | Direct comparison valid |

### Key Distinctions — Maintain Always

**Object ≠ Description ≠ Specification:** The system (what must work), what we say about it (model, doc), and what is testable (acceptance criteria). A passing spec is not a working system.

**Target system ≠ Creator system:** What must work in operation vs who builds and operates it.

**Plan ≠ Reality:** Treat plans as hypotheses, not facts.

**Role ≠ Capability ≠ Method ≠ WorkPlan ≠ Work:** Resolve before designing.

**Design-time ≠ Run-time:** Evidence exists only at run-time.

**Overloaded words — always resolve:**
- "process" → Role | Capability | Method | WorkPlan | Work
- "service" → promise clause | provider | access point | method | work
- "quality" → which dimension? Measure it, don't label it.

### Anti-Patterns — Stop Immediately If:

- Solving before framing (Steps 3–5 without Step 1)
- Comparing before characterizing (no characteristic space defined)
- Using one KPI as the truth (Goodhart's Law)
- Hiding value choices inside technical language
- Treating a spec as proof of execution (Description ≠ Object)
- Testing without predictions (data dredging)
- Claiming "verified" without recorded evidence
- Using time-based stepping stone conditions instead of state-based
- Long reasoning chains without intermediate verification
- Extending a claim beyond its declared ClaimScope (G)

---

## Search Index (All RAG Queries)

Use `fpf_search` with any of these queries when you need full schemas or edge cases.

| Need | RAG search query |
|------|-----------------|
| Problem framing | `"problem card PROB"` |
| Anomaly recording | `"anomaly record ANOM"` |
| Characterization | `"characterization CHR"` |
| Variant generation | `"NQD variant quality"` |
| Scaling preference | `"BLP scaling slope"` |
| Selection policy | `"selection policy SEL Pareto"` |
| Evidence recording | `"evidence record EVID F-G-R"` |
| Decision record | `"decision record DRR"` |
| Aggregation rules | `"WLNK MONO cutset Gamma"` |
| Commensurability | `"commensurability CL bridge"` |
| Stepping stone condition | `"stepping stone state-based"` |
| Lifecycle stage | `"Explore Shape Evidence Operate"` |
| Key distinctions | `"Object Description Specification"` |
