# Shape Layer

Shape is the intake layer before the S5D control plane. It converts vague or
mixed intent into a small kernel that can be routed into Discover, Target,
Decide, or direct out-of-scope execution.

Shape is not accepted state. It may cite or draft companion material, but S5D
truth still lives only in `.s5d/packages/*` and `.s5d/records/*`.

## Use Shape When

- The request is a raw product idea.
- Acceptance is unclear or missing.
- The input mixes PRD, UX, research, customer notes, story text, or chat context.
- The implementation request names a desired outcome but not the affected
  capability or boundary.
- The domain count, risk tier, or first S5D entry point is unclear.

## Skip Shape When

- The work is a tiny bugfix, config-only change, docs-only change, or status
  query that should exit S5D.
- A current spec, accepted decision, and architecture map already make the entry
  point obvious.
- The user asks only for current S5D state.
- The work is already clearly high-tier and concrete enough to enter Target.

## Intake Kernel

Produce the smallest useful kernel:

| Field | Meaning |
|-------|---------|
| `why` | The business, user, operational, or architectural reason this work matters. |
| `capabilities` | Candidate capabilities or use cases implicated by the request. |
| `constraints` | Known technical, product, legal, timing, compatibility, or migration constraints. |
| `non_goals` | What the work explicitly should not attempt. |
| `success_signal` | Observable outcome that would make the work worth doing. |
| `assumptions` | Claims not yet verified. Mark them as assumptions, not facts. |
| `open_questions` | Questions whose answers would change scope, architecture, or risk. |
| `companions` | External notes, PRDs, UX docs, research, stories, or review reports used as input. |

## Routing Outcomes

- `out_of_scope_quick_fix` — the shaped work is tiny/local/docs/config; exit S5D.
- `target_lightweight` — one-domain feature, low risk, acceptance clear enough.
- `target_standard` — multi-domain or material feature work.
- `target_high` — auth, payment, security, PII, compliance, or other high-risk surface.
- `decision_prepare` — architecture choice or tradeoff is the core work.
- `discover_required` — current architecture ownership is missing or stale.
- `blocked_missing_source` — missing external, secret-bearing, irreversible, or
  architecture-changing information.

## Rules

- Do not invent product facts. Use assumptions and open questions.
- Do not open S5D state just to preserve a Shape note.
- Do not let companion documents become the source of lifecycle truth.
- Elevate risk immediately when Shape reveals auth, payment, security, PII,
  compliance, migration, or cross-domain blast radius.
- If ownership is unknown, run Discover before Target.
- Ask the user only for missing information that is irreversible,
  external-side-effecting, secret-bearing, or architecture-changing.

## Native Runtime Support

The kernel is a first-class spec field (`intent_kernel`) — optional, and absent
from specs that never used Shape. Runtime commands (CLI / MCP):

- `s5d shape <kernel.yaml>` / `s5d_shape` — classify the kernel through the
  deterministic router and check structural readiness; with
  `--emit-spec <id>` (`emit_spec`) it scaffolds a spec with the kernel embedded,
  so approval sha-binds the intake intent and the Acceptance Auditor reviews
  bound intent instead of loose markdown.
- `s5d review adversarial <spec>` / `s5d_review_adversarial` — scaffold the
  3-layer report into `.s5d/evidence/<spec>/` and print the tier-correct
  evidence binding step.
- `s5d plan stories <spec> --from <stories.yaml>` / `s5d_plan_stories` — append
  story-shaped workflow phases (each story must carry acceptance; rollback
  defaults). Editing a spec changes its sha — re-approve if already approved.

Readiness is structural (why + success_signal non-empty, no blank list
entries) — content truthfulness stays with the agent.

## Readiness Bridge

Before leaving Shape, make the next step explicit:

```text
Shape: ready
Route: tier=<lightweight|standard|decision|high>, mode=<prepare|execute>, entry=<Discover|Target|Spec>
Reason: <one sentence>
Assumptions: <bounded list>
Open questions: <only blockers>
```

If the next step is Run, Shape should not be the only supporting artifact. A spec
must still define acceptance, implementation scope, gates, and evidence binding.
