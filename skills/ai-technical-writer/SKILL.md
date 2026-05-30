---
name: ai-technical-writer
description: "Organize a project's documentation so it lives in one place, stays consistent, is cross-linked, and is machine-readable. Stack-agnostic. Inventories every markdown doc, finds the mess (scattered docs with no canonical home, missing llms.txt, no front-matter, broken cross-references, orphan docs unreachable by navigation, inconsistent filenames), and emits a reorganization plan PLUS generated machine-readable artifacts (llms.txt + docs index) from the current inventory. Every finding pairs a Fix with a Validate. Read-only by default. Use for: 'organize the docs', 'consolidate documentation', 'generate llms.txt', 'doc index', 'fix broken doc links', 'machine-readable docs', 'упорядочить документацию', 'свести доки в одно место'."
argument-hint: "[--save | generate --apply]"
---

# ai-technical-writer

Documentation architect. Cluster pattern: `detect → analyze → report` (+ `generate`).
Goal: docs in **one place**, **consistent**, **cross-linked**, **machine-readable**.
Read-only by default; the one mutating path (`generate.sh --apply`) is explicit.

**Distinct from `code-quality`.** code-quality lints markdown *syntax* (markdownlint).
This skill is about documentation *architecture*: where docs live, whether they're
reachable, whether a machine can index them. Different concern, no overlap.

## Checks

| Check | Doc smell |
|---|---|
| no-single-home | docs scattered across many dirs, no `docs/` index / canonical entry |
| no-llms-txt | no machine-readable `llms.txt` for agents/LLMs |
| no-frontmatter | docs lack YAML front-matter (no machine metadata: title/status/owner/updated) |
| broken-xref | markdown links point to files that don't exist |
| orphan-docs | docs not linked from any other doc — unreachable by navigation |
| naming-inconsistency | filenames mix conventions (UPPER_SNAKE vs kebab-case) |

## What `generate.sh` produces

From the live doc inventory, it writes two machine-readable artifacts:
- **`llms.txt`** — the [llms.txt standard](https://llmstxt.org): H1 title, a summary
  blockquote, then curated sections of `- [title](path): desc` links.
- **`docs/index.md`** — a table of contents linking every doc, grouped by directory.

Default writes to `test-reports/docs/` (staging — review first). `--apply` writes to
repo root `llms.txt` and `docs/index.md`. Either way it READS existing docs, never
rewrites their content — descriptions are stubs for a human to curate.

## Severity model

`high` = a reader/agent can't find or trust the docs (no home, no machine index, broken
links). `medium` = missing machine-metadata or unreachable docs. `low` = cosmetic
(naming). The skill **does not gate** — it produces a plan + generated scaffolding.

## Flow

```
1. scripts/detect.sh    → JSON: doc inventory + which machine-readable anchors exist
2. scripts/analyze.sh   → JSON: findings [{check,severity,path,detail,fix,validate}]
3. scripts/report.sh    → markdown reorganization plan (stdout, or --save → test-reports/docs/report.md)
4. scripts/generate.sh  → llms.txt + docs/index.md (staging, or --apply to write the repo)
```

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Doc inventory + anchor detection | ✓ `detect.sh` | — |
| Health findings + severity + fix + validate | ✓ `analyze.sh` | — |
| Reorg plan + target shape | ✓ `report.sh` | — |
| llms.txt + index generation (structural) | ✓ `generate.sh` | — |
| Writing good per-doc descriptions / summaries | — | ✓ (curate the stubs) |
| Deciding the topic taxonomy / what to merge or delete | — | ✓ |
| Rewriting doc prose for consistency / terminology | — | ✓ (with Vale, see catalog) |

## Hard rules

- **Every finding ships a `validate`** — a link-checker run, a front-matter assertion,
  a reachability walk. A doc fix you can't verify isn't emitted.
- **Read-only by default.** `report.sh`/`detect`/`analyze` change nothing;
  `generate.sh` stages to `test-reports/docs/`. Only `generate.sh --apply` writes the
  repo, and even then it never edits existing docs' content — it adds an index + llms.txt.
- **Descriptions are stubs.** Generated link descriptions are filenames as placeholders;
  a human/agent curates them before publishing. Don't ship auto-stubs as final prose.
- **One home, then links.** Prefer moving scattered docs into `docs/` and linking from
  an index over leaving them and just indexing in place — but never move files
  automatically; propose the moves, let the human apply.
- **Stay in lane.** Markdown *syntax* lint → `code-quality`. This skill owns *structure*.

## Output layout

```
.claude/skills/ai-technical-writer/
├── SKILL.md
├── scripts/{detect.sh, analyze.sh, report.sh, generate.sh}
└── references/catalog.md
```

## Worked example

```bash
bash .claude/skills/ai-technical-writer/scripts/detect.sh          # inventory
bash .claude/skills/ai-technical-writer/scripts/analyze.sh         # findings (fix+validate)
bash .claude/skills/ai-technical-writer/scripts/report.sh --save   # → test-reports/docs/report.md
bash .claude/skills/ai-technical-writer/scripts/generate.sh        # stage llms.txt + index
# review the staged artifacts, then:
bash .claude/skills/ai-technical-writer/scripts/generate.sh --apply
```

## When NOT to use

- A tiny project with one README — there's nothing to organize.
- You want prose-quality / grammar review → use Vale (see catalog); this skill owns
  structure, not style.
- Markdown rendering/lint errors → `code-quality`.
