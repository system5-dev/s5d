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

Flow: `Route → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn`

`Discover` runs once per project (or when the map is stale — treat it as stale if the domains/components you're about to touch are absent from `.s5d/discovery/architecture-map.md`). Skip it on follow-up tasks that already have a current map.

Applies only to work grounded in an existing repository.

**Skill suite.** S5D ships stack-agnostic domain skills (tests, lint, security, infra, scaling, docs, DDD, boundary, scenarios, system-design) that map to flow stages — recommend/invoke the right one **per stage automatically** rather than doing that stage by hand. Map: [references/skill-suite.md](references/skill-suite.md).

**Reference docs:**
- [metamodel.md](metamodel.md) — read §Complete Artifact Definitions BEFORE writing your first spec. Required-field cascade documented per artifact.
- [domain-capability-mode.md](domain-capability-mode.md) — Product Intent → domain/capability design → implementation scope
- [session-protocol.md](session-protocol.md) — WAL format, spec:// URI, REVIEW markers, conflicts, effectiveness metrics
- [references/fpf/](references/fpf/) — FPF corpus. Load via `references/fpf/agent/load-policy.md`: start with `entrypoints.yaml` / `glossary.yaml` / `query-index.jsonl`, expand to modules only when exact wording needed.

---

## Common Gotchas

- **Quote strings containing `:`** in YAML lists. `- Backward compat: foo` parses as a map, not a string. Reword with `—` or quote.
- **Required-field cascade.** `component` requires `feature + domain + container + paths`. `domain` requires `product + classification`. `capability` requires `domain`. `contract.format` is an enum: `openapi / json_schema / protobuf / typespec`. All in `metamodel.md`.
- **`structure_outline` is an object:** `{ summary: string, signatures: [string], types: [string] }`.
- **Edit-after-approve invalidates approval.** Any spec change after `s5d state approve` changes its sha256. Recovery: preview → approve → import again.
- **`verified-by` ≠ `reviewer`.** Import requires a verifier different from the approver. Conventional split: human approves, Diana verifies.
- **gate:review on decision/high tier** closes via review evidence: `s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass`. Hypothesis/evidence commands accept decision- and high-tier specs.
- **Generated CI runs built-in checks only** (validate, architecture, drift via `s5d ci exec`) — command gates (test/lint/contract) never execute in PR pipelines (fork trust boundary).
- **Decide requires ≥3 hypotheses.** Two-hypothesis decisions are blocked without `--force`.
- **Discovery output = tables, not prose.** "Discover this project" / "дискавери" produces Init Source Survey + Architecture Map in `.s5d/discovery/`, with `[VERIFIED]/[INFERRED]/[SPECULATIVE]` tags per claim. See §Discover.

### MCP-only operations

- `s5d_waiver` — record an expiring gate waiver (FPF B.3.4:5). Doctrine: assurance gates (schema, graph, review) close via fixes/evidence, not waivers — see the High-tier note below.

---

## Worked Example

```bash
s5d init
s5d new decision.refresh-rotation --tier decision --product auth \
  --question "How should refresh tokens rotate?"
s5d decision add-hypothesis <spec> --title "Server-side rotation" \
  --content "Rotate on every refresh, persist token family state" --scope "auth boundary"
s5d decision add-evidence <spec> --hypothesis-id server-side-rotation \
  --evidence-type internal --content "Revocation lookup <5ms at p95" \
  --verdict pass --formality 4 --claim-scope latency --reliability 0.8
s5d new feat.refresh-rotation --tier standard --product auth \
  --hypothesis-id server-side-rotation
s5d verify validate <feature-spec>
s5d decision decide <decision-spec> --title "Use server-side rotation" \
  --winner server-side-rotation --confirmed-by Roman \
  --context "Revocation correctness > token statelessness" \
  --decision "Adopt server-side rotation" \
  --rationale "Best revocation/complexity balance" \
  --consequences "Needs persistent token-family store" \
  --challenge-summary "5 probes run: no fatal flaw; weakest link is family-store availability"
s5d state preview <feature-spec>
s5d state approve <feature-spec> --reviewer Roman
# implement code
s5d verify run-gates <feature-spec>
s5d state import <feature-spec> --verified-by Diana
s5d state reflect <feature-spec> --summary "Shipped cleanly" \
  --heuristic "Link winner to feature spec before decide"
```

---

## Scope

Out of scope: bugfix <30 LOC, config-only, docs-only.

| Tier | Waivers |
|------|---------|
| Lightweight | Steps can be waived |
| Standard | Steps can be waived |
| Decision | Steps can be waived |
| High | No waivers allowed |

---

## Route & Bootstrap

Classify before touching tools. First match wins.

Out of scope (exit S5D): bugfix <30 LOC, config-only, docs-only, status query.

**Tier:** choice/tradeoff/architecture → `decision` | feature, 1 domain, no auth/payment/security → `lightweight` | feature, 2+ domains → `standard` | auth/payment/security/PII/compliance → `high` | ambiguous → pick higher.

**Mode:** "discovery" / "дискавери" / "onboard project" / "map the domains" → `discover` (go to Discover, not Target) | raw product intent / ticket / design → `prepare` | "evaluate/compare" → `prepare` | "implement X" with a confirmed decision record or stated existing architecture → `execute` (enters at Spec; records the Target+Decide auto-waiver) | no signal → `prepare`.

Too vague to tell the domain count → run Discover / Domain-Capability mapping first, then classify. Don't force a tier on unreadable intent.

Emit routing explicitly:
```
Route: tier=standard, mode=prepare, entry=Target
Reason: touches auth + payments, needs target-state framing
```

Cross-check with `s5d_route` (deterministic keyword classifier) when unsure. On disagreement — especially non-English requests or "X and Y" multi-domain, where the classifier is weaker — trust your semantic read and note the divergence.

Run `s5d_init` if no `.s5d/` directory. Then check CI coverage: if the repo has CI (`.github/workflows/` or `.gitlab-ci.yml`) but no generated S5D pipeline, offer `s5d_ci_init` once — local hooks alone are bypassable by a plain `git push`. If generated config exists, `s5d_ci_check` must be clean (stale template → re-run `s5d ci init`).

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

After writing both files, run `s5d discover sync` (or `s5d_discover_sync` over MCP) to rebuild the technical index (`.s5d/discovery/index.*`) so file-level evidence is linked. The two outputs are complementary: source-survey/architecture-map are agent-authored maps; `discover sync` is the deterministic file index.

Do **not** produce a free-form prose summary in place of these tables. Narrative sections are allowed only as a header before each table or as a closing "Recommended S5D entry points" list.

Triggers: explicit ("discovery", "дискавери проекта", "map the domains", "onboard this repo"), or any first-touch on an unknown repo before opening a feature spec.

---

## Target

State what's anomalous. Define acceptance BEFORE options.

`s5d_new` (tier: decision) creates skeleton. For raw product intent, run Domain-Capability Design first: extract feature intent and use cases, discover current architecture, map impacted capabilities/entities/components/UX surfaces, then decide tier. Use the resulting map to populate the feature spec; do not jump from product text to code. If `--product` is unknown, infer it from existing `.s5d/` specs or the repo manifest and label the assumption; ask only when it can't be inferred.

---

## Decide

≥3 hypotheses, different in kind. Per hypothesis: predictions, decomposition, rigor rating, weakest-link analysis.

Challenge probes before `s5d_decide`: Lightweight → 1 probe (strongest counter-argument). Standard / High / Decision → 5 probes (counter-argument, tail failure, evidence weakness, weakest link, existing alternatives). Fatal flaw from probe → revisit hypotheses. Record the outcome via `--challenge-summary` on decide — the runtime refuses to decide without it (`--no-challenge` exists but skipping the challenge needs an explicit WAIVER line).

Human confirms (non-waivable). WAL: `status=AWAITING_HUMAN`. If the winner needs implementation, create the linked feature spec **before** `s5d_decide` — the winner must carry a `spec_ref` — then confirm.

---

## Spec

Problem → acceptance scenarios (≥3 GWT) → implementation hypotheses (≥2) → winner → DO/DON'T.

`s5d_new` creates scaffold. For feature specs, trace every user-facing change as `Feature → UseCase → Capability → Component` and declare cross-domain edges/contracts explicitly. Run `s5d_validate` + `s5d_graph_check`.

---

## Run

`s5d_preview` → `s5d_approve` (human name required, non-waivable). WAL: `status=AWAITING_HUMAN, pending=approve <spec-id>`. Stop.

After approval: optionally use `s5d run start` / `s5d run exec` for bounded engine work, implement → local tests → `s5d_run_gates`. REVIEW markers for non-obvious decisions. Engine completion is only evidence; human acceptance is explicit (`s5d run accept <spec> --id <state> --reviewer <name>`).

---

## Verify / Ship / Learn

Verify: tests → `s5d_import` (SHA256 chain). Import rejects on hash mismatch → re-preview, re-approve.

Ship: push and deploy require explicit human permission per action. Before the first push on a repo, ensure CI enforcement exists (`s5d_ci_init` — generates a PR pipeline running `s5d ci exec` built-in checks); on later ships run `s5d_ci_check` and refresh stale config.

Learn: `s5d_reflect`. Update WAL. Record reusable heuristics.

---

## Command Contract

Key commands (MCP + CLI). Full preconditions and the run/harness surface live in the repo's `S5D.md` Command Contract.

| Action | MCP | CLI |
|--------|-----|-----|
| Bootstrap | `s5d_init` | `s5d init` |
| Create scaffold | `s5d_new` | `s5d new` |
| Add hypothesis | `s5d_add_hypothesis` | `s5d decision add-hypothesis` |
| Add evidence | `s5d_add_evidence` | `s5d decision add-evidence` |
| CI config | `s5d_ci_init` | `s5d ci init` |
| CI staleness | `s5d_ci_check` | `s5d ci check` |
| Validate | `s5d_validate` | `s5d verify validate` |
| Graph check | `s5d_graph_check` | `s5d verify graph-check` |
| Preview | `s5d_preview` | `s5d state preview` |
| Approve | `s5d_approve` | `s5d state approve --reviewer <name>` |
| Run gates | `s5d_run_gates` | `s5d verify run-gates` |
| Import | `s5d_import` | `s5d state import --verified-by <name>` |
| Decide | `s5d_decide` | `s5d decision decide --confirmed-by <name>` |
| Reflect | `s5d_reflect` | `s5d state reflect` |
| Drift check | `s5d_drift_check` | `s5d state drift-check` |
| Rollback | `s5d_rollback` | `s5d state rollback` |
| Trace | `s5d_trace` | `s5d trace <path>` |

Legacy hidden aliases (e.g. `s5d add-hypothesis`, `s5d validate`, `s5d apply preview`) remain supported for existing scripts.

---

## Skill Runtime Contract

The skill is a **human-facing conductor** of the S5D state machine — it is **not a second state machine**. The CLI/MCP own durable state (records, ledger, harness journal); the skill routes, drafts, summarizes, and recommends. It runs only engines marked `approved: true` in `.s5d/config.yaml`.

**The skill may:** route the request, help draft/edit spec YAML, read run artifacts and summarize, normalize engine outputs into hypotheses/evidence via S5D commands, recommend run acceptance when evidence is sufficient.

**The skill must not:** store its own run state outside S5D files, treat harness journal state as run truth, treat engine run as accepted evidence, call Claude/Codex/Gemini directly for S5D execution when `s5d run exec` applies, approve an engine not in `.s5d/config.yaml`, bypass preview/approve/import hash checks.

---

## Waiver

Only way to skip a step:
```
WAIVER: <step> | Reason: <why> | Condition: <when required again> | Approved: <name>
```
Non-waivable: Decide human confirmation, Run approval. Route-to-Spec is an auto-waiver for Target and Decide — record it explicitly.

**High-tier "no waivers" ≠ the Target/Decide auto-waiver.** "No waivers" governs *assurance gates* (schema, graph, review, contract, privacy, human approval) — never waivable, any tier. The Target+Decide auto-waiver is *not* a gate waiver: it points at a **prior confirmed decision** (winner + `confirmed_by`), it does not skip one. A high-tier feature with no prior decision record cannot auto-waive Decide — frame it first.
