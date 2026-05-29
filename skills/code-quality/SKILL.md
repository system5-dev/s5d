---
name: code-quality
description: "Add or upgrade SoTA linters and formatters in this project. Stack-agnostic — detects the project's languages, composes a .pre-commit-config.yaml from per-stack templates, wires the local git hook, and optionally writes a CI workflow (GitHub Actions or GitLab CI). Use for: 'set up linters', 'add code quality', 'lint setup', 'настрой линтеры', 'линтеры в CI'."
argument-hint: "[stack list | --auto | --ci github]"
---

# code-quality

Stack-agnostic linter & formatter installer. Two surfaces: **local** (pre-commit git hook) and **CI** (GitHub Actions or GitLab CI). Both run the same hook set, so what fails locally fails on CI and vice versa.

## Supported stacks (May 2026 SoTA — see `references/catalog.md`)

| Stack | Tool | Pre-commit source |
|---|---|---|
| JS/TS | **Biome v2** (lint + format + type-aware) | local hook calls `npx @biomejs/biome` |
| Python | **Ruff** (lint + format) + optional mypy/pyright | `astral-sh/ruff-pre-commit` |
| Go | **golangci-lint v2** (100+ linters + `fmt`) | `golangci/golangci-lint` |
| Rust | clippy + rustfmt + cargo-audit/deny/machete | local hook |
| Java | **Spotless** + Google/Palantir Java Format + Error Prone + NullAway (JSpecify) | local hook via gradle/maven |
| .NET | `dotnet format` + StyleCop.Analyzers + Roslynator + SonarAnalyzer.CSharp | local hook |
| Ruby | Standard (RuboCop wrapper) | `standardrb/standard` |
| PHP | PHP-CS-Fixer + PHPStan | local hook |
| Swift | SwiftLint + SwiftFormat | `realm/SwiftLint`, `nicklockwood/SwiftFormat` |
| Kotlin | ktlint + detekt | local hook via gradle |
| Shell | shellcheck + shfmt | `koalaman/shellcheck-precommit`, `scop/pre-commit-shfmt` |
| Docker | hadolint | `hadolint/hadolint` |
| Terraform | terraform fmt + tflint + Trivy IaC | `antonbabenko/pre-commit-terraform`, `aquasecurity/trivy` |
| YAML | yamllint | `adrienverge/yamllint` |
| Markdown | markdownlint-cli2 | `DavidAnson/markdownlint-cli2` |
| SQL | sqlfluff | `sqlfluff/sqlfluff` |

Plus generic hygiene (trailing-whitespace, end-of-file, large-file detector, private-key detector) from `pre-commit/pre-commit-hooks`.

## Flow

1. **Detect.** `bash scripts/detect.sh` — deterministic JSON listing stacks + package managers.
2. **Present.** Show the user what was found. Confirm which stacks to set up (default: all detected, drop `yaml`/`markdown` if user wants less noise).
3. **Compose.** `bash scripts/setup.sh --stacks <csv> [--ci github|gitlab]` — writes `.pre-commit-config.yaml` and optionally `.github/workflows/code-quality.yml` (or `.gitlab-ci.code-quality.yml` fragment).
4. **Install deps.** For stacks whose tool ships as a project dependency (JS/Biome, Python/Ruff via uv tool, PHP), instruct the user (or, after confirmation, run the install). Tools that auto-fetch via pre-commit (Ruff hook repo, golangci-lint, Swift, Ruby, etc.) need no extra action.
5. **Verify.** `bash scripts/verify.sh` — runs `pre-commit run --all-files`. Report baseline.
6. **Report.** Files changed, hooks installed, baseline pass/fail, CI workflow location.

## Determinism boundary

- **In scripts (deterministic):** stack detection, template composition, pre-commit framework install, git hook wiring, CI workflow emission, baseline run.
- **In the agent (judgement):** existing-tooling collision handling (e.g. project already uses ESLint — propose migration or augmentation, don't silently replace), config tweaks for project quirks, choosing between alternatives where the catalog flags trade-offs (Google vs Palantir Java Format, mypy vs pyright).

## How the agent runs it

```
# Detect
bash ~/.agents/skills/code-quality/scripts/detect.sh

# Compose config + wire local hook + write CI
bash ~/.agents/skills/code-quality/scripts/setup.sh --auto --ci github

# Run baseline (after stack-specific tools are installed)
bash ~/.agents/skills/code-quality/scripts/verify.sh
```

For a narrow subset:
```
bash ~/.agents/skills/code-quality/scripts/setup.sh --stacks js,python --ci github
```

To regenerate over an existing config:
```
bash ~/.agents/skills/code-quality/scripts/setup.sh --auto --force
```

## Hard rules

- **Never silently replace an existing linter.** If `.eslintrc*`, `.prettierrc*`, `pyproject.toml [tool.black]`, etc. exist, present a migration plan first and wait for the user.
- **Never commit the generated files.** The skill writes; the user reviews; the user commits.
- **Always run `verify.sh` after setup** so the user sees what the baseline looks like before deciding whether to fix issues mass-format or commit-by-commit.
- **Honour existing CI.** If `.github/workflows/ci.yml` exists, write to `.github/workflows/code-quality.yml` (separate file) and suggest merging only if the user asks.
- **No global system installs.** Tools that need installation are either project-local (Biome via package.json, Ruff via uv tool) or wrapped by pre-commit's isolated envs. For tools with no other path (shellcheck, hadolint native, sqlfluff sometimes), document the install command — don't run `brew install` without asking.

## Layout

```
~/.agents/skills/code-quality/
├── SKILL.md                # this file
├── scripts/
│   ├── detect.sh           # JSON stack detection
│   ├── setup.sh            # compose + install pre-commit + write CI
│   └── verify.sh           # run pre-commit run --all-files
├── templates/
│   ├── header.yaml         # .pre-commit-config.yaml top
│   ├── generic.yaml        # cross-cutting hygiene hooks
│   ├── {js,python,go,rust,java,dotnet,ruby,php,swift,kotlin,
│   │    shell,docker,terraform,yaml,markdown,sql}.yaml
│   └── ci/
│       ├── github.yaml     # .github/workflows/code-quality.yml template
│       └── gitlab.yaml     # .gitlab-ci.yml fragment
└── references/
    └── catalog.md          # SoTA catalog with last-verified date
```

## When NOT to use this skill

- The project already has a working, well-maintained linter setup the team is happy with. Suggest targeted upgrades instead (e.g. ESLint 8 → 9, Ruff bump, golangci-lint v1 → v2 with `migrate`).
- The user only wants a one-off lint pass, not persistent hooks. Run the tool directly.
- The repo is multi-module with conflicting tooling per module — handle each module separately, not via root pre-commit config.
