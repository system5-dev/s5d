# system-design — SoTA-per-commodity snapshot

**Last verified:** 2026-05-28
**Re-verify if:** > 3 months elapsed, major release announcement, or independent
benchmarks claim a paradigm shift.

This is the human-facing summary that the decision YAMLs operationalise.
Each section: what's the boring default in 2026, what's gaining ground,
what's losing ground, when self-hosted beats managed.

---

## Self-hosted vs managed — universal heuristic

| Signal | Lean self-hosted | Lean managed |
|---|---|---|
| Ops FTE on this component | ≥ 0.5 | < 0.2 |
| Lock-in tolerance | zero | medium+ |
| Compliance (data must stay in-house) | yes | no |
| Budget / scale (TCO crossover) | high | low/medium |
| Time-to-prod budget | weeks-months | hours-days |
| Team familiarity with the tech | proficient/expert | greenfield |

When in the grey zone, **prefer wire-protocol-portable managed** services
(Postgres protocol, S3 API, OpenTelemetry OTLP) so the self-host escape
hatch stays cheap.

---

## Queue / event bus

**Boring default 2026: AWS SQS** (if you're on AWS) or **NATS JetStream**
(if you're not). RabbitMQ remains a fine pick when you need rich routing.

**Gaining:** **Inngest** for Next.js / serverless stacks — durable workflow
runner with event-trigger DX. NATS JetStream for edge + multi-region.

**Losing:** Single-broker RabbitMQ at scale (quorum queues mandatory now).
SQS FIFO if you actually need streaming throughput (use Kafka).

**Kafka is overkill** unless you need a log, not just a queue. KRaft mode
(Kafka 3.5+) removed ZooKeeper, but operating Kafka is still a team's job.

**Temporal** is its own category — durable workflows, not a queue. Pick
when the business logic itself is the orchestrator. AWS Step Functions
is the managed analog.

---

## Cache / KV

**Boring default 2026: Valkey** (Linux Foundation Redis fork) self-host
OR **AWS ElastiCache for Valkey** managed.

**Gaining:** **DragonFly DB** for vertical scaling (~25× Redis throughput
on multi-core); **Upstash** for per-request pricing on serverless.

**Losing:** Stock Redis OSS (BSL → AGPL license shift in 2024 pushed
new projects to Valkey). Memcached for new projects (data structures
matter; pure cache is rarer than people think).

---

## Search

**Boring default 2026: PostgreSQL FTS + pgvector** up to ~10M docs.
For dedicated typeahead: **Meilisearch** or **Typesense**. For analytics
+ aggregations at scale: **OpenSearch** (managed via AWS) or Vespa.

**Gaining:** **Qdrant** as the OSS vector default. Hybrid keyword+vector
in Meilisearch/Typesense narrowing the gap with Algolia.

**Losing:** Elasticsearch OSS (SSPL split). Algolia at scale (premium
pricing only justifies for elite UX requirements).

---

## Blob storage

**Boring default 2026: AWS S3** (if you're on AWS) or **Cloudflare R2**
(if egress to public matters — zero egress fees).

**Gaining:** **R2** for any CDN-fronted use case (egress savings dwarf
storage savings); **Backblaze B2** for cold archive (cheapest storage).

**Losing:** Self-hosted MinIO for greenfield projects (ops surface
> savings at typical scale). Box for generic asset storage (it's a
document/collaboration tool, not blob infra — use it only for human-facing
document workflows).

---

## Relational / OLTP

**Boring default 2026: PostgreSQL** — managed (RDS, Supabase, Neon)
or self-hosted on a managed-VM substrate.

**Gaining:** **Neon** for preview environments (branching + scale-to-zero
is genuinely transformative). **Supabase** for batteries-included MVPs.
Postgres 17 extensions (vector, time-series via TimescaleDB).

**Losing:** MongoDB for new projects unless the schema really is documents.
DynamoDB unless you've internalised single-table design AND you're on AWS.

**Multi-region active-active:** **CockroachDB**. Don't fake it with
Postgres + logical replication.

---

## Observability stack

**Boring default 2026:** Adopt **OpenTelemetry OTLP** as the wire format;
then choose backend by ops capacity:
- Platform team available → **Grafana LGTM** (Loki + Tempo + Mimir +
  Grafana) self-hosted, backed by S3-compatible object storage.
- No platform team, budget OK → **Datadog** or **Honeycomb**.
- No platform team, budget tight → **Grafana Cloud free tier**.

**Gaining:** **SigNoz** (OSS Datadog-alternative on ClickHouse) for
self-host without LGTM's complexity. **Sentry self-hosted** if you
mostly need errors + traces, not metrics.

**Losing:** ELK stack for new projects (Loki + ClickHouse-based options
are cheaper at high cardinality). Bespoke Prometheus + Grafana setups
without the Loki/Tempo stack.

---

## Feature flags

**Boring default 2026: GrowthBook** OSS — best balance of flag eval +
experimentation in 2026. **LaunchDarkly** for enterprise / large teams.

**Gaining:** **PostHog** (analytics + flags + session replay in one) for
mid-stage startups. GrowthBook Cloud for teams that want managed but
keep self-host as escape.

**Losing:** Custom flag tables ("just a Postgres row with a boolean") past
~10 flags. ConfigCat (still around but less momentum).

---

## Secrets management

**Boring default 2026: AWS Secrets Manager** (if on AWS), **OpenBao**
(Vault OSS fork, post-BSL) for self-host.

**Gaining:** **Doppler** for multi-cloud teams; **Infisical** as the
OSS-first competitor with end-to-end encryption.

**Losing:** Stock HashiCorp Vault for new OSS-leaning projects (license
shift pushed mindshare to OpenBao). `.env` + Bitwarden for teams beyond
3 people or any SOC2 trajectory.

---

## Authentication

**Boring default 2026: NextAuth/Auth.js** (Next.js stacks),
**Better Auth** (framework-agnostic OSS), **Clerk** (managed, polished),
**Supabase Auth** (if you're also using Supabase).

**Gaining:** **Better Auth** as 2025 OSS challenger with plugin model.
**Clerk** for indie/startup velocity. Passkeys (WebAuthn) as default
second factor across the board.

**Losing:** Auth0 for new projects (cost trajectory). Stytch (didn't
break out). Bespoke username/password without password-reset / 2FA
plumbing — that's a CVE waiting to happen.

**Keycloak** is for SSO/IdP infra (you're the identity provider, not
a consumer). Different problem.

---

## Transactional email

**Boring default 2026: Resend** for modern stacks; **Postmark** if
deliverability is paramount; **AWS SES** for cost at scale.

**Gaining:** **Resend** ate SendGrid's startup share via React Email
integration and developer DX.

**Losing:** SendGrid (reputation hits + sales-driven pricing). Self-hosted
Postal / mailcow at any scale below 100k/day (deliverability hell —
the spam folder eats your transactional mail).

---

## Payments

**Boring default 2026: Stripe**, full stop, for direct merchant model.

**Lock-in / cost-aware alternatives:** **Adyen** for global enterprise
(>$1M GMV/mo), **Paddle** / **Lemon Squeezy** (Stripe-owned) for
merchant-of-record (Paddle handles tax compliance for you).

**Losing:** Building payment plumbing yourself. Square for online-first
(in-person is its strength).

**Stripe lock-in is real** — Subscription objects + tax features are
sticky. Plan exit strategy if you're approaching the rate cliff (>$1M GMV).

---

## Cross-cutting: when to NOT pick a managed service

The grey-zone signals that flip you back to self-hosted:

1. **Egress / per-request fees dominate the bill** (Datadog, Mongo Atlas,
   ElastiCache at high write rate).
2. **Compliance forbids the data leaving your VPC** (HIPAA / GDPR /
   sovereignty).
3. **You already have platform team capacity** (the marginal cost of one
   more service is near-zero).
4. **The wire protocol is open** (Postgres, OpenTelemetry, S3 API) — the
   cost of self-hosting later is bounded.

---

## Re-verification checklist (every 3-6 months)

1. Has any commodity gained a new SoTA that displaces the current
   default? (Vibe-check: scan ThoughtWorks Technology Radar, DEV.to
   "best X in 2026", State of Devops Report.)
2. Has any variant shipped a licence change (BSL ↔ MIT / Apache shift)?
   Most disruptive moves in 2023-2025 were licence-driven.
3. Has any of the managed services in our matrix raised prices > 30%?
4. Has compliance landscape moved (e.g. EU AI Act enforcement)?
5. New axis worth adding? (e.g. "supports passkeys?" became a feature-flag
   axis-worthy attribute in 2025.)
