**Trivy** — same binary as SCA. Container image scanning (OS + app layers + secrets).

```bash
# macOS
brew install trivy
# any platform (install script, pinned channel)
curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
# scan an image directly via container (Docker socket mounted)
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy image <image:tag>
```

Verify: `trivy --version`  (skill verified against 0.5x, 2026-05).

Fallback: **Grype** (`brew install grype`) for a second-opinion vuln DB, or when you
want SBOM-driven scanning paired with Syft (`grype sbom:./sbom.cdx.json`).
