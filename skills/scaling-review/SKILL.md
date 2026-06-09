---
name: scaling-review
description: "Review a repo's scalability & performance posture. Detects scaling dimensions (ORM/DB, API handlers, cache, queues, realtime, runtime, heavy libs) and reports anti-patterns: N+1 queries, unbounded findMany, heavy/sync work in the request path, missing outbound timeouts, local-fs writes that break statelessness, no cache layer, serverless connection-pool exhaustion. Every finding pairs a FIX with a VALIDATION method (load test / EXPLAIN / query log) to prove the fix holds. Compiled analyzer (s5d binary), read-only, reports scanned-file counts and stack coverage honestly. Use for: 'scaling review', 'performance audit', 'will this scale', 'N+1 check', 'load readiness', 'аудит масштабируемости', 'выдержит ли нагрузку'."
argument-hint: "[--root PATH]"
---

# scaling-review

Scalability & performance **posture** analyzer. Cluster pattern: `detect → analyze →
flatten`. Read-only — it reads code for scaling anti-patterns, it changes nothing.

**The differentiator — change + validation.** Every finding carries two fields: a
`fix` (the change to make) and a `validate` (how to *prove* the fix actually holds
under scale — a load test, an `EXPLAIN ANALYZE`, a query-count assertion). A
recommendation without a way to confirm it landed is not actionable, so the skill
never emits one.

## Dimensions (8)

| Dimension | Anti-patterns checked |
|---|---|
| db | N+1 (await in a loop/map), unbounded `findMany` (no take/cursor), serverless connection-pool exhaustion |
| api | heavy/sync work in the request path (PDF/Puppeteer/image) |
| concurrency | outbound calls with no timeout/AbortController (slow upstream hangs workers) |
| cache | no cache layer despite many read endpoints |
| queue | presence (work that belongs off the hot path) |
| realtime | ws/SSE presence (fan-out scaling) |
| runtime | serverless vs long-running; local-fs writes that break statelessness |
| heavy-libs | puppeteer/sharp/pdf that should run as jobs |

Anti-pattern checks currently cover **TypeScript/JavaScript** codebases (dimension
detection is partially polyglot — Go/Python handlers are counted). On other stacks
the analyzer reports `stack-not-covered` explicitly — that is a coverage gap
statement, never a clean verdict.

## Severity model

`high` = breaks under real load or data growth (N+1, unbounded query, conn-pool
exhaustion, heavy sync work). `medium` = degrades or risks cascading failure
(missing timeout, no cache, local-fs on serverless). `low` = best-practice. The
skill **does not gate** a build — it produces a report; scaling is a judgement call
against expected load.

## Flow

```
1. s5d skill scaling detect            → JSON: which of 8 scaling dimensions apply + evidence
2. s5d skill scaling analyze           → JSON: findings [{dimension,kind,severity,path,detail,fix,validate}] + scanned_files + stacks + status
3. s5d skill scaling analyze --flatten → anomalies-only markdown at a severity floor (default medium)
```

The analyzer is compiled into the `s5d` binary — no shell/jq/python dependencies.

## Determinism boundary

| Step | Compiled (`s5d skill scaling`) | In agent |
|---|---|---|
| Dimension detection (orm/cache/queue/runtime) | ✓ `detect` | — |
| Anti-pattern findings + severity + fix + validate | ✓ `analyze` | — |
| Anomaly distillation at a severity floor | ✓ `analyze --flatten` | — |
| Confirming a heuristic lead is a real bottleneck | — | ✓ (run the `validate` method) |
| Deciding the load target the fix must meet | — | ✓ |
| Writing the load test / benchmark itself | — | ✓ |

## Hard rules

- **Every finding ships a `validate`.** No fix is emitted without a concrete way to
  prove it holds under scale. This is the skill's contract.
- **Heuristic, not runtime truth.** Findings are pattern-scan `[INFERRED]` leads.
  Confirm each via its validation method before treating it as real — do not report
  a heuristic hit as `[OBSERVED]`.
- **`stack-not-covered` ≠ clean.** A report with zero scanned files is a coverage gap,
  not an endorsement — say so explicitly.
- **Read-only.** The analyzer never modifies the assessed repo.
- **No gate.** scaling-review informs; it does not fail builds. Load expectations are the team's call.
- **Stay in lane.** Security of these paths → `security-scan`; deploy/topology → `infra-scan`. A slow query is ours; a SQL-injection in it is not.

## Output layout

```
~/.agents/skills/scaling-review/
├── SKILL.md
├── agents/scaling-review-assess.md   # isolated assess agent (calls the s5d binary)
└── references/catalog.md             # SoTA load/profiling tooling + last_verified
```

Deterministic logic lives in the `s5d` binary (`rust/src/suite/scaling.rs`), not in scripts.

## Worked example

```bash
s5d skill scaling detect                 # which dimensions apply
s5d skill scaling analyze                # findings JSON (fix + validate per item)
s5d skill scaling analyze --flatten      # anomalies-only markdown (floor: medium)
```

## When NOT to use

- You need a real profile of a running system (flame graphs, live p95) → run a
  profiler/load test directly (see `references/catalog.md`); this skill finds the
  candidates to point them at.
- Security or deployment concerns → `security-scan` / `infra-scan`.
- A micro-optimization in one function with no scale dimension → just fix it.
