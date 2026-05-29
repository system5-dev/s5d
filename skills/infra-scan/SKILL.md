---
name: infra-scan
description: "Analyze a repo's infrastructure & deployment posture — stack-agnostic. Detects deploy surfaces (Dockerfiles, compose, helm, k8s, terraform, CI pipelines, PaaS configs, env files) and reports OPERATIONAL findings: deploy-plane fragmentation/drift, container hardening (pinned base, multistage, non-root, healthcheck), helm/k8s resource+probe sizing, CI deploy-without-test-gate, and env-var drift (used-in-code vs declared). Read-only by default. Use for: 'analyze infrastructure', 'deployment posture', 'infra audit', 'env var drift', 'topology check', 'проверь инфраструктуру', 'аудит деплоя'."
argument-hint: "[--save | --output PATH]"
---

# infra-scan

Infrastructure & deployment **posture** analyzer. Same shape as the rest of the
cluster: `detect → analyze → report`. Read-only — it reads deploy configs, it
never writes them.

**Scope boundary vs `security-scan`:** security-scan reads Dockerfiles/helm/IaC for
**CVEs and security misconfig** (trivy config, checkov). infra-scan reads the *same
files* for **operational posture** — topology, drift, resource sizing, probes,
env-var flow. They are complementary, not overlapping. A `:latest` base is an
infra finding (non-reproducible build); a known-CVE base is a security finding.

## What it reports (8 surfaces)

| Surface | Operational checks |
|---|---|
| containers | pinned base (no `:latest`), multistage, non-root `USER`, `HEALTHCHECK`, `.dockerignore` |
| compose | service topology (detected; deep checks v1.1) |
| helm | resource requests+limits, liveness/readiness probes, replicaCount |
| k8s | manifest presence + kind inventory (deep checks v1.1) |
| terraform | presence (posture checks v1.1) |
| ci-cd | deploy-without-test-gate, multi-pipeline fragmentation |
| paas | vercel/fly/render/netlify runtime config presence |
| env | **drift**: vars used in code (`process.env` / `os.getenv` / `os.Getenv`) but absent from `.env.example` |

## Severity model

`high` = drift/risk that breaks prod or reproducibility (unpinned base, undeclared
env vars, deploy without test gate, split deploy planes). `medium` = missing
guardrail (runs-as-root, no probes). `low` = best-practice (no healthcheck, no
multistage, no .dockerignore). `info` = topology notes. The skill **does not gate**
a build — it produces a posture report; the human decides what to act on.

## Flow

```
1. scripts/detect.sh   → JSON: which of 8 deploy surfaces are present + evidence paths
2. scripts/analyze.sh  → JSON: operational findings [{surface,kind,severity,path,detail}]
3. scripts/report.sh   → markdown posture report (stdout, or --save → test-reports/infra/report.md)
```

## Determinism boundary

| Step | In script | In agent |
|---|---|---|
| Surface detection (Dockerfile/helm/CI/paas/env) | ✓ `detect.sh` | — |
| Posture findings + severity per surface | ✓ `analyze.sh` | — |
| Env used-vs-declared drift computation | ✓ `analyze.sh` | — |
| Markdown rendering | ✓ `report.sh` | — |
| Deciding which findings to fix vs accept | — | ✓ |
| Resolving split-deploy-planes (consolidate vs keep) | — | ✓ |
| Writing the remediation / ADR | — | ✓ (hand to `system-design`) |

## Hard rules

- **Read-only.** No script writes into deploy configs. `report.sh --save` writes
  only to `test-reports/infra/`. Never edit a Dockerfile/helm/CI you were asked to audit.
- **Stay in the operational lane.** CVEs and security misconfig belong to
  `security-scan`. If a finding is "this base has a CVE", it's not ours; "this base
  is `:latest`" is.
- **Drift is the headline.** The highest-value output is env-var drift and
  deploy-plane fragmentation — surface them first; cosmetic best-practice findings are `low`.
- **No gate.** infra-scan informs, it does not fail builds. Posture is a judgement call.
- **Reports under `test-reports/infra/`** — add to `.gitignore`; nothing is written until `--save`.

## Output layout

```
~/.agents/skills/infra-scan/
├── SKILL.md
├── scripts/
│   ├── detect.sh     # JSON surface detection (read-only)
│   ├── analyze.sh    # JSON operational findings (read-only)
│   └── report.sh     # markdown render (stdout | --save)
└── references/
    └── catalog.md    # SoTA infra-analysis tooling + last_verified date
```

## Worked example

```bash
# 1. Which deploy surfaces exist?
bash ~/.agents/skills/infra-scan/scripts/detect.sh

# 2. Operational findings as JSON (pipe to jq / triage)
bash ~/.agents/skills/infra-scan/scripts/analyze.sh

# 3. Human-readable posture report, saved for CI artefact
bash ~/.agents/skills/infra-scan/scripts/report.sh --save
# → test-reports/infra/report.md
```

## When NOT to use

- You need CVE / security-misconfig scanning of Dockerfiles or IaC → that's
  `security-scan` (trivy config, checkov).
- You're choosing a NEW deploy platform or want a topology redesign → that's a
  decision; run `system-design` and feed it infra-scan's findings.
- Single-file Dockerfile lint → run `hadolint` directly (see `references/catalog.md`).
