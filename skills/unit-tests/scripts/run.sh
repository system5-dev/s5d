#!/usr/bin/env bash
# run.sh — run tests + emit reports for each detected stack.
#
# Reports written under ./test-reports/<stack>/ :
#   coverage.lcov   — LCOV format, for Codecov / Coveralls / VS Code Coverage Gutters
#   junit.xml       — JUnit XML, for CI annotations
#   coverage.html   — human-readable summary
#   coverage.txt    — one-line console summary
#
# Coverage threshold (default 70%) is enforced; non-zero exit on fail.
#
# Usage:
#   bash run.sh                                    # detect + run all
#   bash run.sh --stacks js,python                 # subset
#   bash run.sh --threshold 80
#   bash run.sh --stacks rust --no-coverage        # tests only

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DETECT="$SKILL_DIR/scripts/detect.sh"

STACKS=""
THRESHOLD=70
COVERAGE=1
REPORT_DIR="test-reports"

while [ $# -gt 0 ]; do
    case "$1" in
        --stacks)      STACKS="$2"; shift 2 ;;
        --threshold)   THRESHOLD="$2"; shift 2 ;;
        --no-coverage) COVERAGE=0; shift ;;
        --report-dir)  REPORT_DIR="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$STACKS" ]; then
    STACKS=$(bash "$DETECT" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | paste -sd, -)
fi
[ -z "$STACKS" ] && { echo "no stacks detected" >&2; exit 2; }

mkdir -p "$REPORT_DIR"
TOTAL_FAILED=0

run_js() {
    local out="$REPORT_DIR/js"
    mkdir -p "$out"
    # Detect framework
    if grep -q '"vitest"' package.json; then
        echo "▸ vitest" >&2
        local args=(run --reporter=default --reporter=junit --outputFile="$out/junit.xml")
        [ "$COVERAGE" -eq 1 ] && args+=(--coverage --coverage.reporter=lcov --coverage.reporter=html --coverage.reporter=text
                                       --coverage.reportsDirectory="$out/coverage"
                                       --coverage.thresholds.lines="$THRESHOLD"
                                       --coverage.thresholds.functions="$THRESHOLD"
                                       --coverage.thresholds.statements="$THRESHOLD")
        npx --no-install vitest "${args[@]}" || return 1
        [ -f "$out/coverage/lcov.info" ] && cp "$out/coverage/lcov.info" "$out/coverage.lcov"
    elif grep -q '"jest"' package.json; then
        echo "▸ jest" >&2
        local args=(--reporters=default --reporters=jest-junit)
        if [ "$COVERAGE" -eq 1 ]; then
            args+=(--coverage --coverageDirectory="$out/coverage"
                   --coverageReporters=lcov --coverageReporters=html --coverageReporters=text
                   --coverageThreshold="{\"global\":{\"lines\":$THRESHOLD,\"functions\":$THRESHOLD,\"statements\":$THRESHOLD}}")
        fi
        JEST_JUNIT_OUTPUT_DIR="$out" JEST_JUNIT_OUTPUT_NAME="junit.xml" \
            npx --no-install jest "${args[@]}" || return 1
        [ -f "$out/coverage/lcov.info" ] && cp "$out/coverage/lcov.info" "$out/coverage.lcov"
    else
        echo "  ! no JS test framework detected" >&2
        return 0
    fi
}

run_python() {
    local out="$REPORT_DIR/python"
    mkdir -p "$out"
    echo "▸ pytest" >&2
    local args=(--junitxml="$out/junit.xml")
    if [ "$COVERAGE" -eq 1 ]; then
        args+=(--cov --cov-report=lcov:"$out/coverage.lcov"
               --cov-report=html:"$out/coverage_html"
               --cov-report=term
               --cov-fail-under="$THRESHOLD")
    fi
    if command -v uv >/dev/null 2>&1; then
        uv run pytest "${args[@]}" || return 1
    else
        python3 -m pytest "${args[@]}" || return 1
    fi
}

run_go() {
    local out="$REPORT_DIR/go"
    mkdir -p "$out"
    echo "▸ go test" >&2
    if [ "$COVERAGE" -eq 1 ]; then
        go test -coverprofile="$out/coverage.out" -covermode=atomic ./... | tee "$out/test-output.txt"
        local rc=${PIPESTATUS[0]}
        # Convert to LCOV via gcov2lcov if installed, else leave .out only.
        if command -v gcov2lcov >/dev/null 2>&1; then
            gcov2lcov -infile "$out/coverage.out" -outfile "$out/coverage.lcov"
        fi
        go tool cover -html="$out/coverage.out" -o "$out/coverage.html"
        # Enforce threshold
        local pct
        pct=$(go tool cover -func="$out/coverage.out" | tail -1 | awk '{print $3}' | tr -d '%')
        echo "coverage: ${pct}% (threshold ${THRESHOLD}%)" >&2
        awk -v p="$pct" -v t="$THRESHOLD" 'BEGIN{exit !(p >= t)}' || {
            echo "  ✗ go coverage below threshold" >&2
            return 1
        }
        return "$rc"
    else
        go test ./... | tee "$out/test-output.txt"
        return ${PIPESTATUS[0]}
    fi
}

