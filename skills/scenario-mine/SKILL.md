---
name: scenario-mine
description: "Extract BDD scenarios (Given/When/Then) from a project's code and documentation. Reads existing tests, route handlers, doc headings, and (when present) the s5d architecture map. Outputs traceable Gherkin .feature files in .s5d/scenarios/ and/or s5d acceptance YAML snippets ready for feature spec injection. Use for: 'extract scenarios from code', 'mine BDD from this module', 'generate acceptance criteria for use case X', 'извлеки сценарии', 'BDD из кода'."
argument-hint: "[scope path or use-case-id | --auto]"
---

# scenario-mine

Reverse-engineers BDD scenarios from existing material. Three sources feed the mine:

1. **Code** — test names (Jest/Vitest/pytest/Go test/JUnit/xUnit/RSpec/PHPUnit/cucumber) + route handler signatures.
2. **Documentation** — markdown H2/H3/H4 headings and checklist items.
3. **s5d architecture map** — `.s5d/discovery/architecture-map.md` (if present) supplies the use-case taxonomy each scenario gets tagged with.

Output is **traceable**: every scenario carries `@s5d-use-case`, `@s5d-domain`, and `@source:<file:line>` tags so the link between code, scenario, and s5d artefact is mechanical, not narrative.

## Why this skill exists

- s5d feature specs require **≥3 GWT acceptance scenarios** per use case. Hand-writing these for an existing codebase is the bottleneck. This skill mines them.
- Test names already encode behaviour ("`it('returns zeros when attendees is missing')`"). Translating to Gherkin is mostly mechanical lifting.
- Documentation often has the user-facing flow buried in markdown headings. Mining catches behaviour that has no test yet.

## Flow

```
1. discover.sh           → JSON listing tests/features/docs/API/s5d
2. extract-tests.sh      → JSON of test names with file:line
3. extract-docs.sh       → JSON of doc headings + checklist items
4. (agent synthesises)   → Gherkin .feature files + acceptance YAML
5. validate.sh           → Gherkin syntax check
6. (agent writes)        → .s5d/scenarios/<use-case-id>.feature + index.yaml
```

## Output layout

```
.s5d/scenarios/
├── index.yaml                              # generated map: feature ↔ use-case ↔ sources ↔ gaps
├── <use-case-id>.feature                   # one file per use case (main slice)
├── <use-case-id>.<slice>.feature           # additional slices when use case is too dense
└── acceptance/                             # mandatory — YAML snippets for s5d feature spec injection
    ├── <use-case-id>.yaml
    └── <use-case-id>.<slice>.yaml
```

`acceptance/` is **mandatory**, not optional. Every `.feature` file gets a paired `.yaml` snippet so a future s5d feature spec can pull acceptance scenarios mechanically.

> **S5D injection (per decision.skill-suite-integration):** acceptance snippets are injected into a feature spec's `use_cases[].acceptance` at the **YAML layer** — copy the snippet into the spec, then `s5d validate`. There is no dedicated `s5d` CLI verb for acceptance injection yet (deferred); the file-based snippet is the canonical path for now.

## Use case ID convention

A use case ID is the **slug column** in `.s5d/discovery/architecture-map.md` → §"## 4. Use cases". If a use case has no `id` column, **add one before mining** — do not derive slugs implicitly. Two agents mining the same map must reach the same slugs, so the map is the source of truth, not naming intuition.

Slug rules (used to populate the map): lowercase, ASCII, words separated by `-`, no domain prefix (the `@s5d-domain` tag handles that), max ~32 chars. Examples: `apply-for-event-insurance`, `partner-onboarding`, `cron-coi-backfill`.

## Tag contract

**Feature header — mandatory:**
- `@s5d-use-case:<id>` — must resolve to a row in the architecture-map use cases table
- `@s5d-domain:<id>` — denormalised for filter convenience; must match the row's domain
- `@slice:<name>` — when the feature is one slice of a multi-file use case (see "Split feature files" below)

**Every scenario — mandatory:**
- `@kind:happy|edge|failure|permission|speculative` — exactly one
- At least one of:
  - `@source:<file:line>` — the code or doc that motivated this scenario
  - `@from-test:<test-file:line>` — when reverse-engineered from an existing test
