  Install (Vitest — SoTA 2026, replaces Jest for new projects):
    bun add -D vitest @vitest/coverage-v8 @vitest/ui
    # or: npm install --save-dev vitest @vitest/coverage-v8 @vitest/ui

  Migrating from Jest? APIs are largely Jest-compatible.
  See: https://vitest.dev/guide/migration.html

  Existing Jest projects (React Native or large legacy suites):
    bun add -D jest @types/jest ts-jest jest-junit jest-environment-jsdom

  Run with reports:
    bash <skill>/scripts/run.sh --stacks js
