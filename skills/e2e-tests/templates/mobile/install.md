  Maestro — SoTA cross-platform mobile e2e (mobile.dev, 2024-2026).
  Covers React Native, native iOS, native Android with one tool and flow files.

  Install:
    curl -fsSL https://get.maestro.mobile.dev | bash
    # or via brew:
    brew tap mobile-dev-inc/tap && brew install maestro

  Flow files: .maestro/<flow>.yaml — declarative, YAML-based, human-readable.

  Run on a connected device / simulator:
    maestro test .maestro/                                  # all flows
    maestro test --format junit --output report .maestro/   # for CI

  When to keep Detox (React Native legacy): existing investment, native-bridge
  inspection needs. Maestro is recommended for new RN projects.

  When to keep XCUITest (native iOS): performance tests (XCTMetric), or deep
  XCUIElement introspection. Maestro covers happy-path flows just as well.

  When to keep Espresso (native Android): same — keep for fine-grained idling
  resource and view introspection. Maestro for top-of-funnel flows.

  Run via skill:
    bash <skill>/scripts/run.sh --axes mobile
