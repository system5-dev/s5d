#!/usr/bin/env bash
# trace.sh — given a source file path, report the s5d chain:
#   file → component (architecture-map §5)
#        → domain (architecture-map §1)
#        → capabilities (architecture-map §2)
#        → use cases (architecture-map §4)
#        → e2e specs that cover the use cases
#        → .feature scenarios that motivated those e2e specs
#
# Useful for: PR descriptions, code-review notes, "where does this file
# fit in the architecture?" questions, refactor impact assessment.
#
# Usage:
#   bash trace.sh app/api/coi/generate-pdf/route.ts
#   bash trace.sh --format json utils/payment.ts

set -uo pipefail

MAP=".s5d/discovery/architecture-map.md"
SCENARIOS_DIR=".s5d/scenarios"
E2E_DIRS=(e2e e2e/api .maestro)
FORMAT="text"

while [ $# -gt 0 ]; do
    case "$1" in
        --format) FORMAT="$2"; shift 2 ;;
        --map)    MAP="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        -*) echo "unknown flag: $1" >&2; exit 2 ;;
        *)  SRC="${1:-}"; shift ;;
    esac
done

[ -z "${SRC:-}" ] && { echo "usage: $0 <source-file>" >&2; exit 2; }
[ ! -f "$MAP" ] && { echo "no map at $MAP" >&2; exit 2; }

# 1. Find row in §5 Components that mentions this path.
component_row=$(grep -nF "\`$SRC\`" "$MAP" 2>/dev/null | head -1)

if [ -z "$component_row" ]; then
    if [ "$FORMAT" = "json" ]; then
        printf '{"path":"%s","status":"orphan","message":"not in §5 Components"}\n' "$SRC"
    else
        echo "✗ $SRC is not registered in §5 Components."
        echo "  Run 'analyze.sh' to confirm it's an orphan, or add it to the map."
    fi
    exit 1
fi

# Extract row content (split on |): path | domain | feature/use-case | container | notes
# Map row format may vary; extract everything that looks like a domain or capability id.
domain=$(echo "$component_row" | awk -F'|' '{ print $3 }' | xargs)
feature=$(echo "$component_row" | awk -F'|' '{ print $4 }' | xargs)
container=$(echo "$component_row" | awk -F'|' '{ print $5 }' | xargs)

# 2. From the row, pluck capability ids (anything looking like `xxx.yyy`).
caps=$(echo "$component_row" | grep -oE '`[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*`' | sort -u | tr -d '`')

# 3. Which use cases include these capabilities?
use_cases=""
if [ -n "$caps" ]; then
    while IFS= read -r cap; do
        [ -z "$cap" ] && continue
        # Search §4 Use cases for rows mentioning this capability.
        ucs=$(awk '
            /^## 4\. Use cases/ { in_section = 1; next }
            /^## 5\./ { in_section = 0 }
            in_section && /^\| *`[a-z]/ { print }
        ' "$MAP" | grep -F "$cap" | awk -F'|' '{ gsub(/[\` ]/, "", $2); print $2 }')
        for uc in $ucs; do
            [ -z "$uc" ] && continue
            if ! echo "$use_cases" | grep -qF "$uc"; then
                use_cases="${use_cases}${uc}"$'\n'
            fi
        done
    done <<< "$caps"
fi

# 4. For each use case, find e2e specs (by @use-case tag or filename match)
declare e2e_specs_acc=""
if [ -n "$use_cases" ]; then
    while IFS= read -r uc; do
        [ -z "$uc" ] && continue
        for d in "${E2E_DIRS[@]}"; do
            [ ! -d "$d" ] && continue
            spec=$(grep -rlE "@use-case:${uc}|${uc}\\.spec\\." "$d" 2>/dev/null | head -1)
            [ -n "$spec" ] && e2e_specs_acc="${e2e_specs_acc}${uc}\t${spec}"$'\n'
        done
    done <<< "$use_cases"
fi

# 5. For each use case, find the .feature scenarios
declare features_acc=""
if [ -n "$use_cases" ]; then
    while IFS= read -r uc; do
        [ -z "$uc" ] && continue
        feature="$SCENARIOS_DIR/${uc}.feature"
        [ -f "$feature" ] && features_acc="${features_acc}${uc}\t${feature}"$'\n'
    done <<< "$use_cases"
fi

# --- output ------------------------------------------------------------------
if [ "$FORMAT" = "text" ]; then
    echo "── trace ──"
    echo "  source:        $SRC"
    echo "  domain:        $domain"
    echo "  feature/uc:    $feature"
    echo "  container:     $container"
    echo ""
    echo "  capabilities:"
    if [ -z "$caps" ]; then echo "    (none parsed from the map row)"
    else echo "$caps" | sed 's/^/    - /'
    fi
    echo ""
    echo "  use cases:"
    if [ -z "$use_cases" ]; then echo "    (none inferred)"
    else echo "$use_cases" | sed '/^$/d' | sed 's/^/    - /'
    fi
    echo ""
    echo "  e2e specs:"
    if [ -z "$e2e_specs_acc" ]; then echo "    (no e2e coverage for this file's use cases)"
    else printf '%b' "$e2e_specs_acc" | awk -F'\t' '$1!="" { printf "    - %s → %s\n", $1, $2 }'
    fi
    echo ""
    echo "  bdd scenarios:"
    if [ -z "$features_acc" ]; then echo "    (no .feature files found for this file's use cases)"
    else printf '%b' "$features_acc" | awk -F'\t' '$1!="" { printf "    - %s → %s\n", $1, $2 }'
    fi
elif [ "$FORMAT" = "json" ]; then
    printf '{"path":"%s","domain":"%s","capabilities":[' "$SRC" "$domain"
    first=1
    for c in $caps; do
        if [ "$first" -eq 1 ]; then first=0; else printf ','; fi
        printf '"%s"' "$c"
    done
    printf '],"use_cases":['
    first=1
    while IFS= read -r uc; do
        [ -z "$uc" ] && continue
        if [ "$first" -eq 1 ]; then first=0; else printf ','; fi
        printf '"%s"' "$uc"
    done <<< "$use_cases"
    printf ']}\n'
fi
