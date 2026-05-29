**Syft** — SoTA SBOM generator. Emits CycloneDX (and SPDX).

```bash
# macOS
brew install syft
# any platform (install script, pinned to /usr/local/bin)
curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin
# or run via container
docker run --rm -v "$PWD:/src" anchore/syft:latest scan dir:/src -o cyclonedx-json
```

Verify: `syft version`  (skill verified against 1.x, 2026-05).

Pair with **Cosign** (`brew install cosign`) in CI to sign/attest the SBOM.
Fallback: **cdxgen** (`npm install -g @cyclonedx/cdxgen`) for deeper per-ecosystem
metadata when Syft's component graph is too coarse.
