**gitleaks** — SoTA secret detection. Scans git history + work tree.

```bash
# macOS
brew install gitleaks
# any platform (pinned binary release)
curl -sSfL https://github.com/gitleaks/gitleaks/releases/latest/download/gitleaks_$(uname -s)_$(uname -m).tar.gz | tar -xz -C /usr/local/bin gitleaks
# or run via container
docker run --rm -v "$PWD:/src" zricethezav/gitleaks:latest detect --source /src
```

Verify: `gitleaks version`  (skill verified against 8.x, 2026-05).

Fallback: **trufflehog** (`brew install trufflehog`) when you want live credential
verification (it can validate that a found key still works against the provider).
