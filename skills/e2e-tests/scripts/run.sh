#!/usr/bin/env bash
# run.sh — execute e2e tests per axis, emit HTML + JUnit XML + traces/videos.
#
# Outputs under ./test-reports/e2e/<axis>/:
#   index.html        — Playwright HTML report (web/api) or Maestro report (mobile)
#   junit.xml         — JUnit XML for CI annotations
#   traces/           — Playwright .zip traces (on failure + retry)
#   videos/           — Playwright videos (on failure)
#   screenshots/      — failure screenshots
#
# Usage:
#   bash run.sh                                       # detect + run all axes
#   bash run.sh --axes web,api
#   bash run.sh --grep "@critical"                    # focus on tagged tests
#   bash run.sh --headed                              # run with browser visible (web)

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DETECT="$SKILL_DIR/scripts/detect.sh"

AXES=""
GREP_PATTERN=""
HEADED=0
REPORT_DIR="test-reports/e2e"

while [ $# -gt 0 ]; do
    case "$1" in
        --axes)       AXES="$2"; shift 2 ;;
        --grep)       GREP_PATTERN="$2"; shift 2 ;;
        --headed)     HEADED=1; shift ;;
        --report-dir) REPORT_DIR="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$AXES" ]; then
    AXES=$(bash "$DETECT" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | grep -E '^(web|api|mobile)$' | paste -sd, -)
fi
[ -z "$AXES" ] && { echo "no axes detected" >&2; exit 2; }

mkdir -p "$REPORT_DIR"
TOTAL_FAILED=0

run_web() {
    local out="$REPORT_DIR/web"
    mkdir -p "$out"
    echo "▸ playwright (web)" >&2
    local args=(test --reporter=html,junit,list)
    [ -n "$GREP_PATTERN" ] && args+=(--grep="$GREP_PATTERN")
    [ "$HEADED" -eq 1 ] && args+=(--headed)
    PLAYWRIGHT_HTML_REPORT="$out/html" \
    PLAYWRIGHT_JUNIT_OUTPUT_NAME="$out/junit.xml" \
        npx --no-install playwright "${args[@]}" || return 1
    # Copy traces / videos out of playwright's output dir
    [ -d test-results ] && cp -r test-results/* "$out/" 2>/dev/null || true
}

run_api() {
    local out="$REPORT_DIR/api"
    mkdir -p "$out"
    # Two flavours: hurl files OR a Playwright API-only project
    if ls e2e/api/*.hurl >/dev/null 2>&1; then
        echo "▸ hurl (api)" >&2
        hurl --test --report-html "$out/hurl-html" --report-junit "$out/junit.xml" \
            e2e/api/*.hurl || return 1
    elif ls e2e/api/*.spec.ts >/dev/null 2>&1 || ls e2e/api/*.spec.js >/dev/null 2>&1; then
        echo "▸ playwright (api project)" >&2
        local args=(test --project=api --reporter=html,junit,list)
        [ -n "$GREP_PATTERN" ] && args+=(--grep="$GREP_PATTERN")
        PLAYWRIGHT_HTML_REPORT="$out/html" \
        PLAYWRIGHT_JUNIT_OUTPUT_NAME="$out/junit.xml" \
            npx --no-install playwright "${args[@]}" || return 1
    else
        echo "  ! no api tests found (looked in e2e/api/*.hurl and *.spec.ts)" >&2
        return 0
    fi
}

run_mobile() {
    local out="$REPORT_DIR/mobile"
    mkdir -p "$out"
    if [ -d .maestro ]; then
        echo "▸ maestro" >&2
        if ! command -v maestro >/dev/null 2>&1; then
            echo "  ! maestro CLI not installed. See $SKILL_DIR/templates/mobile/install.md" >&2
            return 1
        fi
        maestro test --format junit --output "$out/junit.xml" .maestro/ || return 1
    elif command -v xcodebuild >/dev/null 2>&1 && [ "$(find . -name '*.xcodeproj' -not -path '*/node_modules/*' 2>/dev/null | head -1)" ]; then
        echo "▸ xcodebuild (XCUITest)" >&2
        echo "  ! XCUITest runner not implemented in skill v1 — invoke xcodebuild manually" >&2
        return 0
    elif [ -f gradlew ]; then
        echo "▸ ./gradlew connectedAndroidTest" >&2
        ./gradlew connectedAndroidTest --info | tee "$out/test-output.txt"
        return ${PIPESTATUS[0]}
    fi
}

IFS=',' read -ra axis_arr <<< "$AXES"
for a in "${axis_arr[@]}"; do
    [ -z "$a" ] && continue
    echo "" >&2
    echo "═══ ${a} ═══" >&2
    fn="run_${a}"
    if declare -f "$fn" >/dev/null; then
        "$fn" || TOTAL_FAILED=$((TOTAL_FAILED+1))
    fi
done

echo "" >&2
echo "═══ reports ═══" >&2
find "$REPORT_DIR" -maxdepth 2 -type f 2>/dev/null | sort >&2

if [ "$TOTAL_FAILED" -gt 0 ]; then
    echo "" >&2
    echo "✗ $TOTAL_FAILED axis/axes failed" >&2
    exit 1
fi
echo "" >&2
echo "✓ all axes passed" >&2
exit 0
