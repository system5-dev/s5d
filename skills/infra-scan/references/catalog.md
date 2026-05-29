# infra-scan — tooling catalog

**last_verified: 2026-05-29**

infra-scan's core checks are self-contained shell heuristics (no install needed) so
it runs anywhere. This catalog lists the SoTA external tools the agent may reach for
when a finding needs deeper, tool-grade analysis — and draws the line against
`security-scan`'s territory.

## Operational posture (infra-scan's lane)

| Surface | SoTA tool | Use | Install |
|---|---|---|---|
| Dockerfile | **hadolint** | best-practice lint (pinning, layer hygiene, non-root) | `brew install hadolint` |
| k8s manifests | **kube-linter** (StackRox) | operational checks: resources, probes, replicas, antiaffinity | `brew install kube-linter` |
| k8s manifests | **kube-score** | score-based reliability grading | `brew install kube-score` |
| helm | **helm lint** + `helm template \| kube-linter` | render then lint the rendered manifests | ships with helm |
| env files | **dotenv-linter** | ordering, duplicates, quoting in `.env*` | `brew install dotenv-linter` |
| compose | **docker compose config** | validate + resolve interpolation | ships with docker |

Why heuristics first: the highest-value findings — **env-var drift** (used-in-code vs
declared) and **deploy-plane fragmentation** (vercel + gitlab + helm + docker in one
repo) — are not produced by any off-the-shelf linter. They require cross-file
correlation, which is exactly what `analyze.sh` does. The tools above sharpen the
per-file checks; they do not replace the topology view.

## NOT infra-scan's lane (→ security-scan)

| Concern | Owner |
|---|---|
| CVEs in base images / dependencies | `security-scan` (trivy image, trivy fs) |
| IaC security misconfig (open security groups, public buckets) | `security-scan` (trivy config, checkov) |
| Secrets committed in configs | `security-scan` (gitleaks) |
| SBOM | `security-scan` (syft) |

A `:latest` base image is an **infra** finding (non-reproducible). A base image with a
known CVE is a **security** finding. Same file, different lens — keep them separate so
neither report drowns the other.

## Policy-as-code (v1.1 follow-up)

For teams that want enforceable infra policy rather than advisory posture:
- **Conftest / OPA (Rego)** — write policies against Dockerfile/helm/k8s/terraform
  and gate CI on them.
- **Checkov custom policies** — overlaps security; use for compliance packs (CIS, SOC2).

v1 of infra-scan is advisory (no gate by design). Wiring conftest as an optional
gate is the natural v1.1 extension, mirroring how `security-scan` gates on severity.

## Notes

- Verify tool currency before trusting versions — re-run the agentic research that
  produced this list if `last_verified` is more than ~6 months stale.
- tfsec is **deprecated** (merged into Trivy, 2024) — do not add it; `trivy config`
  is the successor and already lives in `security-scan`.
