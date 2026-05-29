---
name: e2e-tests
description: "Install SoTA e2e-test frameworks per detected axis (web/api/mobile), run tests with mandatory reports (HTML + JUnit + traces + videos on failure), and compute use-case coverage against .s5d/scenarios/. Scaffold a Playwright/Maestro spec from any s5d use case. Use for: 'set up e2e', 'add playwright', 'maestro mobile flows', 'e2e coverage report', 'настрой end-to-end', 'покрытие сценариев'."
argument-hint: "[axis list | --auto | use-case-id]"
---

# e2e-tests

Stack-agnostic e2e-test installer + runner + scaffolder + coverage reporter.
Same shape as `unit-tests` but for **end-to-end** flows. Three axes that can
coexist in one repo:

| Axis | Surface | SoTA framework (2026) | Legacy fallback |
|---|---|---|---|
| `web` | Browser-driven UI flows | **Playwright** | Cypress 14 |
| `api` | HTTP API contracts | **Playwright API** or **Hurl** | supertest, REST Assured |
| `mobile` | iOS / Android / React Native | **Maestro** | Detox, XCUITest, Espresso |

The skill detects which axes the repo has, installs configs with mandatory
reports baked in, scaffolds spec stubs, runs them, and ties results back
to `.s5d/scenarios/<use-case-id>.feature` for use-case coverage.

## What's mandatory

- **Reports in three formats** per axis: HTML (local dev), JUnit XML (CI
  annotations), traces + videos + screenshots on failure (Playwright's
  trace viewer is best-in-class). Written under `test-reports/e2e/<axis>/`.
- **Use-case coverage** report — `(s5d use cases with ≥1 matching e2e spec)
  / total`. Default gate 60 %. The `coverage.sh` script computes this from
  `.s5d/scenarios/*.feature` (scenario-mine output) and `e2e/**/*.spec.*`
  files, looking for `@use-case:<slug>` tags or slug references.
- **Traces on retry** is the Playwright default the generated config keeps.
  Pulled into CI artefacts automatically.

## Flow

```
1. scripts/detect.sh                     → JSON: axes + framework + spec counts
2. scripts/setup.sh --auto --ci github   → install configs + CI workflow
3. scripts/write.sh <use-case-id>        → scaffold spec from .s5d/scenarios/<id>.feature
4. scripts/run.sh                        → run + emit HTML/JUnit/traces/videos
5. scripts/coverage.sh                   → use-case coverage report (text/json/md)
```

## s5d integration

`write.sh` reads `.s5d/scenarios/<use-case-id>.feature` (scenario-mine
output) and emits a Playwright spec with one `test()` per Scenario,
tagged `@use-case:<slug>` so `coverage.sh` can find it later. The link
between Gherkin scenario and e2e implementation is mechanical, not
narrative — drift in either side is detectable.

The coverage gate becomes: **of the 13 use cases the project has, how
many are exercised by e2e?** A failing PR with a new use case but no
spec drops the percentage and triggers the gate.

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Axis + framework detection | ✓ `detect.sh` | — |
| Config generation with base-URL / threshold substitution | ✓ `setup.sh` | — |
| Spec scaffolding from .feature → Playwright spec | ✓ `write.sh` | — |
| Running tests + reports | ✓ `run.sh` | — |
| Use-case coverage computation | ✓ `coverage.sh` | — |
| Picking framework when both Playwright + Cypress applicable | — | ✓ |
| Filling TODO assertions in the stub with real selectors | — | ✓ |
| Choosing which scenarios warrant `@critical` tag | — | ✓ |
| Interpreting coverage gaps to prioritise work | — | ✓ |

## Hard rules

- **Three report formats are non-negotiable.** HTML + JUnit + (traces |
  videos | screenshots on failure). Override the threshold (`--threshold 0`)
  but the reports always run.
- **Tests must pin baseURL via env var.** Hard-coded `http://localhost:3000`
  in a test fails review — set `PLAYWRIGHT_BASE_URL` in setup, CI, and
  test wrappers.
- **No data leakage between runs.** Generated specs include unique
  identifiers (timestamps, UUIDs) in user emails / IDs so concurrent CI
  runs don't collide on shared environments.
- **`@critical` tag for top-of-funnel flows.** Used by the run script's
  `--grep @critical` mode for faster pre-merge gates. The agent picks
  which scenarios get the tag.
- **Auth scenarios MUST be explicit.** If the use case requires login,
  the spec uses the auth flow (not a backdoor token) at least once,
  even if subsequent tests use storage state.
- **Payment / PII scenarios MUST run on a stub/test mode.** No live
  payment processor, no real PII — Stripe test cards, Twilio sandbox,
  etc. The agent confirms test-mode env vars are set before writing
  payment specs.
- **Stubs fail by default.** `write.sh` produces `expect.fail("TODO")`
  so unfilled tests cannot mask gaps in coverage.
- **Honour existing CI.** Generated to `.github/workflows/e2e-tests.yml`
  (separate from any unit-tests.yml or code-quality.yml).

## Output layout

```
~/.agents/skills/e2e-tests/
├── SKILL.md                          # this file
├── scripts/
│   ├── detect.sh                     # JSON axes + framework detection
│   ├── setup.sh                      # install configs + CI workflow
│   ├── write.sh                      # scaffold spec from .s5d/scenarios/<id>.feature
│   ├── run.sh                        # run + emit HTML + JUnit + traces
│   └── coverage.sh                   # use-case coverage report (text/json/md)
├── templates/
│   ├── web/      { install.md, playwright.config.ts, stub.e2e.spec.ts }
│   ├── api/      { install.md, stub.hurl, stub.api.spec.ts }
│   ├── mobile/   { install.md, flow.yaml }
│   └── ci/       { github.yaml, gitlab.yaml }
└── references/
    └── catalog.md                    # SoTA references + last verified date
```

## Worked example

```bash
# 1. What axes are here?
bash ~/.agents/skills/e2e-tests/scripts/detect.sh
# → web (next.js + playwright, 3 specs), api (next-api, 0 specs)

# 2. Install configs (defaults: base url http://localhost:3000)
bash ~/.agents/skills/e2e-tests/scripts/setup.sh --auto --ci github \
     --base-url http://localhost:3001

# 3. Scaffold an e2e spec from a use case mined by scenario-mine
bash ~/.agents/skills/e2e-tests/scripts/write.sh apply-for-event-insurance
# → writes e2e/apply-for-event-insurance.spec.ts with one test() per Scenario,
#   each tagged @use-case:apply-for-event-insurance

# 4. Fill in TODOs, then run
bash ~/.agents/skills/e2e-tests/scripts/run.sh

# 5. Use-case coverage report
bash ~/.agents/skills/e2e-tests/scripts/coverage.sh
# → covered: 1 / 13 (7%)
#   ✓ apply-for-event-insurance  e2e/apply-for-event-insurance.spec.ts
#   ✗ smart-fill-autopopulate    domain=smart-prefill
#   ...
```

CI gate (in generated workflow): `coverage.sh --threshold 60`.

## When NOT to use

- The project is library code with no user-facing surface (npm package,
  Rust crate, language tooling). Unit tests cover everything; e2e adds
  no signal.
- You need load / soak / chaos tests. k6 / Artillery / Locust are
  performance, not e2e.
- You need visual regression as primary metric. Use Percy / Chromatic /
  Lost Pixel. (Playwright's built-in `toMatchScreenshot` is good for
  light visual checks; not a substitute for a dedicated visual diff tool.)
