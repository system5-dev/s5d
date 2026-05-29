---
name: domain-refactor
description: "Refactor a project against its s5d architecture map, with tests as the safety net. Scans code for boundary violations (god-components, domain leaks, missing contracts, drift, orphans), classifies each finding by test-coverage safety, generates a vertical-slice plan, applies atomic moves, and verifies via tests + use-case coverage. Anchored to .s5d/discovery/architecture-map.md and the scenario-mine / unit-tests / e2e-tests skills. Use for: 'refactor by domain', 'align code to architecture', 'fix domain boundaries', 'архитектурный рефакторинг', 'выровнять код с доменами'."
argument-hint: "[analyze | plan | apply <op> | verify | trace <path>]"
---

# domain-refactor

The bridge between **architectural intent** (`.s5d/discovery/architecture-map.md`),
**behaviour intent** (`.s5d/scenarios/*.feature`), **test coverage**
(`unit-tests` + `e2e-tests` reports), and **actual code**. The skill finds
where code does not match the intent, classifies each finding by how SAFE
it is to refactor (based on test coverage), and walks the agent through
atomic, verifiable moves.

This is **not** a magic "auto-refactor the whole codebase" tool. It surfaces
violations, drafts a vertical-slice plan, and applies one slice at a time
with a hard verification gate after each.

## Required inputs

The skill assumes the following exist (and degrades gracefully if not):

- `.s5d/discovery/architecture-map.md` — domains, capabilities, components,
  use cases, edges. Produced by the `s5d` skill in Discover mode.
- `.s5d/scenarios/*.feature` — BDD scenarios with `@s5d-use-case` tags.
  Produced by the `scenario-mine` skill.
- `test-reports/js/coverage/lcov.info` — unit-test coverage. Produced by
  the `unit-tests` skill's `run.sh`.
- `e2e/**/*.spec.*` with `@use-case:<slug>` tags. Produced by the
  `e2e-tests` skill's `write.sh`.

If the inputs aren't there, `analyze.sh` runs anyway — it just won't be
able to classify files as safe/risky/unsafe.

## Violation kinds detected

| Kind | Severity | What it means |
|---|---|---|
| `god-component` | high | File > GOD_SIZE bytes AND ≥3 exports — likely owns multiple capabilities. |
| `domain-leak` | high | Generic/supporting component imports a core-domain entity directly. (v1 stub — placeholder for static analysis to come.) |
| `capability-dup` | high | Same capability id appears twice in §2 Capabilities. |
| `drift-missing` | medium | Path listed in §5 Components, file is gone. |
| `missing-contract` | medium | Cross-domain edge in §7 with no contract column. |
| `orphan` | info | Substantial file (≥200 lines) not registered in §5 Components. |

## Safety classification (the secret sauce)

Every flagged file gets a safety class from test coverage:

| Class | Rule | Refactor stance |
|---|---|---|
| 🟢 safe | line coverage ≥ 70% (configurable) | Decompose freely; tests catch regressions. |
| 🟡 risky | coverage in [30%, 70%) OR e2e spec references it | Augment tests first, then decompose. |
| 🔴 unsafe | coverage < 30% AND no e2e spec | **STOP** — write tests before touching. |

The plan emitted by `plan.sh` puts safe slices FIRST, risky in the middle,
unsafe LAST (with a prerequisite "scaffold tests first" step).

## Flow

```
1. analyze.sh                 → JSON: violations + safety classification
2. plan.sh                    → refactor-plan.md (vertical slices, ordered by safety)
3. plan.sh --decision-spec    → .s5d/packages/decision.domain-refactor.s5d.yaml
                                (skeleton for s5d_decide; record hypotheses + decision)
4. verify.sh --capture        → baseline snapshot
5. apply.sh <op> <args>       → atomic move (map-update OR move-component)
6. verify.sh                  → compare current vs. baseline; fail if regressed
7. (repeat 5+6 per slice)
8. trace.sh <file>            → confirm file traces back to its domain/use_case
```

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Detecting violations | ✓ `analyze.sh` | — |
| Classifying safety from LCOV / e2e | ✓ `analyze.sh` | — |
| Sorting slices by safety + leverage | ✓ `plan.sh` | — |
| Generating s5d decision-spec skeleton | ✓ `plan.sh --decision-spec` | — |
| Choosing seams for decomposing a god-component | — | ✓ |
| Picking which orphans to register vs. ignore | — | ✓ |
| Defining cross-domain contracts (interface shape) | — | ✓ |
| Writing tests for unsafe files before refactor | — | ✓ |
| Performing a single atomic move | ✓ `apply.sh move-component` | — |
| Capturing baseline + comparing after move | ✓ `verify.sh` | — |
| Diagnosing a verify failure | — | ✓ |