run_rust() {
    local out="$REPORT_DIR/rust"
    mkdir -p "$out"
    if [ "$COVERAGE" -eq 1 ] && command -v cargo-llvm-cov >/dev/null 2>&1; then
        echo "▸ cargo llvm-cov" >&2
        cargo llvm-cov --lcov --output-path "$out/coverage.lcov" --html --output-dir "$out/coverage_html" \
            --fail-under-lines "$THRESHOLD" || return 1
    else
        echo "▸ cargo test" >&2
        cargo test --all-targets || return 1
        [ "$COVERAGE" -eq 1 ] && echo "  (cargo-llvm-cov not installed — install: cargo install cargo-llvm-cov)" >&2
    fi
}

run_java() {
    local out="$REPORT_DIR/java"
    mkdir -p "$out"
    if [ -f gradlew ]; then
        echo "▸ ./gradlew test jacocoTestReport" >&2
        ./gradlew test jacocoTestReport || return 1
        # Copy reports
        find . -path '*/build/reports/jacoco*/jacocoTestReport.xml' -exec cp {} "$out/jacoco.xml" \; 2>/dev/null
        find . -path '*/build/test-results/test/*.xml' -exec cp {} "$out/" \; 2>/dev/null
    elif [ -f pom.xml ]; then
        echo "▸ mvn test" >&2
        mvn -q test || return 1
        find . -path '*/target/site/jacoco/jacoco.xml' -exec cp {} "$out/jacoco.xml" \; 2>/dev/null
        find . -path '*/target/surefire-reports/*.xml' -exec cp {} "$out/" \; 2>/dev/null
    fi
}

run_kotlin() {
    local out="$REPORT_DIR/kotlin"
    mkdir -p "$out"
    if [ -f gradlew ]; then
        echo "▸ ./gradlew test koverXmlReport" >&2
        ./gradlew test koverXmlReport || return 1
        find . -path '*/build/reports/kover/*.xml' -exec cp {} "$out/kover.xml" \; 2>/dev/null
    fi
}

run_dotnet() {
    local out="$REPORT_DIR/dotnet"
    mkdir -p "$out"
    echo "▸ dotnet test" >&2
    dotnet test --collect:"XPlat Code Coverage" --results-directory "$out" \
        --logger "junit;LogFilePath=$out/junit.xml" || return 1
    # coverlet emits .cobertura.xml; convert later if needed
    find "$out" -name 'coverage.cobertura.xml' -exec cp {} "$out/coverage.cobertura.xml" \; 2>/dev/null
}

run_ruby() {
    local out="$REPORT_DIR/ruby"
    mkdir -p "$out"
    echo "▸ rspec" >&2
    bundle exec rspec --format RspecJunitFormatter --out "$out/junit.xml" || return 1
    [ -d coverage ] && cp -r coverage "$out/"
}

run_php() {
    local out="$REPORT_DIR/php"
    mkdir -p "$out"
    echo "▸ phpunit" >&2
    vendor/bin/phpunit --log-junit "$out/junit.xml" --coverage-clover "$out/coverage.xml" \
        --coverage-html "$out/coverage_html" || return 1
}

run_swift() {
    local out="$REPORT_DIR/swift"
    mkdir -p "$out"
    echo "▸ swift test --enable-code-coverage" >&2
    swift test --enable-code-coverage || return 1
    # llvm-cov export to LCOV when xcrun available
    if command -v xcrun >/dev/null 2>&1; then
        local bin xctest_path profdata
        bin=$(swift build --show-bin-path 2>/dev/null)
        profdata="$bin/codecov/default.profdata"
        if [ -f "$profdata" ]; then
            xctest_path=$(find "$bin" -name '*.xctest' | head -1)
            [ -n "$xctest_path" ] && xcrun llvm-cov export -format=lcov \
                -instr-profile "$profdata" "$xctest_path/Contents/MacOS/$(basename "$xctest_path" .xctest)" \
                > "$out/coverage.lcov" 2>/dev/null || true
        fi
    fi
}

run_shell() {
    local out="$REPORT_DIR/shell"
    mkdir -p "$out"
    echo "▸ bats" >&2
    bats --formatter junit --output "$out" *.bats tests/*.bats 2>/dev/null || \
        bats $(find . -name '*.bats' -not -path '*/node_modules/*') || return 1
}

# Dispatcher
IFS=',' read -ra stack_arr <<< "$STACKS"
for s in "${stack_arr[@]}"; do
    [ -z "$s" ] && continue
    echo "" >&2
    echo "═══ ${s} ═══" >&2
    fn="run_${s}"
    if declare -f "$fn" >/dev/null; then
        "$fn" || TOTAL_FAILED=$((TOTAL_FAILED+1))
    else
        echo "  ! no runner for $s" >&2
    fi
done

echo "" >&2
echo "═══ summary ═══" >&2
echo "report directory: $REPORT_DIR/" >&2
ls -la "$REPORT_DIR" 2>&1 >&2
echo "" >&2
if [ "$TOTAL_FAILED" -gt 0 ]; then
    echo "✗ $TOTAL_FAILED stack(s) failed" >&2
    exit 1
fi
echo "✓ all stacks passed" >&2
exit 0
