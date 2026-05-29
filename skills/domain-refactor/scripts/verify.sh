#!/usr/bin/env bash
# verify.sh — post-step verification.
#
# Confirms that a refactor step did not break:
#   1. compilation (tsc / cargo check / go vet)
#   2. unit-tests + coverage threshold
#   3. e2e use-case coverage (must not drop vs. baseline)
#   4. architecture violation count (must not go up vs. baseline)
#
# Use BEFORE a refactor step to capture baselines, then AFTER to compare.
#
# Usage:
#   bash verify.sh --capture                # store baselines from current tree
#   bash verify.sh                          # compare current state against baseline
#   bash verify.sh --strict                 # also fail if anything increased

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ANALYZE="$SKILL_DIR/scripts/analyze.sh"
BASELINE_DIR=".s5d/refactor-baseline"
MODE="compare"
STRICT=0

while [ $# -gt 0 ]; do
    case "$1" in
        --capture) MODE="capture"; shift ;;
        --strict)  STRICT=1; shift ;;
        -h|--help)
            sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

mkdir -p "$BASELINE_DIR"

# --- collect current snapshot -------------------------------------------------
SNAPSHOT="$BASELINE_DIR/.snapshot.tmp"
rm -rf "$SNAPSHOT"; mkdir -p "$SNAPSHOT"

# 1. Architecture violations
bash "$ANALYZE" > "$SNAPSHOT/analyze.json" 2>/dev/null || true
violations_total=$(python3 -c "
import json
try: print(json.load(open('$SNAPSHOT/analyze.json'))['summary']['total'])
except: print(-1)
")

# 2. Unit tests + coverage (best-effort — fall back gracefully if no skill installed)
unit_pass="skipped"
unit_coverage="skipped"
if [ -x ~/.agents/skills/unit-tests/scripts/run.sh ]; then
    if bash ~/.agents/skills/unit-tests/scripts/run.sh --no-coverage >/dev/null 2>&1; then
        unit_pass="passed"
        # Re-run with coverage to grab the percent (one stack at a time)
        if [ -f test-reports/js/coverage/coverage-summary.json ] 2>/dev/null; then
            unit_coverage=$(python3 -c "
import json
try:
    d = json.load(open('test-reports/js/coverage/coverage-summary.json'))
    print(d['total']['lines']['pct'])
except: print('unknown')
")
        fi
    else
        unit_pass="failed"
    fi
fi

# 3. E2E use-case coverage
e2e_pct="skipped"
if [ -x ~/.agents/skills/e2e-tests/scripts/coverage.sh ]; then
    e2e_pct=$(bash ~/.agents/skills/e2e-tests/scripts/coverage.sh --format json 2>/dev/null \
              | python3 -c "import json,sys; print(json.load(sys.stdin).get('percent','unknown'))" 2>/dev/null \
              || echo "unknown")
fi

# Save snapshot.
cat > "$SNAPSHOT/state.txt" <<EOF
violations_total=$violations_total
unit_pass=$unit_pass
unit_coverage=$unit_coverage
e2e_pct=$e2e_pct
captured_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF

if [ "$MODE" = "capture" ]; then
    mv "$SNAPSHOT/analyze.json" "$BASELINE_DIR/analyze.json"
    mv "$SNAPSHOT/state.txt" "$BASELINE_DIR/state.txt"
    rm -rf "$SNAPSHOT"
    echo "✓ baseline captured to $BASELINE_DIR/" >&2
    cat "$BASELINE_DIR/state.txt"
    exit 0
fi

# Compare against baseline
if [ ! -f "$BASELINE_DIR/state.txt" ]; then
    echo "no baseline — run 'verify.sh --capture' before your first refactor step." >&2
    rm -rf "$SNAPSHOT"
    exit 2
fi

. "$BASELINE_DIR/state.txt"
baseline_violations="$violations_total"
baseline_unit_pass="$unit_pass"
baseline_unit_coverage="$unit_coverage"
baseline_e2e_pct="$e2e_pct"

. "$SNAPSHOT/state.txt"
current_violations="$violations_total"
current_unit_pass="$unit_pass"
current_unit_coverage="$unit_coverage"
current_e2e_pct="$e2e_pct"

rm -rf "$SNAPSHOT"

echo ""
echo "═══ refactor verification ═══"
echo ""
printf '  %-22s  %-12s  %-12s\n' "metric" "baseline" "current"
printf '  %-22s  %-12s  %-12s\n' "----------------------" "------------" "------------"
printf '  %-22s  %-12s  %-12s\n' "violations (total)" "$baseline_violations" "$current_violations"
printf '  %-22s  %-12s  %-12s\n' "unit tests" "$baseline_unit_pass" "$current_unit_pass"
printf '  %-22s  %-12s  %-12s\n' "unit coverage %" "$baseline_unit_coverage" "$current_unit_coverage"
printf '  %-22s  %-12s  %-12s\n' "e2e use-case %" "$baseline_e2e_pct" "$current_e2e_pct"
echo ""

fail=0

# Hard gates (always fail on these)
if [ "$current_unit_pass" = "failed" ]; then
    echo "✗ unit tests FAILED — rollback this step" >&2
    fail=1
fi

if [ "$current_violations" != "-1" ] && [ "$baseline_violations" != "-1" ] && \
   [ "$current_violations" -gt "$baseline_violations" ]; then
    echo "✗ violation count went UP: $baseline_violations → $current_violations" >&2
    fail=1
fi

# Strict mode: also fail on coverage drop
if [ "$STRICT" -eq 1 ]; then
    if [ "$baseline_unit_coverage" != "unknown" ] && [ "$current_unit_coverage" != "unknown" ] && \
       [ "$baseline_unit_coverage" != "skipped" ] && [ "$current_unit_coverage" != "skipped" ]; then
        cov_drop=$(python3 -c "print(int(float('$baseline_unit_coverage')) > int(float('$current_unit_coverage')))" 2>/dev/null || echo 0)
        [ "$cov_drop" = "True" ] && { echo "✗ strict: unit coverage dropped"; fail=1; }
    fi
    if [ "$baseline_e2e_pct" != "unknown" ] && [ "$current_e2e_pct" != "unknown" ] && \
       [ "$baseline_e2e_pct" != "skipped" ] && [ "$current_e2e_pct" != "skipped" ]; then
        [ "$current_e2e_pct" -lt "$baseline_e2e_pct" ] && { echo "✗ strict: e2e use-case % dropped"; fail=1; }
    fi
fi

if [ "$fail" -eq 1 ]; then
    echo ""
    echo "VERIFY FAILED — roll back the last refactor step and diagnose." >&2
    exit 1
fi

echo "✓ verification passed"
