# unit-tests — SoTA framework catalogue

**Last verified:** 2026-05-28
**Re-verify if:** more than 3 months have passed, a framework reaches a new major
release, or a new framework consistently appears in independent reviews / surveys.

This catalogue justifies the choices baked into `templates/<stack>/`. Each
section names the SoTA option, the legacy fallback, and when to pick the
fallback.

---

## JS / TypeScript

**Primary: Vitest 4** (released October 2025). Native ESM, esbuild-powered
TypeScript, V8 coverage, 5-50× faster than Jest on real suites, ~57 % lower
memory, Vite plugin reuse, plugin system, browser mode for component tests.
Default choice for any new JS/TS project in 2026; Angular 21 adopted it as
default in late 2025, surpassed Jest in State of JS satisfaction.

**Fallback: Jest 30** (June 2025). Keep if:
- React Native project (Jest is the only officially-supported runner).
- Large legacy monorepo with custom moduleNameMapper / transform pipeline
  that doesn't translate cleanly to Vite's resolve.alias.
- Heavy investment in Jest-specific plugins (jest-axe, jest-mock-axios,
  custom reporters) without Vitest equivalents.

**Coverage tool:** Vitest uses V8 coverage natively (or istanbul if
configured). Jest uses istanbul. V8 is faster and more accurate but lacks
some edge-case reporting istanbul has.

