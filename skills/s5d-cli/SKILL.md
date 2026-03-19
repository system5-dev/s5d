---
name: s5d-cli
description: "S5D direct CLI/manual path for existing codebases. Use the same FPF-strict S5D method without bypassing its mandatory terms or gates."
---

> **DEPRECATED.** This skill is superseded by the unified `/s5d` flow. Use `/s5d` instead.


# /s5d-cli — Direct S5D Operations

For when you need to work with S5D files directly. Most users should use `/s5d` instead.

This is a manual path for the same S5D method, not a weaker variant. WLNK, F-G-R, CL, ADI, human approval, and repository-backed architecture scope still apply.

**Scope:** existing repository and codebase only. No codebase, no S5D.

## Init

Create `.s5d/` in current project:
```bash
mkdir -p .s5d/packages .s5d/records .s5d/.locks
```

## Create Spec

Write a YAML file to `.s5d/packages/<id>__<date>.s5d.yaml` with this structure:

```yaml
s5d: "1.0"
id: <feature-id>
version: "1.0.0"
product: <product-name>
tier: <note|decision|lightweight|standard|high>

meta:
  title: <title>
  authors: [<author>]
  date: "<YYYY-MM-DD>"

context: |
  ## Problem
  <what needs to be solved>

  ## Goal
  <desired outcome>

artifacts:
  features:
    - id: <feature-id>
      product: <product-name>
      name: <feature-name>

gates:
  - kind: schema
```

## Create Record

Write to `.s5d/records/<id>__<date>.record.yaml`:

```yaml
spec_ref: <spec-filename>
status: proposed
status_history:
  - status: proposed
    timestamp: "<ISO-8601>"
gate_results: []
gate_waivers: []
reflection: null
```

## Status Transitions

proposed → previewed → approved → applied → **operated** (closed by production evidence)
                                           ↘ deprecated | removed

## Decision Spec

For decisions, add to the spec:

```yaml
tier: decision
problem:
  question: "<the question>"

hypotheses:
  - id: h1
    title: "<hypothesis>"
    content: "<why plausible>"
    layer: L0

decision: null  # filled by decide
```

## Add Evidence

Add to hypotheses in the spec:

```yaml
hypotheses:
  - id: h1
    evidence:
      - type: benchmark
        content: "10K rows, <5ms query"
        verdict: PASS
```

## Decide

Write decision to the RECORD file (not spec — spec is immutable):

```yaml
decision:
  title: "<what was decided>"
  winner: h1
  rationale: "<why>"
  timestamp: "<ISO-8601>"
```

## Gate Waivers

A gate without a configured command **must** have a waiver — SKIP is not valid. Record in the record file:

```yaml
gate_waivers:
  - gate: schema
    reason: "schema gate command is not configured yet"
    condition: "when gate_commands.schema is configured in s5d.yaml"
    recorded_at: "<ISO-8601>"
```

Via diana CLI:
```bash
s5d waiver <spec> --gate schema --reason "..." --condition "..."
```

Gate statuses after `run-gates`:
- `passed` — command ran and exited 0
- `failed` — command ran and failed, OR no command AND no waiver
- `waived` — no command, but waiver recorded
- `timeout` — command exceeded timeout

`import` accepts `passed` or `waived`. Anything else blocks.

## Typed output protocol

At each decision phase, produce typed JSON FIRST, validate against constraints, THEN write YAML or call CLI.

**Phase 3 (Hypothesize)** — ≥3 hypotheses as JSON array before writing to spec:
```json
{ "hypotheses": [{ "title": "...", "content": "...", "scope": "...", "kind": "system|episteme" }] }
```

**Phase 4b (Evidence)** — Metamodel decomposition per hypothesis:
```json
{ "hypothesis_id": "h1", "decomposition": { "domains": [...], "capabilities": [...], "edges": [...] },
  "blast_radius": { "domain_count": 3, "new_components": 5 },
  "f_g_r": { "formality": 2, "claim_scope": ["..."], "reliability": 0.8 } }
```

**Phase 5 (Audit)** — comparison matrix before recommendation:
```json
{ "comparison": [{ "hypothesis_id": "h1", "scores": {...}, "wlnk": { "component": "...", "reason": "..." } }],
  "cl": { "level": 2, "justification": "..." },
  "recommendation": { "winner": "h1", "stepping_stone": { "id": "h2", "condition": "..." } } }
```

**Phase 7 (Spec)** — feature spec JSON before writing YAML. Validate: `spec.id == features[0].id`, no edge cycles, valid archetypes.

---

## Reflect (OPERATE stage — closes lifecycle)

```bash
s5d reflect <spec> --summary "<what happened in production>" \
  --worked "thing1,thing2" \
  --issues "issue1" \
  --follow-ups "task1" \
  --evidence "https://grafana/dashboard" \
  --evidence "error rate < 0.1% over 7 days"
```

`--evidence` is repeatable. Each value is a URL, file path, or plain metric description.

After reflect: `record.status` → `operated`. For decision specs: each evidence item is written back into the winning hypothesis as `HypothesisEvidence` (type: external, verdict: pass, claim_scope: [operate]).

Raw record YAML structure:
```yaml
reflection:
  summary: "<what happened>"
  worked: ["<thing1>", "<thing2>"]
  issues: ["<issue1>"]
  follow_ups: ["<task1>"]
  evidence:
    - path: "https://grafana/dashboard"
status: operated
status_history:
  - status: operated
    timestamp: "<ISO-8601>"
```
