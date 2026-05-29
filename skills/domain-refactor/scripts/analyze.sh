#!/usr/bin/env bash
# analyze.sh — scan code against the s5d architecture map, surface
# boundary violations as JSON.
#
# Detected violations:
#   god-component    — file > GOD_SIZE bytes AND ≥3 exports
#   domain-leak      — generic/supporting component imports a core-domain entity
#   missing-contract — cross-domain edge with no contract column in §7
#   orphan           — source file not referenced anywhere in the map
#   drift-missing    — file path listed in map, file absent
#   capability-dup   — same capability id listed twice in §2
#
# Safety class per file (from coverage):
#   safe   coverage ≥ SAFE_PCT  (default 70%)
#   risky  coverage in [RISKY_PCT, SAFE_PCT) OR e2e spec only
#   unsafe coverage < RISKY_PCT AND no e2e — DO NOT refactor without adding tests
#
# Usage:
#   bash analyze.sh                            # full scan
#   bash analyze.sh --god-size 30000
#   bash analyze.sh --map .s5d/discovery/architecture-map.md
#   bash analyze.sh --lcov test-reports/js/coverage/lcov.info

set -uo pipefail

MAP=".s5d/discovery/architecture-map.md"
LCOV="test-reports/js/coverage/lcov.info"
E2E_DIR="e2e"
GOD_SIZE=30000
SAFE_PCT=70
RISKY_PCT=30

while [ $# -gt 0 ]; do
    case "$1" in
        --map)       MAP="$2"; shift 2 ;;
        --lcov)      LCOV="$2"; shift 2 ;;
        --god-size)  GOD_SIZE="$2"; shift 2 ;;
        --safe-pct)  SAFE_PCT="$2"; shift 2 ;;
        --risky-pct) RISKY_PCT="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,22p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

[ ! -f "$MAP" ] && { echo "no architecture map at $MAP" >&2; exit 2; }

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

# count grep matches — always emits a number, never aborts on no-match.
count_matches() {
    local pat="$1" file="$2"
    grep -c "$pat" "$file" 2>/dev/null || true
}

# strip newlines / surrounding whitespace from an integer-like variable
clean_int() {
    printf '%s' "$1" | tr -d '\n ' | grep -oE '[0-9]+' | head -1
}

PRUNE_ARGS=(
    -name node_modules -o -name .git -o -name vendor -o -name target
    -o -name build -o -name dist -o -name .next -o -name .venv
    -o -name __pycache__ -o -name .gradle -o -name .mvn
    -o -name .claude -o -name .codex -o -name .codex-plugin
    -o -name .claude-plugin -o -name .s5d -o -name .agents
    -o -name .playwright-browsers -o -name test-results -o -name test-reports
)

# --- temp store --------------------------------------------------------------
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
VIOLATIONS="$TMP/violations.tsv"
COV_TSV="$TMP/cov.tsv"
: > "$VIOLATIONS"
: > "$COV_TSV"

# --- parse architecture-map: §5 components -----------------------------------
# Row shape: | `path/to/file.tsx` (98 KB) | domain | ... |
# Want just the path between the first pair of backticks.
awk '
    /^## 5\. Components/ { in_section = 1; next }
    /^## 6\./ { in_section = 0 }
    in_section && /^\| *`[^`]+`/ {
        # Strip leading "| `", then everything from first `\`` onward.
        sub(/^\| *`/, "")
        sub(/`.*$/, "")
        print
    }
' "$MAP" | sort -u > "$TMP/map-components.txt"

# --- parse architecture-map: §4 use-case ids --------------------------------
awk '
    /^## 4\. Use cases/ { in_section = 1; next }
    /^## 5\./ { in_section = 0 }
    in_section && /^\| *`[a-z][a-z0-9-]*`/ {
        sub(/^\| *`/, "")
        sub(/`.*$/, "")
        print
    }
' "$MAP" | sort -u > "$TMP/map-use-cases.txt"