Sources:
- [Vitest 4 announcement](https://vitest.dev/blog/vitest-v4)
- [Vitest vs Jest comprehensive comparison](https://devtoolswatch.com/en/vitest-vs-jest-2026)
- [Jest 30 release notes](https://jestjs.io/blog/2025/06/04/jest-30)

---

## Python

**Primary: pytest** + pytest-cov + coverage.py. Pytest's fixture system,
parametrisation, plugin ecosystem (pytest-asyncio, pytest-mock, hypothesis)
and clearer assertion failures make it the universal default. coverage.py
is the de-facto coverage engine; pytest-cov is the integration.

**Fallback: unittest** (stdlib). Only when external dependencies are forbidden.

**Notable extension:** hypothesis for property-based testing. Adds a small
binary dependency but catches edge cases unit tests never think to write.

Sources:
- [pytest docs](https://docs.pytest.org/)
- [coverage.py 7.x](https://coverage.readthedocs.io/)

---

## Go

**Primary: built-in `go test`** + `go test -cover` + `go tool cover`. The
Go team treats testing as a first-class feature of the language, so the
toolchain ships with everything. No "framework" choice to make.

**Coverage export:**
- `gcov2lcov` to convert `coverage.out` → LCOV (Codecov, VS Code).
- `go-junit-report` to convert `go test -v` output → JUnit XML for CI.

**Notable extensions** (optional):
- `testify` for richer assertions (`assert.Equal` instead of manual `if`).
- `gomock` for interface mocking.

Sources:
- [Go testing docs](https://pkg.go.dev/testing)

---

## Rust

**Primary: built-in `cargo test`** for execution + **cargo-llvm-cov** for
coverage. LLVM source-based instrumentation tracks region-level coverage
(more accurate than line-level), runs on Linux/macOS/Windows, supports
LCOV/HTML/Cobertura output, has built-in threshold gating.

**Fallback: cargo-tarpaulin**. Keep if:
- You rely on `should_panic` doc tests or `--no-fail-fast` for coverage
  signal — tarpaulin's ptrace backend captures these where LLVM doesn't.
- You're on Linux x86_64 only and the existing setup works.

Tarpaulin on macOS/Windows now uses the LLVM engine by default — the
practical gap has narrowed.

**Notable extensions:**
- `rstest` for parametric tests.
- `proptest` / `quickcheck` for property-based testing.
- `mockall` for trait mocking.

Sources:
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
- [Rust Project Primer — Coverage](https://rustprojectprimer.com/measure/coverage.html)

---

## Java

**Primary: JUnit 5 (Jupiter)** + **JaCoCo**. Same combination since ~2019;
remains the SoTA. JUnit 5 brought parameterised tests, nested classes,
extension model, async support. JaCoCo offers Maven + Gradle plugins,
threshold rules, XML / HTML / CSV / Cobertura output, ratchet mode.

**Notable extensions:**
- Mockito for mocking.
- AssertJ for fluent assertions.
- ArchUnit for architecture tests.
- Pitest for mutation testing (sanity-check coverage quality).

Sources:
- [JUnit 5](https://junit.org/junit5/)
- [JaCoCo 0.8.x](https://www.jacoco.org/jacoco/)

---

## Kotlin

**Primary: JUnit 5 + Kover**. Kover is JetBrains' own coverage tool, built
specifically for the Kotlin compiler. Better Kotlin DSL ergonomics than
JaCoCo, native support for sealed classes, inline functions, suspend
functions. Generates XML / HTML / verification rules.

**Alternative test framework: Kotest** if you want behaviour-DSL style
(`should "do X" { }`) instead of JUnit annotations.

**Fallback: JaCoCo** if the project is mostly Java with some Kotlin.

Sources:
- [Kover](https://github.com/Kotlin/kotlinx-kover)
- [Kotest](https://kotest.io/)

---

## .NET / C\#

**Primary: xUnit** + **coverlet** (the `coverlet.collector` NuGet ships with
`dotnet test --collect:"XPlat Code Coverage"`). xUnit's parallelisation
defaults, theory data sources, and async support beat NUnit and MSTest
for new code. Coverlet emits Cobertura / LCOV / OpenCover / JSON.

**Fallback: NUnit** for projects already on it; **MSTest** for
Microsoft-shop projects with tooling investment.

**Threshold enforcement:** coverlet's `<Threshold>` in runsettings; combine
with `coverlet.msbuild` for in-build coverage.

Sources:
- [xUnit](https://xunit.net/)
- [coverlet](https://github.com/coverlet-coverage/coverlet)

---

## Ruby

**Primary: RSpec** + **SimpleCov**. RSpec dominates new Rails / Ruby
projects; readable spec DSL, rich matcher library, parallel execution.
SimpleCov is the only coverage tool of consequence in the Ruby ecosystem.

**Alternative: Minitest** (stdlib) for projects that want zero dev-deps.

Sources:
- [RSpec](https://rspec.info/)
- [SimpleCov](https://github.com/simplecov-ruby/simplecov)

---

## PHP

**Primary: PHPUnit 11+** + Xdebug or PCOV for coverage driver. PCOV is
2-3× faster than Xdebug for coverage but Xdebug doubles as the debugger
many teams already use. PHPUnit's `<coverage>` block emits Cobertura,
Clover, HTML.

**Threshold check:** PHPUnit doesn't fail on threshold natively; use
`rregeer/phpunit-coverage-check` as a post-step.

**Alternative: Pest** if your team prefers the Jest-style describe/it
syntax over xUnit-style.

Sources:
- [PHPUnit 11 release notes](https://phpunit.de/announcements/phpunit-11.html)
- [Pest](https://pestphp.com/)

---

## Swift

**Primary: Swift Testing** (Apple, introduced WWDC 2024, default for
Swift 6 in 2026) for **unit tests**. Macro-based, parallel-by-default,
two universal assertions (`#expect`, `#require`), structs over class
inheritance, parametric tests built in, native async.

**Keep XCTest for:** UI tests (XCUIApplication), performance tests
(XCTMetric). Both frameworks coexist in one test target.

**Coverage:** `swift test --enable-code-coverage` or `xcodebuild test
-enableCodeCoverage YES`. Export to LCOV via `xcrun llvm-cov export`.

Sources:
- [Swift Testing](https://github.com/apple/swift-testing)
- [Migrating XCTest to Swift Testing](https://useyourloaf.com/blog/migrating-xctest-to-swift-testing/)

---

## Shell

**Primary: bats-core** (Bash Automated Testing System, the active fork).
Lightweight, Bash 3.2 compatible, has bats-support and bats-assert helper
libs. Common in CI pipelines, package distributions, dotfile repos.

**Coverage:** kcov, optional. Run `kcov --include-path=$(pwd) coverage bats *.bats`.

Sources:
- [bats-core](https://github.com/bats-core/bats-core)
- [kcov](https://github.com/SimonKagstrom/kcov)

---

## Cross-cutting

### Report format conventions

| Format | Consumed by |
|---|---|
| **LCOV** | Codecov, Coveralls, VS Code "Coverage Gutters" extension |
| **Cobertura XML** | GitLab CI (inline MR annotations), Codecov, Bamboo |
| **JUnit XML** | GitHub Actions PR annotations (via dorny/test-reporter), GitLab CI, Jenkins |
| **HTML** | Local dev (`open test-reports/<stack>/coverage/index.html`) |
| **Console summary** | CI logs, terminal feedback |

The skill emits all four where the framework supports it.

### Coverage thresholds — choosing a number

- **0–50 %**: a stake in the ground. Use only when starting from a
  greenfield codebase and tightening over time.
- **70 %**: the default this skill uses. Catches the obvious "no tests
  for module X" failure modes without forcing trivial-test grinding.
- **80 %**: target for teams that have a habit of test-first development.
- **90 %+**: critical-path or compliance-driven code. Mostly aspirational
  for general application code.

Per-file thresholds matter more than global thresholds — 90 % global
average can hide a critical file at 20 %. Most native configs support
per-file gates; the skill turns them on in `simplecov.rb` and lets the
others default to global only.

### Mutation testing

Not in v1. Coverage measures whether a line was *executed*; mutation
testing (PIT / Stryker / mutmut / cargo-mutants) measures whether
assertions are strong enough to catch a bug. Re-verify if mutation tools
become as fast and ergonomic as line coverage.

---

## Re-verification checklist (every 3-6 months)

1. Vitest vs Jest market share — has Jest 31 shipped with native ESM
   and closed the speed gap?
2. Swift Testing maturity — has it absorbed UI testing or stayed
   unit-only?
3. Has cargo-tarpaulin matched cargo-llvm-cov on accuracy?
4. PHPUnit 12 / Pest market share — is the Jest-style DSL eating share?
5. Pipeline of "AI test generation" tools (CodiumAI, Tabnine, etc.) —
   does one warrant integration into `write.sh`?
6. Any new coverage report format gaining traction at Codecov / GitHub?
