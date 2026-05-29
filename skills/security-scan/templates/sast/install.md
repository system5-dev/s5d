**Semgrep** — SoTA SAST, polyglot, rule registry.

```bash
# macOS
brew install semgrep
# any platform (pinned)
python3 -m pip install semgrep
# or run via container
docker run --rm -v "$PWD:/src" semgrep/semgrep semgrep --config auto
```

Verify: `semgrep --version`  (skill verified against 1.x, 2026-05).

Fallback: **CodeQL** (`gh extension install github/gh-codeql`) when a language pack
Semgrep lacks is needed, or when you want GitHub-native code scanning.