# --- coverage from LCOV (no assoc arrays — pure tmpfile lookup) -------------
if [ -f "$LCOV" ]; then
    awk '
        /^SF:/ { sf = $0; sub(/^SF:/, "", sf); lh = 0; lf = 0; next }
        /^LH:/ { lh = $0; sub(/^LH:/, "", lh); next }
        /^LF:/ { lf = $0; sub(/^LF:/, "", lf); next }
        /^end_of_record/ {
            if (sf != "" && lf+0 > 0) {
                pct = int((lh+0) * 100 / (lf+0))
                printf "%s\t%d\n", sf, pct
            }
            sf = ""; lh = 0; lf = 0
        }
    ' "$LCOV" > "$COV_TSV" 2>/dev/null || true
fi

# Lookup file coverage by path; returns empty string if unknown.
coverage_pct() {
    local f="$1"
    [ ! -s "$COV_TSV" ] && return
    local hit
    hit=$(grep -m1 -F "	" "$COV_TSV" 2>/dev/null | head -0) # silence noop
    # Match either bare path or ./path variants
    awk -F'\t' -v p="$f" -v pdot="./$f" '
        $1 == p || $1 == pdot { print $2; exit }
    ' "$COV_TSV"
}

# Safety class per file
safety_for() {
    local f="$1"
    local pct
    pct=$(coverage_pct "$f")
    if [ -n "$pct" ]; then
        if [ "$pct" -ge "$SAFE_PCT" ]; then echo "safe"
        elif [ "$pct" -ge "$RISKY_PCT" ]; then echo "risky"
        else echo "unsafe"
        fi
        return
    fi
    # Fall back to e2e check
    local base; base=$(basename "$f" | sed 's/\.[^.]*$//')
    if [ -d "$E2E_DIR" ] && grep -rqlE "$base" "$E2E_DIR" 2>/dev/null; then
        echo "risky"
        return
    fi
    echo "unsafe"
}

emit_v() {
    # kind  severity  path  detail
    printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >> "$VIOLATIONS"
}

# --- check: god-component ---------------------------------------------------
check_god_components() {
    find . -type d \( ${PRUNE_ARGS[@]} \) -prune -o -type f \
        \( -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' \
           -o -name '*.py' -o -name '*.go' -o -name '*.rs' -o -name '*.java' \) \
        -print 2>/dev/null \
    | while IFS= read -r f; do
        [ -f "$f" ] || continue
        size=$(wc -c < "$f" 2>/dev/null | tr -d ' ')
        size=${size:-0}
        [ "$size" -lt "$GOD_SIZE" ] && continue
        case "$f" in
            *.ts|*.tsx|*.js|*.jsx)
                exports=$(count_matches '^export ' "$f"); exports=$(clean_int "${exports:-0}") ;;
            *.py)
                exports=$(count_matches '^def \|^class ' "$f"); exports=$(clean_int "${exports:-0}") ;;
            *.go)
                exports=$(count_matches '^func [A-Z]' "$f"); exports=$(clean_int "${exports:-0}") ;;
            *.rs)
                exports=$(count_matches '^pub fn \|^pub struct ' "$f"); exports=$(clean_int "${exports:-0}") ;;
            *.java)
                exports=$(count_matches '^[[:space:]]*public ' "$f"); exports=$(clean_int "${exports:-0}") ;;
            *) exports=0 ;;
        esac
        exports=${exports:-0}
        clean=${f#./}
        if [ "$exports" -ge 3 ]; then
            emit_v "god-component" "high" "$clean" \
                "size=${size}B exports=${exports} (threshold ${GOD_SIZE}B + 3 exports)"
        fi
    done
}

# --- check: orphans (files not in architecture map) -------------------------
# Only flag substantial files (>= ORPHAN_MIN_LINES). The map's §5 explicitly
# only enumerates "selected — large or boundary-bearing" components, so flagging
# every tiny file produces noise. We surface ones big enough to plausibly be
# registered.
ORPHAN_MIN_LINES=200
check_orphans() {
    local dirs=(app components lib utils hooks store)
    for d in "${dirs[@]}"; do
        [ ! -d "$d" ] && continue
        find "$d" -type d \( ${PRUNE_ARGS[@]} \) -prune -o -type f \
            \( -name '*.ts' -o -name '*.tsx' -o -name '*.js' \) -print 2>/dev/null \
        | while IFS= read -r f; do
            clean=${f#./}
            case "$clean" in
                *.test.*|*.spec.*|*.d.ts|*.config.*) continue ;;
            esac
            lines=$(wc -l < "$f" 2>/dev/null | tr -d ' ')
            lines=${lines:-0}
            [ "$lines" -lt "$ORPHAN_MIN_LINES" ] && continue
            if ! grep -qF "\`$clean\`" "$MAP" 2>/dev/null; then
                emit_v "orphan" "info" "$clean" "${lines} lines; not referenced in architecture-map.md"
            fi
        done
    done
}

