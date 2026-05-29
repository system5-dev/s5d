**Trivy** — SoTA all-in-one scanner. SCA here; also drives container, IaC, license.

```bash
# macOS
brew install trivy
# any platform (install script, pinned channel)
curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
# or run via container
docker run --rm -v "$PWD:/src" aquasec/trivy fs /src
```

Verify: `trivy --version`  (skill verified against 0.5x, 2026-05).

Fallback: **osv-scanner** (`brew install osv-scanner`) for OSV.dev-native results,
or GitHub **Dependabot** when you want platform-native PR alerts instead of a local gate.
