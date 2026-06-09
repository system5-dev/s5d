---
name: ddd-refactor
description: "Assess a codebase against Domain-Driven Design and emit a refactoring plan. Detects DDD modeling smells: anemic domain models, transaction-script controllers, business logic leaking into route handlers, money/identifiers as primitives (no value objects), missing anti-corruption layer at integration seams (payment/storage vendors), ORM called directly from controllers (no repository), aggregate-boundary leaks. Every finding pairs a DDD Fix with a Validate step. Compiled analyzer (s5d binary), read-only, reports scanned-file counts and stack coverage honestly. Use for: 'DDD refactor', 'domain-driven design review', 'anemic domain', 'add anti-corruption layer', 'aggregate boundaries', 'rich domain model', 'DDD рефакторинг', 'выделить домен'."
argument-hint: "[--root PATH]"
---

# ddd-refactor

Domain-Driven Design assessment + refactoring plan. Cluster pattern: `detect → analyze
→ flatten`. Read-only. Every finding pairs a **Fix** (the DDD move) with a **Validate**
(how to prove it landed — a test, a grep that must return empty, a dependency-direction
check).

**Distinct from `domain-refactor`.** `domain-refactor` scans for boundary violations
against `.s5d/discovery/architecture-map.md` (god-components, drift, orphans — is the
code where the map says). `ddd-refactor` assesses **modeling quality** (is the domain
rich or anemic, are there aggregates / value objects / an ACL, or is it a transaction
script). Different lens, different output — run both; they don't overlap.

## Checks (7)

| Check | DDD smell |
|---|---|
| anemic-domain | ORM models are data-only; behavior lives in services/controllers |
| transaction-script | >300-LOC handlers orchestrate DB + external + rules inline |
| domain-logic-in-controllers | money/price/tax math scattered across the HTTP layer |
| value-objects-as-primitives | money as `number`, identifiers as `string` (primitive obsession) |
| missing-acl | a vendor SDK (payment/storage) referenced across many files, no adapter |
| repository-absence | ORM called directly from controllers, no repository per aggregate |
| aggregate-boundary-leak | a handler writes 3+ models with no aggregate root coordinating |

Deterministic checks currently cover **TypeScript/JavaScript** codebases. On other
stacks the analyzer reports `stack-not-covered` explicitly — that is a coverage gap
statement, never a clean verdict.

## Severity model

`high` = the model fights you at scale (anemic core, transaction scripts, money rules
sprayed across controllers, no ACL on a churny vendor). `medium` = a missing tactical
pattern (value objects, repositories, aggregates). The skill **does not gate** — DDD
is a direction, not a pass/fail; it produces a sequenced plan.

## Flow

```
1. s5d skill ddd detect            → JSON: DDD signals (ORM entities, domain layer, seams, value types, events)
2. s5d skill ddd analyze           → JSON: findings [{check,severity,path,detail,fix,validate}] + scanned_files + stacks + status
3. s5d skill ddd analyze --flatten → anomalies-only markdown at a severity floor (default medium)
```

The analyzer is compiled into the `s5d` binary — no shell/jq/python dependencies.

## Determinism boundary

| Step | Compiled (`s5d skill ddd`) | In agent |
|---|---|---|
| DDD signal detection | ✓ `detect` | — |
| Smell findings + severity + fix + validate | ✓ `analyze` | — |
| Anomaly distillation at a severity floor | ✓ `analyze --flatten` | — |
| Choosing aggregate roots & bounded-context lines | — | ✓ |
| Designing the value objects / ACL interface | — | ✓ |
| Confirming a heuristic smell is real | — | ✓ (run the Validate) |

## Hard rules

- **Every finding ships a `validate`.** A DDD move with no way to confirm it landed is
  not emitted. Many validates are "this grep must return 0" (e.g. no `prisma.` in
  controllers) — mechanical and checkable.
- **Heuristic, not truth.** Findings are `[INFERRED]` leads from pattern scans; confirm
  before refactoring. Don't report a smell as `[OBSERVED]` runtime fact.
- **`stack-not-covered` ≠ clean.** A report with zero scanned files is a coverage gap,
  not an endorsement — say so explicitly.
- **Read-only.** The analyzer never modifies the assessed repo.
- **Tests are the safety-net.** Every code-touching DDD move needs coverage first
  (unit-tests / e2e skills) — sequence accordingly; do not refactor a 0%-covered aggregate blind.
- **Stay in lane.** Boundary-vs-map drift → `domain-refactor`. Scaling hotspots →
  `scaling-review`. DDD here is about the shape of the model, not where files live.

## Output layout

```
.claude/skills/ddd-refactor/
├── SKILL.md
├── agents/ddd-refactor-assess.md   # isolated assess agent (calls the s5d binary)
└── references/catalog.md           # DDD pattern references + last_verified
```

Deterministic logic lives in the `s5d` binary (`rust/src/suite/ddd.rs`), not in scripts.

## Worked example

```bash
s5d skill ddd detect                 # DDD signals
s5d skill ddd analyze                # findings JSON (fix + validate per move)
s5d skill ddd analyze --flatten      # anomalies-only markdown (floor: medium)
```

## When NOT to use

- The codebase is a thin CRUD app with no real domain — DDD is overhead; skip it.
- You want boundary alignment to the architecture map → `domain-refactor`.
- You're choosing whether to split a service/repo → that's a decision; run `system-design`.