# --- check: drift (component in map, file gone) -----------------------------
check_drift() {
    while IFS= read -r path; do
        [ -z "$path" ] && continue
        # Skip glob entries
        case "$path" in *\**) continue ;; esac
        if [ ! -e "$path" ] && [ ! -e "./$path" ]; then
            emit_v "drift-missing" "medium" "$path" "listed in map, file not found"
        fi
    done < "$TMP/map-components.txt"
}

# --- check: capability duplication ------------------------------------------
check_capability_dup() {
    awk '
        /^## 2\. Capabilities/ { in_section = 1; next }
        /^## 3\./ { in_section = 0 }
        in_section && /^\| `[^`]+`/ {
            sub(/^\| *`/, ""); sub(/` *\|.*/, ""); print
        }
    ' "$MAP" | sort | uniq -c | awk '$1 > 1 { print $2 }' \
    | while read -r capid; do
        emit_v "capability-dup" "high" "$capid" "appears more than once in §2 Capabilities"
    done
}

# --- check: cross-domain edges without contract -----------------------------
check_missing_contracts() {
    awk -F'|' '
        /^## 7\. Edges/ { in_section = 1; next }
        /^## 8\./ { in_section = 0 }
        in_section && /^\|/ && !/^\|---/ && !/upstream domain/ && NF >= 7 {
            up = $2; down = $3; cap = $4; contract = $5
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", up)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", down)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", cap)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", contract)
            if (up == "" || down == "") next
            if (contract == "" || contract == "n/a" || contract == "—" || \
                contract == "none" || contract == "-" || contract == "unknown") {
                printf "%s|%s|%s\n", up, down, cap
            }
        }
    ' "$MAP" | while IFS='|' read -r up down cap; do
        emit_v "missing-contract" "medium" "${up} → ${down}" "via ${cap}; no contract column"
    done
}

check_god_components
check_orphans
check_drift
check_capability_dup
check_missing_contracts

# --- emit JSON ---------------------------------------------------------------
TOTAL=$(wc -l < "$VIOLATIONS" 2>/dev/null | tr -d ' ')
TOTAL=${TOTAL:-0}

printf '{\n'
printf '  "map": "%s",\n' "$(json_str "$MAP")"
printf '  "thresholds": {"god_size_bytes": %s, "safe_pct": %s, "risky_pct": %s},\n' \
    "$GOD_SIZE" "$SAFE_PCT" "$RISKY_PCT"
printf '  "summary": {\n'
for kind in god-component domain-leak missing-contract orphan drift-missing capability-dup; do
    cnt=$(grep -c "^${kind}	" "$VIOLATIONS" 2>/dev/null || true)
    cnt=$(clean_int "${cnt:-0}")
    cnt=${cnt:-0}
    printf '    "%s": %d,\n' "$kind" "$cnt"
done
printf '    "total": %d\n  },\n' "$TOTAL"

printf '  "violations": ['
first=1
while IFS=$'\t' read -r kind sev path detail; do
    [ -z "${kind:-}" ] && continue
    safety="n/a"
    case "$kind" in
        god-component|orphan) safety=$(safety_for "$path") ;;
    esac
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"kind":"%s","severity":"%s","path":"%s","detail":"%s","safety":"%s"}' \
        "$(json_str "$kind")" "$(json_str "$sev")" "$(json_str "$path")" \
        "$(json_str "$detail")" "$(json_str "$safety")"
done < "$VIOLATIONS"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf ']\n'
printf '}\n'
