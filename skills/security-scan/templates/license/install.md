**Trivy** — same binary as SCA/container/IaC. Dependency license classification.

```bash
# macOS
brew install trivy
# any platform (install script, pinned channel)
curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
# or run via container
docker run --rm -v "$PWD:/src" aquasec/trivy fs --scanners license /src
```

Verify: `trivy --version`  (skill verified against 0.5x, 2026-05).

Fallback: **ScanCode Toolkit** (`pip install scancode-toolkit`) for forensic-grade
license detection (per-file, with text-match evidence) when a legal review needs
more than Trivy's package-level classification.
