  Two SoTA options for API e2e:

  Option A — Playwright API requests (same toolchain as web e2e).
    bun add -D @playwright/test
    # Use test.use({ baseURL }) and request fixture; no browser needed.
    # The generated playwright.config.ts already defines an 'api' project.

  Option B — Hurl (text-based HTTP testing, language-agnostic).
    brew install hurl
    # or: cargo install hurl
    # Define .hurl files with requests + assertions + captures.

  Pick Hurl if:
    - The repo is backend-only and you don't want a JS toolchain.
    - You want flat text files that read like curl recipes.
    - Pen-testers / SREs share the test files.

  Pick Playwright API if:
    - The repo already runs Playwright for web (reuse infrastructure).
    - You need rich assertions / TypeScript types / shared fixtures.

  Run via skill:
    bash <skill>/scripts/run.sh --axes api
