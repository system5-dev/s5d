# scenario-mine — BDD frameworks per stack + s5d integration

**Last verified:** 2026-05-28

The skill itself is framework-agnostic — it outputs Gherkin `.feature` files that any BDD runner can consume. This catalogue helps the user pick a runner per stack after the scenarios are mined.

---

## BDD execution frameworks (per stack)

| Stack | Framework | Notes |
|---|---|---|
| JS/TS | **Cucumber.js** (`@cucumber/cucumber`) | The canonical implementation. Works with Playwright for browser steps. |
| JS/TS (Vitest-native) | **vitest-cucumber** (Andre Quintela) | If the project is on Vitest and prefers staying in one runner. |
| Python | **behave** or **pytest-bdd** | `behave` is closer to original Cucumber semantics; `pytest-bdd` integrates with pytest's fixtures. |
| Go | **godog** (cucumber/godog) | Official Cucumber port. |
| Rust | **cucumber-rs** | Async-friendly, latest releases support async/await. |
| Java | **Cucumber-JVM** | The reference implementation. |
| Kotlin | **Cucumber-JVM** + Kotlin step DSL, or **Kotest BDD** | Cucumber for traditional Gherkin; Kotest if you want behaviour DSL inside test files. |
| .NET | **Reqnroll** | Forked from SpecFlow (which is now unmaintained as of 2024-2025). Reqnroll has the active community. |
| Ruby | **Cucumber** | Where Cucumber started. |
| PHP | **Behat** | Direct Gherkin runner. |
| Swift | **XCUITest + Cucumberish** | Limited ecosystem; many iOS teams use plain XCUITest with BDD-style naming instead. |

---

## s5d integration

`scenario-mine` is the **mining** side; s5d is the **decision** side. The handshake:

### When you have a fresh project (no s5d, no scenarios)

1. Run **s5d discovery** first (`/s5d дискавери`) → produces `.s5d/discovery/architecture-map.md` with `use_cases` table.
2. Run `scenario-mine` against each use case → produces `.s5d/scenarios/<use-case-id>.feature`.
3. When you start an s5d feature spec for one of those use cases, the spec's `artifacts.use_cases[].acceptance` field can be **populated** from the corresponding `.feature` file (the agent translates Gherkin scenarios into the YAML acceptance shape in `templates/s5d-acceptance.yaml`).

### When you have s5d already

The architecture map's `use_cases` table is the **anchor**. Every Feature file `@s5d-use-case` tag MUST resolve to a use case in the map.

If `scenario-mine` finds extracted scenarios that don't fit any existing use case:
- Surface them as **`Unknowns`** for the user.
- Recommend either (a) extending the architecture map to add a new use case, or (b) discarding the scenario as out-of-scope.

### Two-way drift detection

After scenarios are written, drift can happen in three directions:

1. **Code changed, scenarios stale.** Sources referenced by `@source:<file:line>` no longer exist or no longer match. Detect by running `discover.sh` and diffing against `index.yaml`.
2. **Scenarios changed, code untested.** New scenarios added without backing test. Detect by checking `@from-test` tags vs. discovered tests.
3. **s5d use case changed, scenarios orphan.** Use case removed/renamed in architecture-map. Detect by validating every `@s5d-use-case` tag against the map.

A future `scenario-mine drift-check` subcommand could automate all three. Not in v1.

---

## Gherkin style conventions adopted here

- **One Feature per s5d use case.** No exceptions.
- **Background:** holds preconditions that apply to every scenario in the file. Keep ≤3 steps; if more, the scenarios are probably too coupled.
- **Scenario name = one sentence in declarative voice.** "Renter completes Step 3 with autofilled address." Not "test that the renter can complete step 3 when autofill works".
- **Step language: Given (state) / When (action) / Then (observable outcome).** Avoid "And/But" chains longer than 3 steps — split into multiple scenarios.
- **Tags are mandatory.** `@s5d-use-case` and (`@source` OR `@from-test`) on every scenario.
- **Data tables and Scenario Outline** for parametric behaviour (typically lifted from a parametrised existing test).

---

## Anti-patterns

- **Mining test internals.** "Then `calculatePremium` returns `{base: 100, taxes: 7}`" — that's an assertion, not a behaviour. Lift the *observable user outcome*, not the function return shape.
- **Mining UI selectors.** "Then `[data-testid='continue-btn']` is enabled" — Gherkin is a behavioural language, not a step recorder.
- **Inventing scenarios with no source.** If you can't tag `@source` or `@from-test`, the scenario is speculation. Flag `@kind:speculative` and ask the user.
- **Mining duplicate behaviour across use cases.** A login scenario lives once, in the `auth-identity` use case, not in every other use case that requires login. Background steps reference it.

---

## Re-verification checklist (every 3-6 months)

1. Has SpecFlow / Reqnroll status changed in .NET land? (As of 2026-05, Reqnroll is the active fork.)
2. Have new test runners introduced native Gherkin support that obviates Cucumber-* wrappers (e.g. Vitest)?
3. Has the s5d schema for `use_cases[].acceptance` changed shape? Update `templates/s5d-acceptance.yaml` if so.
