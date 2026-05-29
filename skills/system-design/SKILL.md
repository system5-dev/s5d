---
name: system-design
description: "Interview-driven technology selection for generic commodity components — queue, cache, search, blob storage, relational DB, observability stack, feature flags, secrets management, auth, transactional email, payments. The agent asks throughput / consistency / ops capacity / budget / compliance questions, scores variants via weighted axes, applies hard constraints (zero ops capacity → no self-hosted; SOC2 → no env files; etc.), and emits an s5d decision-tier ADR. Use for: 'which queue should we use', 'self-hosted vs managed', 'pick a feature-flag system', 'architecture decision', 'архитектурный выбор', 'выбор стека'."
argument-hint: "[commodity-id | list]"
---

# system-design

**Generative skill, not analytical.** Where `domain-refactor` finds what's
already wrong, `system-design` helps decide what to build. It runs an
interview through canonical axes — throughput, durability, consistency,
ops capacity, budget, compliance, lock-in tolerance — scores variants via
weighted multi-criteria analysis, applies hard constraints, and lands the
chosen variant as an **s5d decision-tier ADR**.

The whole point is **generic commodity** decisions: queue, cache, search,
DB, blob, observability, flags, secrets, auth, email, payments. NOT for
core domain modelling (that's `s5d` + `scenario-mine`).

## Why this exists

Without a frame, "which queue should we use?" turns into a 90-minute
debate or a coin flip. With the frame:

1. The agent asks the **5-12 questions** that actually matter for queues.
2. Each variant scores against those answers.
3. Hard constraints disqualify variants outright (zero ops capacity → no
   self-hosted Kafka, period).
4. Top 3 surface; the human picks the winner.
5. The choice lands as a versioned ADR in `.s5d/packages/decision.*.s5d.yaml`,
   reviewable via the `s5d` skill's approve/import/drift workflow.

The decision becomes **portable artefact**, not slack-history archaeology.

## Available decision frames (v1)

```
auth                  Authentication / identity
blob-storage          Object / blob storage
cache                 In-memory cache / KV store
email-transactional   Transactional email
feature-flags         Feature flags / progressive delivery
observability-stack   Observability stack (logs + metrics + traces)
payments              Payments / billing platform
queue                 Message queue / event bus
relational-db         Relational / OLTP database
search                Full-text search / semantic search
secrets-management    Secrets / config management
```

Each frame lives at `templates/decisions/<id>.yaml` with: axes + weights
(what matters for this commodity), variants (with per-axis scores 1-5),
hard constraints (rule-based disqualifications / warnings).

Adding a new commodity = one new YAML file. No code changes.

## Flow

```
1. matrix.sh <commodity>                       → comparison matrix in md
2. interview.sh <commodity>                    → ordered questions (high-weight first)
3. (agent asks user, captures answers in answers.json)
4. recommend.sh <commodity> answers.json       → ranked variants + flagged warnings
5. adr.sh <commodity> --chose <id> ...         → .s5d/packages/decision.<id>-choice.s5d.yaml
6. s5d_validate / s5d_preview / s5d_approve    → land the ADR in the ledger
```

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Render comparison matrix | ✓ `matrix.sh` | — |
| Order questions by axis weight | ✓ `interview.sh` | — |
| Apply weighted scoring | ✓ `recommend.sh` | — |
| Evaluate hard constraints | ✓ `recommend.sh` | — |
| Generate s5d ADR skeleton | ✓ `adr.sh` | — |
| **Conducting the interview itself** | — | ✓ |
| Picking the winner when scores are close | — | ✓ (consult human) |
| Filling rationale + consequences in the ADR | — | ✓ |
| Recognising the question doesn't fit any frame | — | ✓ |

The scripts are deterministic scaffolding. The agent leads the conversation
and writes the why.

## Hard rules

- **Scoring is a frame, not the answer.** Top weighted score is the starting
  point. Agent + human pick the winner; close runners-up surface as
  rejected hypotheses in the ADR.
- **Hard constraints are non-negotiable.** If `ops_capacity:zero` AND
  variant is self-hosted, that variant is OUT — even if it tops the weighted
  score. No "but the team will learn ops" overrides.
- **The interview is conducted by the agent, not the script.** `interview.sh`
  emits the question set; the AGENT asks them conversationally, captures
  answers, sanity-checks for surprises.
- **ADR is mandatory.** No commodity decision lands without a written
  `.s5d/packages/decision.*.s5d.yaml`. Future drift makes the original
  intent recoverable.
- **Updating the decision = new ADR.** When we choose to MIGRATE from one
  variant to another (e.g. ElastiCache → DragonFly self-host because budget
  changed), write a NEW decision spec referencing the prior one as
  context, not edit the old one in place.
- **Decision frames are versioned.** When a frame is updated (new variant,
  new axis, weight rebalance) the change is committed with a clear message;
  prior ADRs cite the frame's commit SHA in the future.
- **Out-of-scope:** anything domain-specific. This skill is for commodity
  — generic interchangeable infrastructure. For "should we extract the
  payments domain into a separate service" — that's `s5d` decision tier,
  not this skill.

## Output layout

```
~/.agents/skills/system-design/
├── SKILL.md                              # this file
├── scripts/
│   ├── matrix.sh                         # comparison matrix renderer
│   ├── interview.sh                      # question set per commodity
│   ├── recommend.sh                      # weighted scoring + constraints
│   └── adr.sh                            # write s5d decision spec
├── templates/
│   ├── interview-axes.yaml               # canonical axis definitions
│   └── decisions/
│       ├── auth.yaml
│       ├── blob-storage.yaml
│       ├── cache.yaml
│       ├── email-transactional.yaml
│       ├── feature-flags.yaml
│       ├── observability-stack.yaml
│       ├── payments.yaml
│       ├── queue.yaml
│       ├── relational-db.yaml
│       ├── search.yaml
│       └── secrets-management.yaml
└── references/
    └── catalog.md                        # SoTA per commodity + last-verified date
```

## Worked example

```bash
# 1. What commodity are we picking?
bash ~/.agents/skills/system-design/scripts/matrix.sh list
# → 11 frames

# 2. See the trade-offs for queues
bash ~/.agents/skills/system-design/scripts/matrix.sh queue --format md

# 3. Get the question set, ordered by axis weight
bash ~/.agents/skills/system-design/scripts/interview.sh queue

# 4. Agent asks the human those questions, captures the answers
# (or: bash interview.sh queue --format json > /tmp/q.json, human edits)
cat > /tmp/queue-answers.json <<'JSON'
{
  "commodity": "queue",
  "answers": {
    "throughput": 100,
    "durability": "standard",
    "ops_capacity": "marginal",
    "budget_monthly_usd": 200,
    "latency_p99": 50,
    "multi_region": "single",
    "lock_in_tolerance": "medium",
    "time_to_prod": "days",
    "team_familiarity": "tangential"
  }
}
JSON

# 5. Rank
bash ~/.agents/skills/system-design/scripts/recommend.sh queue /tmp/queue-answers.json
# → top 3 variants with weighted score + any constraint warnings

# 6. Human picks; agent writes the ADR
bash ~/.agents/skills/system-design/scripts/adr.sh queue --chose inngest \
    --decided-by Roman \
    --rationale "Marginal ops capacity + days time-to-prod + serverless-friendly app → managed event-driven service. Inngest matches the Next.js stack and gives durable retries out of the box. Exit cost acceptable for the current scale." \
    --answers /tmp/queue-answers.json

# 7. The ADR is now a draft s5d decision spec at
#    .s5d/packages/decision.queue-choice.s5d.yaml
#    Review with the s5d skill before approving.
```

## When NOT to use

- The decision is for code structure / domain modelling (use `s5d` decision
  tier directly with hypotheses).
- The decision is for code-level patterns (use `domain-refactor` plus
  `s5d`).
- The question is "what feature should we build next" — that's product
  strategy, not system design.
- The repo has < 3 services AND no platform team — defaults from a
  starter template are fine; over-engineering decisions is a tax for tiny
  teams.

## Reading order if you're new

1. `references/catalog.md` — SoTA snapshot (2026-05-28) per commodity.
2. `templates/interview-axes.yaml` — what questions exist and why.
3. One decision frame (e.g. `templates/decisions/cache.yaml`) to see how
   axes + weights + variants + constraints compose.
4. Run the worked example above end-to-end on a throwaway commodity.
