# SoTA Linter Catalog

**Last verified:** 2026-05-28
**Re-verify if:** more than 3 months have passed, a new major release of any listed tool, or a new linter consistently appears in top-of-feed comparisons.

This is the reference behind the template files in `../templates/`. Each entry: tool + role + why it's SoTA + replacement notes if you're migrating.

---

## JS / TypeScript

**Primary: Biome v2 ("Biotype", June 2025)** — a single Rust-based binary that lints, formats, and (as of v2) does type-aware linting without invoking `tsc`. Replaces ESLint + Prettier + most of typescript-eslint.

- **Speed:** 15-50× faster than ESLint on 10k-file repos (parses once, multiple passes on one AST, uses all CPU cores).
- **Type-awareness:** v2 catches floating-promises at ~75% of typescript-eslint's coverage, at a fraction of the cost.
- **Plugins:** GritQL-based custom rules (introduced v2.0).
- **Single config:** `biome.json` replaces `.eslintrc.*` + `.prettierrc.*`.

**Fallback: ESLint 9 (flat config) + Prettier 3 + typescript-eslint** — pick this only when (a) team has heavy investment in custom ESLint plugins or (b) full type-aware linting via `tsc` is non-negotiable.

**Sources:** [Biome v2 announcement](https://biomejs.dev/blog/biome-v2/), [Biome migration guide](https://dev.to/pockit_tools/biome-the-eslint-and-prettier-killer-complete-migration-guide-for-2026-27m).

---

## Python

**Primary: Ruff (Astral)** — Rust-based, replaces flake8 + isort + pylint + black + pyupgrade + several others. Single binary, single config (`pyproject.toml [tool.ruff]`).

- Linter (`ruff check`) and formatter (`ruff format`) in one.
- ~150× faster than the toolchain it replaces.
- Drop-in compatible with most flake8/pylint rule sets.

**Type checking:** pick one
- **pyright** — fast, IDE-aligned (used in VS Code Pylance), strict mode catches most.
- **mypy** — older, more mature plugin ecosystem, still the default for typing-heavy codebases.

For new projects: Ruff + pyright. For existing mypy-driven projects: keep mypy and add Ruff.

**Sources:** [Astral Ruff](https://docs.astral.sh/ruff/), upstream pre-commit hook: `astral-sh/ruff-pre-commit`.

---

## Go

**Primary: golangci-lint v2 (released March 2025, latest v2.7.2 Dec 2025)** — bundles 100+ linters. Major v2 changes: revamped config (`linters.default: fast|all`), new `fmt` subcommand (replaces separate gofumpt/gofmt invocations), `migrate` command for v1→v2 conversion.

- Single config: `.golangci.yaml`.
- Recommended linters to enable by default: `errcheck`, `gosimple`, `govet`, `ineffassign`, `staticcheck`, `unused`, `revive`, `gofmt`, `goimports`, `misspell`, `prealloc`.
- For security: `gosec`.

**Migration from v1:** run `golangci-lint migrate` — generates v2 config automatically.

**Sources:** [golangci-lint v2 release](https://ldez.github.io/blog/2025/03/23/golangci-lint-v2/), [official changelog](https://golangci-lint.run/docs/product/changelog/).

---

## Rust

**Primary: built-in `clippy` + `rustfmt`** (both ship with `rustup component add`). No third-party tool needed for lint/format.

- `clippy --all-targets --all-features -- -D warnings` as the strict gate.
- `rustfmt` is opinionated; configure via `rustfmt.toml` if needed (e.g. `max_width = 100`).

**Supply chain (add as separate hooks):**
- `cargo audit` — RustSec advisories (installed via `cargo install cargo-audit`).
- `cargo deny` — license + dependency policy.
- `cargo machete` — find unused dependencies.

---

## Java

**Primary: Spotless + Google or Palantir Java Format + Error Prone + NullAway**

- **Spotless** (Diffplug) — formatter orchestrator with **ratchet mode** (only checks files changed vs a git ref). Gradle plugin: `com.diffplug.spotless`. Maven plugin: `com.diffplug.spotless:spotless-maven-plugin`.
- **Formatter inside Spotless:** Google Java Format (strict, AOSP) or Palantir Java Format (120-char, lambda-friendly). Palantir for modern Java, Google for AOSP/Android-style codebases.
- **Error Prone** (Google) — compile-time static analysis. Hooks into `javac` as an annotation processor. Catches API misuse, deprecated patterns, type-mismatched collection ops.
- **NullAway** (Uber) — Error Prone plugin. Eliminates NPEs with <10% build overhead. **JSpecify** (`@NullMarked`, `@Nullable`) is the emerging standard; Guava 33.4.1+ adopted it.

**Build wiring (Gradle Kotlin DSL):**
```kotlin
plugins {
    id("com.diffplug.spotless") version "7.0.0"
    id("net.ltgt.errorprone") version "4.1.0"
}
spotless {
    java {
        palantirJavaFormat()
        targetExclude("**/generated/**")
        ratchetFrom("origin/main")
    }
}
dependencies {
    errorprone("com.google.errorprone:error_prone_core:2.36.0")
    errorprone("com.uber.nullaway:nullaway:0.12.2")
}
tasks.withType<JavaCompile>().configureEach {
    options.errorprone.nullaway {
        annotatedPackages.add("com.your.company")
    }
}
```

**Migration: legacy codebases run `./gradlew spotlessApply` in a single large commit, then enable ratchet mode going forward.**

**Sources:** [Spotless](https://github.com/diffplug/spotless), [NullAway](https://github.com/uber/NullAway), [Palantir Java Format](https://github.com/palantir/palantir-java-format).

---

## .NET / C#

**Primary: `dotnet format` (SDK 6+ built-in) + Roslyn analyzers (NuGet packages)**

- **`dotnet format`** — reads `.editorconfig`, applies formatting. Built into `dotnet` CLI.
- **StyleCop.Analyzers** (NuGet) — style enforcement (SA-prefix rules). Add to every project via `Directory.Build.props`.
- **Roslynator** (NuGet) — additional analyzers, refactorings, fixes. Lightweight.
- **SonarAnalyzer.CSharp** (NuGet) — bug/vulnerability/code-smell detection from Sonar's ruleset (free for OSS).

**`Directory.Build.props` template:**
```xml
<Project>
  <ItemGroup>
    <PackageReference Include="StyleCop.Analyzers" Version="*" PrivateAssets="all" />
    <PackageReference Include="Roslynator.Analyzers" Version="*" PrivateAssets="all" />
    <PackageReference Include="SonarAnalyzer.CSharp" Version="*" PrivateAssets="all" />
  </ItemGroup>
  <PropertyGroup>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
    <EnforceCodeStyleInBuild>true</EnforceCodeStyleInBuild>
    <AnalysisLevel>latest-recommended</AnalysisLevel>
  </PropertyGroup>
</Project>
```

**Note:** StyleCop's default ruleset is aggressive. Disable rules that don't add value (start with SA1200, SA1101).

**Sources:** [dotnet format docs](https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-format), [Roslynator](https://github.com/dotnet/roslynator).

---

## Ruby

**Primary: Standard** — zero-config wrapper around RuboCop with sensible defaults. Use when you don't want to bikeshed style.

**Alternative:** full RuboCop with `rubocop-performance`, `rubocop-rails`, `rubocop-rspec`. Pick this when the team needs custom rules.

---

## PHP

**Primary: PHP-CS-Fixer + PHPStan**

- **PHP-CS-Fixer** — formatting + style (replaces phpcs/phpcbf for new projects).
- **PHPStan** — static analysis. Level 9 is the strictest; start at level 5 for legacy code and ratchet up.

**Alternative:** **Psalm** (Vimeo) — similar to PHPStan, stronger on Laravel/Symfony integrations.

---

## Swift

**Primary: SwiftLint + SwiftFormat (Nick Lockwood)**

- **SwiftLint** (Realm) — lint with extensive Swift-style rules.
- **SwiftFormat** (Nick Lockwood) — opinionated formatter.

Apple's own `swift-format` (built into Swift toolchain) is gaining ground; consider it for projects that want zero extra deps.

---

## Kotlin

**Primary: ktlint + detekt**

- **ktlint** (Pinterest) — formatter + minimal linter. Gradle plugin: `org.jlleitschuh.gradle.ktlint`.
- **detekt** — static analyzer with broader rule set than ktlint. Gradle plugin: `io.gitlab.arturbosch.detekt`.

Run both: ktlint handles style, detekt handles bugs / complexity / smells.

---

## Shell

**Primary: shellcheck + shfmt**

- **shellcheck** — bash/sh static analysis. The canonical shell linter.
- **shfmt** (Mateusz Vellingiri) — bash formatter. `-i 2 -ci -bn` for 2-space, case-indent, binary-on-next-line.

---

## Docker

**Primary: hadolint** — Dockerfile linter. Catches anti-patterns (apt-get without `--no-install-recommends`, missing pinned versions, USER-as-root, etc.).

---

## Terraform / OpenTofu

**Primary: `terraform fmt` + tflint + Trivy (IaC mode)**

- **`terraform fmt`** — built into Terraform CLI. Canonical formatter.
- **tflint** — Terraform-aware linter with provider-specific rules (AWS, GCP, Azure plugins).
- **Trivy** (Aqua) — replaced **tfsec** in 2024. Scans IaC for security issues (CIS, NIST, custom). Use `trivy config <path>` mode.

**Sources:** [tflint](https://github.com/terraform-linters/tflint), [Trivy](https://github.com/aquasecurity/trivy).

---

## YAML

**Primary: yamllint** — readable, well-configured. Sensible default rules: line-length 120, indentation, no-trailing-spaces.

---

## Markdown

**Primary: markdownlint-cli2 (David Anson)** — fast linter with auto-fix. Config: `.markdownlint.jsonc` at repo root.

---

## SQL

**Primary: sqlfluff** — multi-dialect (postgres, mysql, sqlite, snowflake, bigquery, redshift, mssql, oracle, etc.) linter + formatter. Set `--dialect <name>` per project.

---

## Orchestration

**pre-commit framework (Yelp)** — the cross-stack orchestrator used by this skill. Each hook runs in its own isolated environment (Python venv, Node sandbox, Docker, etc.), so adding a Ruff hook doesn't conflict with a Biome hook. Mature, ubiquitous, supports `--from-ref`/`--to-ref` for diff-only runs in CI.

**Alternative considered, not adopted:**
- **lefthook** (Evil Martians, Go) — faster orchestrator. Drop-in replacement candidate; not chosen because pre-commit's hook ecosystem is broader.
- **trunk-check** (Trunk.io) — meta-runner that bundles linters per language. Convenient but introduces a vendor dependency and a parallel config.
- **husky / lint-staged** (JS only) — too JS-specific for a polyglot skill.

---

## Re-verification checklist (every 3-6 months)

1. Are any tools listed here deprecated or marked end-of-life?
2. Has a 1.0+ release replaced an alpha or beta listed here?
3. Has a new linter consistently appeared in independent reviews / surveys?
4. Are pre-commit hook revs (e.g. `v0.8.4` for Ruff) outdated by more than 3 minor versions? If so, bump in the template files.
5. Has the language ecosystem itself changed the canonical formatter (e.g. Swift adopting swift-format over SwiftLint as the recommended default)?
