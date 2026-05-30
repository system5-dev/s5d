# ai-technical-writer — tooling catalog

**last_verified: 2026-05-30**

The skill's detection/generation is self-contained (no install). This catalog lists the
SoTA tools used to **validate** and **enforce** the doc structure once it's organized.

## Machine-readable docs

| Standard / tool | Use |
|---|---|
| **llms.txt** (llmstxt.org) | the convention this skill targets: `/llms.txt` = title + summary + curated links so agents/LLMs can index a project. Pair with `/llms-full.txt` for inlined content. |
| **YAML front-matter** | per-doc machine metadata (title, status, owner, last_reviewed, tags) — lets the index and staleness checks be programmatic. |
| **JSON Schema for front-matter** | validate the front-matter block shape in CI. |

## Link integrity (the `validate` side of broken-xref / orphan)

| Tool | Use | Install |
|---|---|---|
| **lychee** | fast async link checker (internal + external), CI-friendly | `brew install lychee` |
| **markdown-link-check** | per-file md link checker | `npm i -g markdown-link-check` |
| **remark-validate-links** | resolves relative links + heading anchors across the tree | `npm i -D remark-cli remark-validate-links` |

## Consistency / terminology (prose, agent's job — not this skill's heuristics)

| Tool | Use |
|---|---|
| **Vale** | prose linter with a project style + terminology vocab (enforce ubiquitous language in docs) — `brew install vale` |
| **textlint** | pluggable natural-language lint (terminology, no-dead-links, write-good) |
| **markdownlint** | markdown *syntax* — owned by the `code-quality` skill, listed here for the boundary |

## Single-home rendering (optional)

| Tool | Use |
|---|---|
| **MkDocs (+ Material)** | render `docs/` into a searchable site; `mkdocs.yml` nav = the index made canonical |
| **Docusaurus** | docs site with versioning + front-matter-driven sidebar |
| **doctoc** | auto-generate/refresh a TOC inside a doc |

## TOC / index automation

- **doctoc** (`npm i -g doctoc`) — inject a `<!-- TOC -->` into long docs.
- This skill's `generate.sh` builds the cross-doc index + llms.txt; MkDocs/Docusaurus
  nav can then consume the same structure.

## Workflow this skill recommends

1. `analyze` → see the mess (scatter, orphans, broken links, no llms.txt).
2. Agree a `docs/` taxonomy; move scattered topic docs in (human-applied).
3. `generate --apply` → llms.txt + docs/index.md; curate the stub descriptions.
4. Wire **lychee** (link integrity) + a front-matter assertion + **Vale** (terminology)
   into CI — these are the `validate` steps made permanent so docs can't rot back.

## Notes

- Re-run the agentic research behind this list if `last_verified` is > ~12 months stale
  (llms.txt + Vale + lychee are active projects).
- Auto-generated link descriptions are placeholders. Never publish an llms.txt whose
  descriptions are just filenames — the value is the curated one-line summaries.
