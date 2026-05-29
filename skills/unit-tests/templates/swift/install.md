  Swift Testing (SoTA Swift 6 default) + XCTest (still needed for UI/perf).

  Swift Testing is bundled with Xcode 16+ and Swift 6 toolchain — no extra install.
  Both frameworks coexist in one test target; UI/performance tests stay on XCTest.

  Package.swift target:
    .testTarget(
      name: "{{MODULE}}Tests",
      dependencies: ["{{MODULE}}"]
    )

  Run with coverage:
    swift test --enable-code-coverage
    # OR: xcodebuild test -scheme MyScheme -enableCodeCoverage YES \
    #         -resultBundlePath test-reports/swift/result.xcresult

  Export LCOV (skill does this automatically):
    xcrun llvm-cov export -format=lcov \
        -instr-profile .build/<arch>/debug/codecov/default.profdata \
        .build/<arch>/debug/<Module>PackageTests.xctest/Contents/MacOS/<Module>PackageTests

  Threshold check — use xcov or xccov-to-sonarqube (xcresultparser):
    brew install a7ex/homebrew-formulae/xcresultparser

  Run via skill:
    bash <skill>/scripts/run.sh --stacks swift
