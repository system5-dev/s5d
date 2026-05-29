  Install (Playwright — SoTA 2026 web e2e):
    bun add -D @playwright/test
    # or: npm install --save-dev @playwright/test
    bunx playwright install --with-deps

  For visual regression (built into Playwright):
    bunx playwright test --update-snapshots   # baseline
    bunx playwright test                       # compare

  Cypress 14 is still around if the team has heavy investment; otherwise pick
  Playwright. Playwright wins on cross-browser, parallelism, trace viewer,
  CDP debugging, and API testing in the same framework.

  Run via skill:
    bash <skill>/scripts/run.sh --axes web
