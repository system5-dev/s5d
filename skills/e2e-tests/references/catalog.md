# e2e-tests — SoTA framework catalogue

**Last verified:** 2026-05-28
**Re-verify if:** more than 3 months have passed, a major release of any tool,
or a new framework consistently appears in independent reviews.

Three axes — web, api, mobile. For each: primary SoTA, legacy fallback, when
to keep the fallback.

---

## WEB

**Primary: Playwright** (Microsoft). Multi-browser (Chromium/Firefox/WebKit),
auto-wait selectors, parallel runners, trace viewer, codegen, CDP debugging,
network mocking, fixtures system, built-in API testing, visual regression
via `toMatchScreenshot`. The default for new web e2e in 2026.

**Fallback: Cypress 14** (Cypress.io). Stronger time-travel debugger and
better single-domain UX, but: single-tab limitation, slower at scale, no
parallelism on the open-source plan, single-browser-runtime per spec.
Keep if existing investment is heavy and migration cost outweighs benefits.

**Legacy: Selenium 4 / WebdriverIO** — still around for browser farm /
SauceLabs-style cross-environment matrices, but loses on developer ergonomics.

Sources:
- [Playwright](https://playwright.dev/)
- [Playwright vs Cypress comparison](https://playwright.dev/docs/why-playwright)

---

## API

**Primary: Playwright API** (same toolchain as web). `test.use({ baseURL })`
+ `request` fixture, no browser. Best when the repo already runs Playwright
for web — single dependency, single config, shared fixtures.

**Alternative: Hurl** (Orange-OpenSource). Text-based HTTP testing.
`.hurl` files read like curl recipes with assertions and captures.
Language-agnostic, fast, no JS dependency. Pick if the repo is backend-only
or pen-testers / SRE share the files.

**Stack-native fallbacks:**
- Node: `supertest` (Express/Fastify ecosystem-native).
- Python: pytest + `httpx` (or `requests`).
- Go: built-in `httptest` + `net/http/httptest`.
- Java/Kotlin: REST Assured.
- Rust: `reqwest` in cargo-test.

When to prefer a stack-native runner over Playwright API: the API has no
public surface and tests live alongside unit tests; OR the team only ever
maintains one language. Otherwise, prefer Playwright API for consolidation.

Sources:
- [Playwright API testing](https://playwright.dev/docs/api-testing)
- [Hurl](https://hurl.dev/)

---

## MOBILE

**Primary: Maestro** (mobile.dev). Cross-platform — one tool covers iOS,
Android, and React Native. Declarative YAML flow files, fast feedback,
visual studio mode for flow editing, cloud runner. Acquired strong
momentum 2023-2025; SoTA for new mobile e2e in 2026.

**Fallback per platform:**
- React Native: **Detox** (Wix) — historical default, deep RN integration.
  Keep if existing Detox suites + native-bridge inspection needs.
- iOS native: **XCUITest** (Apple) — required for UI tests targeting
  XCTMetric performance assertions and deep XCUIElement introspection.
- Android native: **Espresso** + UI Automator — required for idling
  resources and fine-grained view introspection.

For top-of-funnel happy-path flows, Maestro replaces all three. For deep
platform-specific assertions, keep the native runner alongside Maestro.

Sources:
- [Maestro](https://maestro.mobile.dev/)
- [Detox](https://github.com/wix/Detox)
- [XCUITest](https://developer.apple.com/documentation/xctest/user_interface_tests)

---

## Visual regression (orthogonal)

Mentioned because it often comes up alongside e2e — but it's a separate
discipline and not handled by v1 of this skill.

| Tool | Notes |
|---|---|
| **Playwright `toMatchScreenshot`** | Built-in, free, good for light checks. |
| **Percy** (BrowserStack) | Hosted, cross-browser, parallel review UI. |
| **Chromatic** (Storybook makers) | Best for component-level diffs. |
| **Lost Pixel** | Open-source, self-hostable. |

The generated `playwright.config.ts` has a `visual` project that filters
to `@visual`-tagged tests. Use it for light visual checks. For a dedicated
visual diff workflow, pair with Percy or Chromatic.

---

## Coverage for e2e

E2E tests don't measure code coverage in the traditional sense (line/branch).
The meaningful coverage metric is **use-case coverage** — fraction of
business-level user flows that have at least one e2e spec exercising them.

This skill computes that from `.s5d/scenarios/<id>.feature` files
(scenario-mine output). The `coverage.sh` script matches `.feature` files
to `e2e/**/*.spec.*` (and `.maestro/*.yaml`) files by `@use-case:<slug>` tags
or slug literal presence.

**Threshold defaults:**
- New project: start at 30 %, raise quarterly.
- Mature project: 60 % (the skill's default CI gate).
- Compliance-critical: 80 %+, with per-domain breakouts.

**What use-case coverage does NOT measure:**
- Test quality (a passing test that asserts `true` covers the use case).
- Critical-path depth (one test of the happy path leaves edge cases
  uncovered).
- Visual / accessibility / performance regressions.

Pair use-case coverage with:
- `@critical` tag for prioritised pre-merge runs.
- Mutation testing for spec-quality signal (Stryker on the JS code under
  test; this is a unit-tests concern, surfaced here for completeness).
- Manual review of which scenarios per use case are covered.

---

## Hosted CI runners

For each platform, the generated `templates/ci/github.yaml` picks a sane
default runner:

| Axis | Runner | Note |
|---|---|---|
| Web | `ubuntu-latest` with `npx playwright install --with-deps` | Linux is cheapest; pin browser version via Playwright. |
| API | `ubuntu-latest` | Hurl native binary or Playwright API project. |
| Mobile | `macos-latest` for iOS / Maestro on iOS simulator; `ubuntu-latest` for Maestro on Android emulator | Mac runners are expensive; consider running on PR for `@critical` only. |
| Visual | `ubuntu-latest` + Docker (`mcr.microsoft.com/playwright`) | Use Microsoft's Playwright image to pin browser binary across local + CI. |

---

## Re-verification checklist (every 3-6 months)

1. Has Cypress 15+ shipped cross-browser parallelism that narrows the
   Playwright gap?
2. Maestro Cloud pricing / open-source split — has it stayed
   self-hostable?
3. Hurl 5+ — has it added GraphQL / gRPC support that broadens applicability?
4. Visual regression — does Playwright's `toMatchScreenshot` need a
   replacement for production-grade visual diff?
5. Any new "AI test writer" tool (Currents.dev, KaneAI, Reflect) that
   warrants integration with `write.sh`?
