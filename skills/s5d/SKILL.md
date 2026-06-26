---
name: s5d
description: "S5D — control plane for agentic repo changes. Target state → run evidence → decision record → trace → verify."
argument-hint: "[question, feature request, or problem description]"
---

# S5D

Control plane for changes in a repository with AI participation. Four things on top of normal development:

1. **Target state** — describe what the system should become and which architecture it reuses (DDD: domains, capabilities, components).
2. **Explicit choice** — compare alternatives before committing (≥3 hypotheses, different in kind).
3. **Run evidence** — record what agents/tools produced, with integrity (approval chain, SHA256 binding, ledger).
4. **Verify in code** — check that code matches the decision; roll back when it doesn't.

Flow: `Route → Shape → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn`

`Shape` is a pre-control-plane intake layer for unclear intent. It may create or
cite companion notes, but it does not create accepted S5D state. Only
`.s5d/packages/*` and `.s5d/records/*` are lifecycle truth.

`Discover` runs once per project (or when the map is stale — treat it as stale if the domains/components you're about to touch are absent from `.s5d/discovery/architecture-map.md`). Skip it on follow-up tasks that already have a current map.

Applies only to work grounded in an existing repository.

**Skill suite.** S5D ships stack-agnostic domain skills (tests, lint, security, infra, scaling, docs, DDD, boundary, scenarios, system-design) that map to flow stages — recommend/invoke the right one **per stage automatically** rather than doing that stage by hand. Map: [references/skill-suite.md](references/skill-suite.md).

**Reference docs:**
- [metamodel.md](metamodel.md) — read §Complete Artifact Definitions BEFORE writing your first spec. Required-field cascade documented per artifact.
- [references/shape-layer.md](references/shape-layer.md) — raw/vague intent → S5D-ready intake kernel
- [references/adversarial-review.md](references/adversarial-review.md) — blind diff, edge-case, and acceptance review pattern
- [domain-capability-mode.md](domain-capability-mode.md) — Product Intent → domain/capability design → implementation scope
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts, effectiveness metrics
- [references/fpf/](references/fpf/) — FPF corpus. Load via `references/fpf/agent/load-policy.md`: start with `entrypoints.yaml` / `glossary.yaml` / `query-index.jsonl`, expand to modules only when exact wording needed.

---

## Common Gotchas

- **Quote strings containing `:`** in YAML lists. `- Backward compat: foo` parses as a map, not a string. Reword with `—` or quote.
- **Required-field cascade.** `component` requires `feature + domain + container + paths`. `domain` requires `product + classification`. `capability` requires `domain`. `contract.format` is an enum: `openapi / json_schema / protobuf / typespec`. All in `metamodel.md`.
- **`structure_outline` is an object:** `{ summary: string, signatures: [string], types: [string] }`.
- **Edit-after-approve invalidates approval.** Any spec change after `s5d state approve` changes its sha256. Recovery: preview → approve → import again.
- **`verified-by` ≠ `reviewer`.** Import expects a verifier different from the approver — a methodological check that warns and blocks by default but is `--force`-overridable, not a structural invariant. Conventional split: human approves, a second party verifies.
- **gate:review on decision/high tier** closes via review evidence: `s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass`. Hypothesis/evidence commands accept decision- and high-tier specs.
- **Generated CI runs built-in checks only** (validate, component path/architecture markers, drift via `s5d ci exec`) — command gates (test/lint/contract) never execute in PR pipelines (fork trust boundary).
- **Decide requires ≥3 hypotheses.** Two-hypothesis decisions are blocked without `--force`.
- **Discovery output = tables, not prose.** "Discover this project" / "onboard this repo" produces Init Source Survey + Architecture Map in `.s5d/discovery/`, with `[VERIFIED]/[INFERRED]/[SPECULATIVE]` tags per claim. See §Discover.
- **Shape is not accepted state.** Shape notes, PRDs, UX docs, stories, research, and review reports are companion inputs until bound into `.s5d/packages/*` or `.s5d/records/*`.
- **Review reports do not pass gates by themselves.** Adversarial review findings close only when fixed, explicitly deferred, or recorded as S5D evidence/gate state.

### MCP-only operations

- `s5d_waiver` — record an expiring gate waiver (FPF B.3.4:5). Doctrine: assurance gates (schema, graph, review) close via fixes/evidence, not waivers — see the High-tier note below.

---

## Worked Example

```bash
s5d init
s5d new decision.refresh-rotation --tier decision --product auth \
  --question "How should refresh tokens rotate?"

# Decide needs >=3 hypotheses, different in kind, each carrying >=1 evidence
# before s5d decision decide will accept them.
s5d decision add-hypothesis <decision-spec> --title "Server-side rotation" \
  --content "Rotate on every refresh, persist token family state" --scope "auth boundary"
s5d decision add-hypothesis <decision-spec> --title "Client-side sliding expiry" \
  --content "No rotation; short TTL refreshed by the client" --scope "auth boundary"
s5d decision add-hypothesis <decision-spec> --title "Reuse-detection only" \
  --content "Keep static refresh tokens, revoke on detected reuse" --scope "auth boundary"
s5d decision add-evidence <decision-spec> --hypothesis-id server-side-rotation \
  --evidence-type internal --content "Revocation lookup <5ms at p95" \
  --verdict pass --formality 4 --claim-scope latency --reliability 0.8
s5d decision add-evidence <decision-spec> --hypothesis-id client-side-sliding-expiry \
  --evidence-type internal --content "No server revocation path — fails logout-everywhere" \
  --verdict fail --formality 3 --claim-scope revocation --reliability 0.8
s5d decision add-evidence <decision-spec> --hypothesis-id reuse-detection-only \
  --evidence-type internal --content "Detects theft but cannot pre-empt it" \
  --verdict fail --formality 3 --claim-scope revocation --reliability 0.7

# Winner needs a linked feature spec (spec_ref) BEFORE decide.
s5d new feat.refresh-rotation --tier standard --product auth \
  --hypothesis-id server-side-rotation
s5d verify validate <feature-spec>
s5d verify graph-check <feature-spec>

s5d decision decide <decision-spec> --title "Use server-side rotation" \
  --winner server-side-rotation --rejected client-side-sliding-expiry,reuse-detection-only \
  --confirmed-by <approver> \
  --context "Revocation correctness > token statelessness" \
  --decision "Adopt server-side rotation" \
  --rationale "Best revocation/complexity balance" \
  --consequences "Needs persistent token-family store" \
  --challenge-summary "5 probes run: no fatal flaw; weakest link is family-store availability"
s5d state preview <feature-spec>
s5d state approve <feature-spec> --reviewer <approver>
# implement code
s5d verify run-gates <feature-spec>
s5d state import <feature-spec> --verified-by <verifier>
s5d state reflect <feature-spec> --summary "Shipped cleanly" \
  --heuristic "Link winner to feature spec before decide"
```

---

## Scope

Out of scope: bugfix <30 LOC, config-only, docs-only, status query. If Shape
reveals the work is tiny and local, exit S5D rather than opening control-plane
state.

**Two kinds of waivability — do not conflate (full rules in §Waiver):**

- **Structurally non-waivable (no API to bypass):** Decide human confirmation
  (`confirmed_by`) and Run approval (`reviewer`). No flag skips these.
- **Policy-non-waivable but API-reachable:** assurance gates (schema, graph,
  review, contract, privacy). Doctrine (especially high-tier): close them via
  fixes/evidence, not waivers — but the API does not forbid it (`s5d_waiver` records
  `status: waived` with expiry; import accepts a non-expired one). Hold "never
  waivable" as discipline, not a runtime invariant.
- **Waivable with a recorded `WAIVER` line (Lightweight / Standard / Decision):**
  workflow *steps* — e.g. the Target/Decide auto-waiver pointing at a prior
  confirmed decision, or a readiness skip. A waivable step is never a
  human-confirmation point.

So "Decision tier allows step waivers" does **not** mean its Decide confirmation
can be skipped — that confirmation is in the never-waivable set above.

---

## Route & Bootstrap

Classify before touching tools. First match wins. (Out-of-scope work exits S5D — see §Scope.)

**Tier:** choice/tradeoff/architecture → `decision` | feature, 1 domain, no auth/payment/security → `lightweight` | feature, 2+ domains → `standard` | auth/payment/security/PII/compliance → `high` | ambiguous → pick higher.

**Mode:** "discovery" / "onboard project" / "map the domains" → `discover` (go to Discover, not Target) | raw product intent / vague ticket / mixed PRD/design/transcript / unclear acceptance → `shape` | "evaluate/compare" → `prepare` | "implement X" backed by a **concrete decision/spec reference** — a prior decision record (winner + `confirmed_by`) or an existing component in the architecture map the change reuses → `execute` (enters at Spec; records the Target+Decide auto-waiver) | no signal → `prepare`.

The `execute` auto-waiver points at that concrete reference; it never *skips* a decision. A merely *asserted* "the architecture already exists", with no decision record and no mapped component, does not qualify — route it to `prepare`/Target and frame it first.

**MUST verify the backing reference yourself — `s5d_route` does not.** It classifies on keywords and emits `WAIVER: Target+Decide` for any "implement/build/add" phrasing without checking a decision record or the architecture map. Treat its waiver line as a suggestion, not proof a decision exists.

Too vague to tell the domain count after Shape → run Discover / Domain-Capability mapping, then classify. Don't force a tier on unreadable intent.

Emit routing explicitly:
```
Route: tier=standard, mode=prepare, entry=Target
Reason: touches auth + payments, needs target-state framing
```

Cross-check with `s5d_route` (deterministic keyword classifier) when unsure. On disagreement — especially non-English requests or "X and Y" multi-domain, where the classifier is weaker — trust your semantic read and note the divergence.

Run `s5d_init` if no `.s5d/` directory. Then check CI coverage: if the repo has CI (`.github/workflows/` or `.gitlab-ci.yml`) but no generated S5D pipeline, offer `s5d_ci_init` once — local hooks alone are bypassable by a plain `git push`. If generated config exists, `s5d_ci_check` must be clean (stale template → re-run `s5d ci init`).

---

## Shape

Use Shape when the input is a raw idea, vague product request, mixed research,
PRD/design/story material, or an implementation request with unclear acceptance.
Follow [references/shape-layer.md](references/shape-layer.md).

Produce an intake kernel: why this matters, impacted capabilities, known
constraints, non-goals, success signal, assumptions, open questions, and
companion sources. Then route:

- tiny/local/docs/config work → exit S5D and do the direct task
- enough ownership and acceptance → Target
- missing current architecture map → Discover, then Target
- architecture tradeoff → decision-tier Target
- auth/payment/security/PII/compliance → high-tier Target

Do not invent product facts. Mark assumptions explicitly and ask only when the
missing fact is irreversible, external, secret-bearing, or would change the
architecture.

---

## Discover

**Discovery is not a narrative report.** Output is structured tables per `domain-capability-mode.md`. Two artifacts, both saved to `.s5d/discovery/`:

1. **Init Source Survey** — `.s5d/discovery/source-survey.md`. Table with one row per source (GitHub, tracker, design, docs, delivery, runtime evidence, MCP connectors). Columns: `Source | Status | Canonical For | Access | Scope | Confidence | Notes`. Statuses: `connected | available-manual | unavailable | not-used`.

2. **Architecture Map** — `.s5d/discovery/architecture-map.md`. One table per layer:
   - **Domains** — `id | name | classification (core/supporting/generic) | maturity | owner | confidence`
   - **Capabilities** — `id | domain | name | implemented-by | consumed-by`
   - **Entities** — `id | owning-domain | lifecycle | projections | aggregate notes`
   - **Use cases** — `name | feature/source | capabilities | entities | UX surfaces`
   - **Components** — `path | domain | feature | container | capabilities/entities`
   - **UX surfaces** — `screen/flow | bound entities | triggered capabilities | nav in/out`
   - **Edges** — `upstream domain | downstream consumer | capability | contract | transport | archetype`
   - **Unknowns** — `gap | why it matters | how to verify`

Every claim carries a tag: `[VERIFIED]` (read from code/docs/tool), `[INFERRED]` (deduced from verified), `[SPECULATIVE]` (plausible, unconfirmed). Untagged = error.

After writing both files, run `s5d discover sync` (or `s5d_discover_sync` over MCP) to rebuild the machine snapshot (`manifest.yaml`, `files.jsonl`, `graph.json`, `evidence.jsonl`, `metamodel.yaml`). **The two are independent, not validated against each other:** `discover sync` neither generates, reads, nor checks `source-survey.md` / `architecture-map.md`. Those two markdown maps are agent-authored *context* — useful for humans and for your own staleness judgment, but the runtime does not consume them. Don't treat them as a gate or a source of truth the tooling enforces.

Do **not** produce a free-form prose summary in place of these tables. Narrative sections are allowed only as a header before each table or as a closing "Recommended S5D entry points" list.

Triggers: explicit ("discovery", "map the domains", "onboard this repo"), or any first-touch on an unknown repo before opening a feature spec.

---

## Target

Consume the shaped intake kernel when one exists. State what's anomalous. Define
acceptance BEFORE options.

`s5d_new` (tier: decision) creates skeleton. For raw product intent, run Domain-Capability Design first: extract feature intent and use cases, discover current architecture, map impacted capabilities/entities/components/UX surfaces, then decide tier. Use the resulting map to populate the feature spec; do not jump from product text to code. If `--product` is unknown, infer it from existing `.s5d/` specs or the repo manifest and label the assumption; ask only when it can't be inferred.

---

## Decide

≥3 hypotheses, different in kind. Per hypothesis: predictions, decomposition, rigor rating, weakest-link analysis.

Challenge probes before `s5d_decide`, matching the runtime's `--challenge-mode`: `tactical` → 1 probe (strongest counter-argument) for small-blast-radius decisions; `standard` → 5 probes (counter-argument, tail failure, evidence weakness, weakest link, existing alternatives) for decisions gating standard/high work. Fatal flaw from probe → revisit hypotheses. Record the outcome via `--challenge-summary` on decide: the MCP path refuses to decide without it; the CLI accepts `--no-challenge`/`--force` (warns and records the decision with no challenge object — the runtime does not route this through the gate-waiver system). Skipping the challenge is a skill-level step skip: record an explicit WAIVER line.

Human confirms (non-waivable). WAL: `status=AWAITING_HUMAN`. If the winner needs implementation, create the linked feature spec **before** `s5d_decide` — the winner must carry a `spec_ref` or decide bails.

**Linking happens at creation time, on both surfaces:** `s5d new <feature> --hypothesis-id <id>` (CLI) or `s5d_new` with `hypothesis_id` (MCP) sets `spec_ref` on the winner as it scaffolds — one shared linker, warns if the id matches no decision hypothesis. There is no separate "link" command.

---

## Spec

Problem → acceptance scenarios (≥3 GWT) → implementation hypotheses (≥2) → winner → DO/DON'T.

`s5d_new` creates scaffold. For feature specs, trace every user-facing change as `Feature → UseCase → Capability → Component` and declare cross-domain edges/contracts explicitly. Run `s5d_validate` + `s5d_graph_check`.

Before Run, check implementation readiness: one shippable goal, actionable
paths/actions, no `TBD`, at least three acceptance scenarios, explicit
cross-domain contracts, and no unresolved open question that would change the
architecture. **Most of this is an agent-side discipline, not a runtime gate** —
`validate` only checks structural fields (non-empty `why`/`success_signal`,
non-blank list entries, phase shape), not GWT counts or `TBD`. Treat weak
readiness as a stop *you* enforce; only the schema/graph/review gates and the
human approval are machine-checked.

---

## Run

`s5d_preview` → `s5d_approve` (human name required, non-waivable). WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop.

After approval: optionally use `s5d run start` / `s5d run exec` for bounded engine work, implement → local tests → `s5d_run_gates`. REVIEW markers for non-obvious decisions. Engine completion is only evidence; human acceptance is explicit (`s5d run accept <spec> --id <state> --reviewer <name>`).

**`s5d run exec` is CLI-only and needs a configured, `approved: true` engine in `.s5d/config.yaml`** — with no engine configured it can't run. The MCP surface exposes `s5d_phase_start`/`_phase_accept` and `s5d_execute_loop` (emits a task package), not direct external-engine execution. So "don't call Claude/Codex/Gemini directly" applies only when an approved engine is actually wired; otherwise you implement in-session and record evidence by hand.

### Autonomous loop (mandate envelope)

For long unattended runs, a spec may carry a `mandate:` envelope — `{scope, budget (max_calls/max_time_s), min_gate_floor, stop_conditions}`. A human admits it **once** (`s5d_mandate_admit`, SHA-bound to the spec); then the agent drives the loop by calling `s5d_mandate_run` each cycle. Each step adjudicates admission → recorded gate floor → drift → budget and returns the next phase, or halts: escalate (gate/drift/budget/spec-edited) or complete (scope exhausted). High/decision tiers are excluded — they stay human-gated per action. s5d only adjudicates and authorizes; **the agent runs the phase** (`s5d run start`/`exec`) and re-invokes — s5d never spawns an engine.

---

## Adversarial Review

For material diffs, decision-tier work, high-tier work, or any `gate:review`
closure, use [references/adversarial-review.md](references/adversarial-review.md).
Run the review as a layered pattern: blind diff review, edge-case review, and
acceptance audit. If a layer cannot run, record the missing layer as a review
limitation rather than pretending the review passed.

Findings close only when fixed, explicitly deferred, or recorded as S5D
evidence/gate state. For decision/high specs, bind review evidence with
`s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass`
(`--verdict` is required).
For feature specs, bind the result through the record/gate mechanism available
for that spec. A standalone markdown report is a companion document, not a
passing gate.

---

## Verify / Ship / Learn

Verify: tests → adversarial review when material → `s5d_import` (SHA256 chain).
Import rejects on hash mismatch → re-preview, re-approve.

Ship: push and deploy require explicit human permission per action. Before the first push on a repo, ensure CI enforcement exists (`s5d_ci_init` — generates a PR pipeline running `s5d ci exec` built-in checks); on later ships run `s5d_ci_check` and refresh stale config.

Learn: `s5d_reflect`. Update WAL. Record reusable heuristics.

---

## Command Contract

Key commands (MCP + CLI). Full preconditions and the run/harness surface live in the repo's `S5D.md` Command Contract.

| Action | MCP | CLI |
|--------|-----|-----|
| Bootstrap | `s5d_init` | `s5d init` |
| Create scaffold | `s5d_new` | `s5d new` |
| Shape kernel (route + readiness; `emit_spec` embeds it) | `s5d_shape` | `s5d shape <kernel.yaml>` |
| Adversarial review scaffold | `s5d_review_adversarial` | `s5d review adversarial <spec>` |
| Story phases | `s5d_plan_stories` | `s5d plan stories <spec> --from <yaml>` |
| Add hypothesis | `s5d_add_hypothesis` | `s5d decision add-hypothesis` |
| Add evidence | `s5d_add_evidence` | `s5d decision add-evidence` |
| CI config | `s5d_ci_init` | `s5d ci init` |
| CI staleness | `s5d_ci_check` | `s5d ci check` |
| Validate | `s5d_validate` | `s5d verify validate` |
| Graph check | `s5d_graph_check` | `s5d verify graph-check` |
| Preview | `s5d_preview` | `s5d state preview` |
| Approve | `s5d_approve` | `s5d state approve --reviewer <name>` |
| Run gates | `s5d_run_gates` | `s5d verify run-gates` |
| Mandate admit (authorize autonomous loop) | `s5d_mandate_admit` | `s5d mandate admit <spec> --reviewer <name>` |
| Mandate run (one loop control step) | `s5d_mandate_run` | `s5d mandate run <spec>` |
| Benchmark assistant variants | — | `s5d run benchmark <suite.json\|yaml> [--format markdown\|json]` |
| Import | `s5d_import` | `s5d state import --verified-by <name>` |
| Decide | `s5d_decide` | `s5d decision decide --confirmed-by <name>` |
| Reflect | `s5d_reflect` | `s5d state reflect` |
| Drift check | `s5d_drift_check` | `s5d state drift-check` |
| Rollback | `s5d_rollback` | `s5d state rollback` |
| Trace | `s5d_trace` | `s5d trace <path>` |
| Debt harvest | `s5d_debt` | `s5d debt [--check]` |
| Run work states (list/start/accept) | `s5d_phase_list` / `s5d_phase_start` / `s5d_phase_accept` | `s5d run list` / `start` / `accept` |
| Bounded engine work | `s5d_execute_loop` (emits a task package) | `s5d run exec` (runs an approved engine) — **different operations, not aliases** |
| Problem / acceptance card | `s5d_set_problem` / `s5d_set_acceptance` | `s5d decision set-problem` / `set-acceptance` |
| Architecture check | `s5d_check` | `s5d verify check` |
| Discover sync / check | `s5d_discover_sync` / `s5d_discover_check` | `s5d discover sync` / `check` |
| Codebase sync / check | `s5d_codebase_sync` / `s5d_codebase_check` | `s5d codebase sync` / `check` |
| Reconcile drift | `s5d_reconcile` | `s5d state reconcile` |
| Route classify | `s5d_route` | `s5d route` |
| Record gate waiver | `s5d_waiver` | — (MCP-only) |
| Note / Status / Show | `s5d_note` / `s5d_status` / `s5d_show` | `s5d note` / `s5d status` / `s5d show` |

**Naming note:** the phase work-states are `s5d run list/start/accept` (CLI) ↔ `s5d_phase_*` (MCP) — same states, different names. But `s5d run exec` (CLI, runs an approved engine) and `s5d_execute_loop` (MCP, emits a task package) are **distinct operations**, not two names for one thing. This table is the working set; the full surface lives in the repo's `S5D.md`.

Legacy hidden aliases (e.g. `s5d add-hypothesis`, `s5d validate`, `s5d apply preview`) remain supported for existing scripts.

### Simplification debt markers

Constitution #3 allows temporary measures only when explicitly marked with a
removal plan. Mark one at the cut-site with an inline comment: a comment leader
(`//`, `#`, `/*`, or `<!--`) immediately followed by the token
`s5d:debt(ceiling="what was cut", trigger="when to revisit")`. `ceiling` names
the limitation; `trigger` names the condition to revisit it.

`s5d debt` harvests these read-only, binds each marker to its owning
spec/component (via the architecture component-path map), groups the report by
owner, and flags `no-trigger` (empty or missing trigger — rots silently) and
`unowned` (no component covers the file). `s5d debt --check` exits non-zero when
any `no-trigger` marker exists, for use in CI or a pre-commit hook.

---

## Skill Runtime Contract

The skill is a **human-facing conductor** of the S5D state machine — it is **not a second state machine**. The CLI/MCP own durable state (records, ledger, harness journal); the skill routes, drafts, summarizes, and recommends. It runs only engines marked `approved: true` in `.s5d/config.yaml`.

**The skill may:** route the request, shape vague intent, help draft/edit spec YAML, read run artifacts and summarize, coordinate adversarial review, normalize engine outputs into hypotheses/evidence via S5D commands, recommend run acceptance when evidence is sufficient.

**The skill must not:** store its own run state outside S5D files, treat Shape or review markdown as accepted state, treat harness journal state as run truth, treat engine run as accepted evidence, call Claude/Codex/Gemini directly for S5D execution when `s5d run exec` applies, approve an engine not in `.s5d/config.yaml`, bypass preview/approve/import hash checks.

---

## Waiver

Only way to skip a step:
```
WAIVER: <step> | Reason: <why> | Condition: <when required again> | Approved: <name>
```
Non-waivable: Decide human confirmation, Run approval. Route-to-Spec is an auto-waiver for Target and Decide — record it explicitly.

**High-tier "no waivers" ≠ the Target/Decide auto-waiver.** The never-waivable set and the policy-vs-API distinction live in §Scope. The Target+Decide auto-waiver is *not* a gate waiver — it points at a **prior confirmed decision** (winner + `confirmed_by`), it does not skip one; a high-tier feature with no prior decision record cannot auto-waive Decide, so frame it first.
