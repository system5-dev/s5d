  Install (cargo-llvm-cov — SoTA Rust coverage):
    cargo install cargo-llvm-cov
    rustup component add llvm-tools-preview

  Run with coverage + LCOV export:
    cargo llvm-cov --lcov --output-path coverage.lcov \
                   --html --output-dir test-reports/rust/coverage_html \
                   --fail-under-lines 70

  Run via skill:
    bash <skill>/scripts/run.sh --stacks rust