- Multiple `@source` / `@from-test` tags on the same scenario are explicitly allowed when several files together motivate it (e.g. webhook + ensure-policy + e2e test).

Untagged scenarios fail review. `@kind:speculative` is allowed only when no `@source` or `@from-test` can be produced — must be surfaced in the run report (see "Reporting" below).

## Split feature files

A use case may span multiple `.feature` files when it has >7 scenarios or when scenarios cluster by very different rhythms (e.g. happy-path UI flow vs. parametric pricing). Naming:

```
.s5d/scenarios/<use-case-id>.feature            # main happy / failure scenarios
.s5d/scenarios/<use-case-id>.<slice>.feature    # e.g. .pricing, .coi, .permissions
```

All slices share the same `@s5d-use-case` tag and add `@slice:<name>` at the Feature header. The index.yaml lists every slice with its scenario count.

## Domain shape variants

Different domains produce different scenario rhythms. The skill must recognise these and adjust expectations:

- **B2C apply / dashboard** — actor is the visitor / customer. Mix of happy-path UI flows and edge cases. Heavy `@from-test` coverage (e2e + unit tests).
- **B2B partner program** — actor is a partner user with tenant boundaries. **Every use case here MUST include at least one `@kind:permission` scenario** (partner A cannot read partner B's data). Coverage typically thin — expect many `@kind:speculative` until rollout matures.
- **Ops / mission-control** — actor is an admin. Permission scenarios mandatory (admin-only access). Often no test coverage; `@source` on route handlers + `@kind:speculative` is the norm.
- **Cron / scheduled** — actor is the scheduler (Vercel cron etc.), not a human. Scenarios test behaviour under: empty queue, single record, many records, transient external failure, partial-success retry. If the route handler has no retry/idempotency logic, that's a **gap to surface**, not invent.
- **Smart-fill / AI** — actor is the visitor + the LLM provider. Scenarios should cover: model-deprecation fallback, malformed-output handling, rate-limit response, redact-PII before send (if relevant).

## Reporting

After mining a batch of use cases, emit a structured run report (in your final agent turn):

```
files:
  - <slug>.feature  (N scenarios; X happy, Y edge, Z failure, W permission, S speculative)
  - <slug>.<slice>.feature  (...)

validator: <count>/<total> ✓

speculative_scenarios:
  - <slug>:<scenario-name>  — <one-line justification>

coverage_gaps:
  - <capability or use case>: no test evidence found
  - <capability>: route handler is a stub (file:line)
  - <observability>: cron <slug> has no logging / metric / alert evidence
```

The skill is doing as much value through the **gaps** it surfaces as through the scenarios it writes. Don't hide them.

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Stack-aware source discovery | ✓ `discover.sh` | — |
| Test-name pattern extraction | ✓ `extract-tests.sh` (9 langs) | — |
| Markdown heading/checklist pull | ✓ `extract-docs.sh` | — |
| Reading s5d architecture map | — | ✓ (parse use_cases table) |
| Picking which extracted item maps to which use case | — | ✓ (judgement; some tests cross use cases) |
| Synthesising natural Given/When/Then sentences from a test name | — | ✓ |
| Identifying missing edge cases (no test for X) | — | ✓ |
| Gherkin syntax validation | ✓ `validate.sh` | — |
| Writing files + index | — | ✓ |

## Hard rules

- **Trace, don't invent.** Every scenario needs at least one `@source` or `@from-test` tag. If you cannot point to a file:line, the scenario is `@kind:speculative` AND surfaced in the run report — never silently invented.
- **Anchor to architecture-map use cases.** Refuse to write a feature file without a `@s5d-use-case` tag when an architecture map exists. If a candidate scenario doesn't fit any existing use case, surface it as a coverage gap (the architecture map needs extending) — don't invent a use case here.
- **One use case per `@s5d-use-case` tag.** A use case CAN span multiple `.feature` files via the `@slice:` tag (see "Split feature files"). What it cannot do: stretch one feature file across two use cases.
- **`acceptance/<slug>.yaml` is mandatory.** Every Feature file gets a matching acceptance YAML for s5d feature spec injection. No exceptions.
- **Multi-tenant domains require a `@kind:permission` scenario.** Partner-program, mission-control, anything with tenant or role boundaries — at least one scenario per use case must verify the boundary holds.
- **Cron use cases require a failure / retry scenario.** If the route handler has no retry/idempotency logic, the scenario reads as speculative AND that's surfaced as an observability/reliability gap.
- **Never reverse-engineer authentication / authorisation rules without explicit confirmation.** Auth scenarios drive real access decisions; they need product sign-off, not extraction.
- **Idempotent regeneration.** Re-running the skill must produce the same output for the same input (sort scenarios deterministically, use stable file ordering, derive timestamps from a fixed clock when possible).
- **Scope filter is directory-based.** `scripts/discover.sh --scope <dir>` walks one directory subtree. Glob support is not implemented in v1 — to mine multiple disjoint paths, run discover multiple times and merge.

## Worked example

```bash
# 1. Run discovery on a specific use case scope
bash ~/.agents/skills/scenario-mine/scripts/discover.sh --scope app/api/application > /tmp/sources.json

# 2. Read the s5d architecture-map to find the use case id
grep -A1 "apply-for-event-insurance" .s5d/discovery/architecture-map.md

# 3. Feed test files into extract-tests.sh
python3 -c "import json; d=json.load(open('/tmp/sources.json'));
            print('\n'.join(d['tests']['js']))" \
  | bash ~/.agents/skills/scenario-mine/scripts/extract-tests.sh > /tmp/tests.json

# 4. Feed relevant docs into extract-docs.sh
echo "PROJECT_SUMMARY.md
README.md" | bash ~/.agents/skills/scenario-mine/scripts/extract-docs.sh > /tmp/docs.json

# 5. Agent synthesises .s5d/scenarios/apply-for-event-insurance.feature
#    using templates/feature.gherkin as a starting structure, with
#    each scenario tagged @source:<file:line> from the extracted JSON.

# 6. Validate
bash ~/.agents/skills/scenario-mine/scripts/validate.sh .s5d/scenarios/apply-for-event-insurance.feature

# 7. Update index.yaml
```

## Drift check (manual, v1)

After scenarios exist, drift accumulates in three directions. Detect via:

1. **Code moved, scenarios stale.** Resolve every `@source:<file:line>` and `@from-test:<file:line>` in the `.feature` files; flag any that no longer exists or whose surrounding code has materially changed.
2. **Test gone, scenario unanchored.** For every `@from-test` tag, confirm the test still exists and the test name still matches the scenario name.
3. **Use case removed, scenarios orphaned.** Cross-check every `@s5d-use-case` against the architecture-map.md use cases table.

A future `scenario-mine drift-check` subcommand could automate all three. v1 is manual: re-run `discover.sh`, diff against `index.yaml`'s `sources` lists, fix or regenerate.

## When NOT to use

- The codebase has no tests, no docs, no architecture map. There's nothing to mine — that's a discovery problem. Run the s5d Discover flow first.
- The user wants to **write new** scenarios for unimplemented behaviour. This skill extracts existing material. For greenfield scenarios, the s5d Spec phase is the right tool.
- The repo has an active, well-maintained BDD suite with up-to-date `.feature` files. Targeted updates are cheaper than full re-mining.

## Layout

```
~/.agents/skills/scenario-mine/
├── SKILL.md                       # this file
├── scripts/
│   ├── discover.sh                # JSON: tests, features, docs, API routes, s5d artefacts
│   ├── extract-tests.sh           # JSON: test names per file:line (9 langs)
│   ├── extract-docs.sh            # JSON: markdown headings + checklists
│   └── validate.sh                # Gherkin sanity check
├── templates/
│   ├── feature.gherkin            # canonical .feature shape with tag contract
│   ├── s5d-acceptance.yaml        # snippet for s5d feature spec acceptance field
│   └── index.yaml                 # generated map shape
└── references/
    └── catalog.md                 # BDD frameworks per stack + s5d integration notes
```
