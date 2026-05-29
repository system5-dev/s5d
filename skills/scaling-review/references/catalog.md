# scaling-review — tooling catalog

**last_verified: 2026-05-29**

scaling-review's detection is self-contained grep heuristics (no install). This catalog
lists the SoTA tools the agent reaches for to **validate** a finding — because every
finding ships a `validate` step, and these are the instruments that execute it.

## Load / throughput validation

| Tool | Use | Install |
|---|---|---|
| **k6** (Grafana) | scriptable load tests, p95/p99, thresholds-as-code (CI-gateable) | `brew install k6` |
| **Artillery** | YAML/JS load tests, good for HTTP + WebSocket | `npm i -g artillery` |
| **autocannon** | fast Node HTTP benchmarking, quick local p95 | `npm i -g autocannon` |
| **vegeta** | constant-rate HTTP load (Go), clean latency histograms | `brew install vegeta` |

Use these to execute the `validate` of `unbounded-query`, `sync-heavy-in-request`,
`no-cache-layer`, `serverless-conn-pool` (drive concurrency past DB max_connections and
watch for "too many clients").

## Database validation

| Tool | Use |
|---|---|
| **EXPLAIN (ANALYZE, BUFFERS)** | confirm index scan vs seq scan after adding `take`/index |
| **Prisma query logging** (`log: ['query']`) | count queries per request — proves N+1 is gone (1-2 queries, not N) |
| **pg_stat_statements** | find the real top-cost queries in a running DB |
| **pgbouncer / Prisma Accelerate / Supabase pooler** | the FIX for `serverless-conn-pool`; validate active connections stay bounded |

## Profiling (when a heuristic lead needs runtime confirmation)

| Tool | Use |
|---|---|
| **clinic.js** (doctor/flame) | Node event-loop lag, flame graphs |
| **0x** | single-command Node flame graph |
| **pprof** (Go) | CPU/heap profiles for the Go backend |
| **Chrome DevTools / Lighthouse** | front-end TTFB/LCP (note: UI perf is out of cluster scope; engineering only) |

## Cache / queue (the FIX side)

- **Redis / Vercel KV / Prisma Accelerate** — hot-read cache; validate hit ratio > 70% + origin QPS drop.
- **BullMQ / SQS / Kafka** — move heavy sync work off the request path; validate request p95 drops to enqueue time + queue depth observable.

## Notes

- These tools VALIDATE; they do not detect. Detection is the skill's job; confirmation
  is the human's, using the `validate` field on each finding.
- Re-run the agentic research behind this list if `last_verified` is > ~6 months stale.
- Front-end/UX performance is intentionally out of scope — this cluster covers
  engineering only (no ux-ui-review).
