#!/usr/bin/env bash
# coverage.sh — compute USE-CASE coverage for e2e tests.
#
# For e2e the meaningful coverage metric is not lines-of-code but
# fraction of s5d use cases that have at least one e2e spec exercising them.
#
# Strategy:
#   1. List use cases from .s5d/discovery/architecture-map.md or
#      .s5d/scenarios/*.feature (whichever is the source of truth).
#   2. For each use case, search e2e specs for either:
#        - An @use-case:<id> tag in a comment / docstring
#        - A reference to the use case slug in test titles
#        - A reference to one of the use case's UX surfaces or capabilities
#          (from the architecture map)
#   3. Report covered / uncovered, plus the spec file that covers it.
#
# Output formats:
#   --format text    (default) — human-readable summary
#   --format json    — machine-readable
#   --format md      — markdown for PR comment
#
# Usage:
#   bash coverage.sh
#   bash coverage.sh --format json > test-reports/e2e/use-case-coverage.json
#   bash coverage.sh --threshold 60   # fail if < 60%

set -uo pipefail

FORMAT="text"
THRESHOLD=0

while [ $# -gt 0 ]; do
    case "$1" in
        --format)    FORMAT="$2"; shift 2 ;;
        --threshold) THRESHOLD="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,22p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# --- collect use cases --------------------------------------------------------
USE_CASES=()
USE_CASE_DOMAINS=()

# Source of truth #1: .s5d/scenarios/*.feature header tags (most explicit)
if [ -d .s5d/scenarios ]; then
    while IFS= read -r f; do
        slug=$(grep -m1 '^@s5d-use-case:' "$f" 2>/dev/null | sed 's/^@s5d-use-case://; s/[[:space:]]*$//')
        domain=$(grep -m1 '^@s5d-domain:' "$f" 2>/dev/null | sed 's/^@s5d-domain://; s/[[:space:]]*$//')
        [ -z "$slug" ] && continue
        # Dedup
        found=0
        for existing in "${USE_CASES[@]:-}"; do [ "$existing" = "$slug" ] && found=1 && break; done
        [ "$found" -eq 0 ] && USE_CASES+=("$slug") && USE_CASE_DOMAINS+=("$domain")
    done < <(find .s5d/scenarios -maxdepth 1 -name '*.feature' 2>/dev/null)
fi

# Source of truth #2: architecture-map.md id column (fallback if no scenarios)
if [ "${#USE_CASES[@]}" -eq 0 ] && [ -f .s5d/discovery/architecture-map.md ]; then
    while IFS= read -r slug; do
        USE_CASES+=("$slug"); USE_CASE_DOMAINS+=("unknown")
    done < <(grep -oE '`[a-z][a-z0-9-]+`' .s5d/discovery/architecture-map.md | tr -d '`' | sort -u)
fi

if [ "${#USE_CASES[@]}" -eq 0 ]; then
    echo "{\"error\":\"no use cases found — needs .s5d/scenarios/ or architecture-map.md\"}" >&2
    exit 2
fi

# --- collect e2e specs --------------------------------------------------------
SPECS=()
for d in e2e tests/e2e cypress/e2e .maestro; do
    [ -d "$d" ] || continue
    while IFS= read -r f; do SPECS+=("$f"); done < <(find "$d" -type f \
        \( -name '*.spec.ts' -o -name '*.spec.js' -o -name '*.spec.tsx' \
        -o -name '*.e2e.ts' -o -name '*.cy.ts' -o -name '*.cy.js' \
        -o -name '*.hurl' -o -name '*.yaml' \) 2>/dev/null)
done

# --- match use cases to specs -------------------------------------------------
declare -a COVERED_BY
i=0
for uc in "${USE_CASES[@]}"; do
    match=""
    # Pattern A: explicit @use-case:<slug> tag
    # Pattern B: slug literal anywhere in the file
    if [ "${#SPECS[@]}" -gt 0 ]; then
        match=$(grep -lE "@use-case:${uc}|@s5d-use-case:${uc}|\\b${uc}\\b" "${SPECS[@]}" 2>/dev/null | paste -sd';' -)
    fi
    COVERED_BY[$i]="$match"
    i=$((i+1))
done

# --- counts -------------------------------------------------------------------
TOTAL=${#USE_CASES[@]}
COVERED=0
for c in "${COVERED_BY[@]:-}"; do [ -n "$c" ] && COVERED=$((COVERED+1)); done
PERCENT=0
[ "$TOTAL" -gt 0 ] && PERCENT=$(( (COVERED * 100) / TOTAL ))

# --- output -------------------------------------------------------------------
case "$FORMAT" in
    text)
        echo "═══ e2e use-case coverage ═══"
        echo "covered: $COVERED / $TOTAL  (${PERCENT}%)"
        echo ""
        echo "── covered ──"
        i=0
        for uc in "${USE_CASES[@]}"; do
            [ -n "${COVERED_BY[$i]:-}" ] && printf "  ✓ %-40s  %s\n" "$uc" "${COVERED_BY[$i]}"
            i=$((i+1))
        done
        echo ""
        echo "── uncovered (gaps) ──"
        i=0
        for uc in "${USE_CASES[@]}"; do
            [ -z "${COVERED_BY[$i]:-}" ] && printf "  ✗ %-40s  domain=%s\n" "$uc" "${USE_CASE_DOMAINS[$i]:-unknown}"
            i=$((i+1))
        done
        ;;
    md)
        echo "# E2E Use-Case Coverage"
        echo ""
        echo "**Covered:** $COVERED / $TOTAL (${PERCENT}%)"
        echo ""
        echo "| Use case | Status | Covered by |"
        echo "|---|---|---|"
        i=0
        for uc in "${USE_CASES[@]}"; do
            if [ -n "${COVERED_BY[$i]:-}" ]; then
                echo "| \`$uc\` | ✓ | ${COVERED_BY[$i]} |"
            else
                echo "| \`$uc\` | ✗ | — |"
            fi
            i=$((i+1))
        done
        ;;
    json)
        printf '{\n  "total":%d,\n  "covered":%d,\n  "percent":%d,\n  "use_cases":[' \
            "$TOTAL" "$COVERED" "$PERCENT"
        first=1; i=0
        for uc in "${USE_CASES[@]}"; do
            if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
            cov="${COVERED_BY[$i]:-}"
            covered=$([ -n "$cov" ] && echo "true" || echo "false")
            printf '    {"id":"%s","domain":"%s","covered":%s,"specs":"%s"}' \
                "$uc" "${USE_CASE_DOMAINS[$i]:-unknown}" "$covered" "$cov"
            i=$((i+1))
        done
        printf '\n  ]\n}\n'
        ;;
    *)
        echo "unknown format: $FORMAT" >&2; exit 2 ;;
esac

# --- threshold gate -----------------------------------------------------------
if [ "$THRESHOLD" -gt 0 ] && [ "$PERCENT" -lt "$THRESHOLD" ]; then
    echo "" >&2
    echo "✗ coverage ${PERCENT}% below threshold ${THRESHOLD}%" >&2
    exit 1
fi
