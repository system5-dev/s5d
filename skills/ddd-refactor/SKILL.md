---
name: ddd-refactor
description: "Assess a codebase against Domain-Driven Design and emit a refactoring plan — stack-agnostic. Detects DDD modeling smells: anemic domain models, transaction-script controllers, business logic leaking into route handlers, money/identifiers as primitives (no value objects), missing anti-corruption layer at integration seams (carrier/payment/storage), prisma/ORM called directly from controllers (no repository), aggregate-boundary leaks, and ubiquitous-language drift (the same domain modeled twice). Every finding pairs a DDD Fix with a Validate step. Read-only. Use for: 'DDD refactor', 'domain-driven design review', 'anemic domain', 'add anti-corruption layer', 'aggregate boundaries', 'rich domain model', 'DDD рефакторинг', 'выделить домен'."
argument-hint: "[--save | --output PATH]"
---

# ddd-refactor

Domain-Driven Design assessment + refactoring plan. Cluster pattern: `detect → analyze
→ report`. Read-only. Every finding pairs a **Fix** (the DDD move) with a **Validate**
(how to prove it landed — a test, a grep that must return empty, a dependency-direction
check).

**Distinct from `domain-refactor`.** `domain-refactor` scans for boundary violations
against `.s5d/discovery/architecture-map.md` (god-components, drift, orphans — is the
code where the map says). `ddd-refactor` assesses **modeling quality** (is the domain
rich or anemic, are there aggregates / value objects / an ACL, or is it a transaction
script). Different lens, different output — run both; they don't overlap.

## Checks (8)

| Check | DDD smell |
|---|---|
| anemic-domain | ORM models are data-only; behavior lives in services/controllers |
| transaction-script | >300-LOC handlers orchestrate DB + external + rules inline |
| domain-logic-in-controllers | premium/tax/bind/refund math scattered across the HTTP layer |
| value-objects-as-primitives | money as `number`, identifiers as `string` (primitive obsession) |
| missing-acl | a vendor (carrier/payment/storage) referenced across many files, no adapter |
| repository-absence | ORM called directly from controllers, no repository per aggregate |
| aggregate-boundary-leak | a handler writes 3+ models with no aggregate root coordinating |
| ubiquitous-language-drift | the same domain modeled twice (e.g. parallel TS + Go) |

## Severity model

`high` = the model fights you at scale (anemic core, transaction scripts, money rules
sprayed across controllers, no ACL on a churny vendor). `medium` = a missing tactical
pattern (value objects, repositories, aggregates, language drift). The skill **does not
gate** — DDD is a direction, not a pass/fail; it produces a sequenced plan.

## Flow

```
1. scripts/detect.sh   → JSON: DDD signals (ORM entities, domain layer, seams, value types, events)
2. scripts/analyze.sh  → JSON: findings [{check,severity,path,detail,fix,validate}]
3. scripts/report.sh   → markdown DDD refactoring plan (stdout, or --save → test-reports/ddd/report.md)
```

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| DDD signal detection | ✓ `detect.sh` | — |
| Smell findings + severity + fix + validate | ✓ `analyze.sh` | — |
| Markdown plan + sequencing | ✓ `report.sh` | — |
| Choosing aggregate roots & bounded-context lines | — | ✓ |
| Designing the value objects / ACL interface | — | ✓ |
| Confirming a heuristic smell is real | — | ✓ (run the Validate) |

## Hard rules

- **Every finding ships a `validate`.** A DDD move with no way to confirm it landed is
  not emitted. Many validates are "this grep must return 0" (e.g. no `prisma.` in
  controllers) — mechanical and checkable.
- **Heuristic, not truth.** Findings are `[INFERRED]` leads from grep; confirm before
  refactoring. Don't report a smell as `[OBSERVED]` runtime fact.
- **Read-only.** No source changes. `report.sh --save` writes only to `test-reports/ddd/`.
- **Tests are the safety-net.** Every code-touching DDD move needs coverage first
  (unit-tests / e2e skills) — sequence accordingly; do not refactor a 0%-covered aggregate blind.
- **Stay in lane.** Boundary-vs-map drift → `domain-refactor`. Scaling hotspots →
  `scaling-review`. DDD here is about the shape of the model, not where files live.

## Output layout

```
.claude/skills/ddd-refactor/
├── SKILL.md
├── scripts/{detect.sh, analyze.sh, report.sh}   # read-only, JSON / markdown
└── references/catalog.md                          # DDD pattern references + last_verified
```

## Worked example

```bash
bash .claude/skills/ddd-refactor/scripts/detect.sh       # DDD signals
bash .claude/skills/ddd-refactor/scripts/analyze.sh      # findings JSON (fix + validate per move)
bash .claude/skills/ddd-refactor/scripts/report.sh --save   # → test-reports/ddd/report.md
```

## When NOT to use

- The codebase is a thin CRUD app with no real domain — DDD is overhead; skip it.
- You want boundary alignment to the architecture map → `domain-refactor`.
- You're choosing whether to split a service/repo → that's a decision; run `system-design`.
