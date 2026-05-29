**Trivy** — same binary as SCA/container. IaC misconfiguration scanning.

```bash
# macOS
brew install trivy
# any platform (install script, pinned channel)
curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
# or run via container
docker run --rm -v "$PWD:/src" aquasec/trivy config /src
```

Verify: `trivy --version`  (skill verified against 0.5x, 2026-05).

Fallback: **Checkov** (`pip install checkov`) for a larger policy catalog and
custom-policy authoring. NOTE: **tfsec is deprecated** (merged into Trivy in 2024) —
do not reach for it; `trivy config` is its successor.