## Hard rules

- **No refactor on top of a dirty working tree.** `apply.sh` refuses if `git status` is non-empty. Commit or stash first.
- **No refactor without a baseline.** `apply.sh move-component` refuses if `.s5d/refactor-baseline/state.txt` is absent. Run `verify.sh --capture` first.
- **No code touched in `unsafe` files.** The plan flags them; the agent (or human) writes tests first via `unit-tests/write.sh`. Only after `analyze.sh` re-classifies the file as safe/risky does the slice proceed.
- **One slice per commit.** Atomic moves keep `git bisect` useful and rollback trivial.
- **Map updated in the same commit as the code move.** Otherwise drift accumulates and `analyze.sh` flags it next time.
- **`apply.sh` does NOT auto-decompose god-components.** Decomposition is multi-step (identify seams, extract files, update barrel). The skill outputs a plan and the agent/human executes each `move-component` call.
- **Refactor ≥ 30 LOC requires an s5d decision** (per AGENTS.md). `plan.sh --decision-spec` scaffolds the decision spec; the agent fills in hypotheses and records the choice via `s5d_decide`.
- **Verify.sh `--strict` is the CI mode.** Local development can tolerate small coverage dips between steps; CI fails them hard.
- **The architecture map is the source of truth** — if the agent disagrees with the map, fix the map first (in its own slice), then refactor code to it. Don't refactor code to match agent intuition while ignoring the map.

## Output layout

```
~/.agents/skills/domain-refactor/
├── SKILL.md                       # this file
├── scripts/
│   ├── analyze.sh                 # JSON violation report + safety classification
│   ├── plan.sh                    # markdown plan + optional s5d decision spec
│   ├── apply.sh                   # map-update | move-component
│   ├── verify.sh                  # baseline + compare; CI gate
│   └── trace.sh                   # file → domain → capability → use case → e2e
├── templates/
│   ├── move-checklist.md          # pre/move/post checklist per slice
│   └── ci/github.yaml             # architecture-drift PR gate
└── references/
    └── catalog.md                 # anti-pattern playbook
```

## Worked example (on this repo)

```bash
# 1. Find violations + classify safety
bash ~/.agents/skills/domain-refactor/scripts/analyze.sh > /tmp/refactor.json
# → 6 god-components (2 unsafe: utils/http-errors.ts, CoverageDetailModal.tsx),
#   3 drift-missing, 1 missing-contract, 70 orphans

# 2. Generate the plan
bash ~/.agents/skills/domain-refactor/scripts/plan.sh --output refactor-plan.md
# → vertical-slice plan. SAFE/RISKY god-components first; UNSAFE flagged
#   with "scaffold tests via unit-tests/write.sh first" prereq.

# 3. (Optional) Open an s5d decision spec
bash ~/.agents/skills/domain-refactor/scripts/plan.sh --decision-spec
# → .s5d/packages/decision.domain-refactor.s5d.yaml — fill in hypotheses

# 4. Capture baseline before touching code
bash ~/.agents/skills/domain-refactor/scripts/verify.sh --capture
# → snapshots violation total + unit pass/coverage + e2e use-case %

# 5. Trace a candidate before moving it (sanity check)
bash ~/.agents/skills/domain-refactor/scripts/trace.sh utils/payment.ts
# → reports the domain / capabilities / use cases for the file

# 6. Apply one atomic slice
bash ~/.agents/skills/domain-refactor/scripts/apply.sh move-component \
    utils/payment.ts lib/payments/payment.ts
# → git mv + import rewrite + map row update, all in one go

# 7. Verify
bash ~/.agents/skills/domain-refactor/scripts/verify.sh
# → fails (and demands rollback) if unit tests broke OR violation count rose

# 8. Commit the slice
git add -A && git commit -m "refactor(payments): move payment.ts to lib/payments/"

# 9. Repeat 5-8 per slice
```

## When NOT to use

- Architecture map is missing or stale. Run `/s5d` discovery first to produce a map.
- Test coverage is non-existent. Run the `unit-tests` and `e2e-tests` skills first — refactoring without tests is hallucination.
- The "refactor" is actually new feature work that introduces NEW domains / capabilities. That's `/s5d` Spec territory, not domain-refactor.
- The change is < 30 LOC and touches one file. Per AGENTS.md scope rules, this skill is out-of-scope for tiny refactors.
