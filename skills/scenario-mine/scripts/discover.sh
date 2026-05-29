#!/usr/bin/env bash
# discover.sh — enumerate candidate sources for scenario mining.
# Output: JSON listing test files, .feature files, doc files, and route handlers
# grouped by category. The agent feeds these into extract-tests.sh / extract-docs.sh.
#
# Usage:
#   bash discover.sh                          # scan whole repo
#   bash discover.sh --scope app/apply        # restrict to a path
#   bash discover.sh --scope 'app/api/apply*' # glob-aware path filter

set -euo pipefail

ROOT="$(pwd)"
SCOPE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --scope) SCOPE="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,10p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# Directories pruned from every find (build artefacts, vendor code, lockfiles,
# and agent-tooling folders so a skill's own templates don't register as code).
PRUNES=(node_modules .git vendor target build dist .next .venv __pycache__
        .gradle .mvn .cache .pytest_cache .playwright-browsers
        .claude .codex .codex-plugin .claude-plugin .agents)
# NOTE: .s5d is NOT pruned — discover should still find .s5d/scenarios/*.feature
# and .s5d/discovery/architecture-map.md as input. The ~/.agents/skills/*/templates
# false-positive is the actual problem.

prune_expr() {
    local expr=""
    for d in "${PRUNES[@]}"; do
        expr="$expr -name $d -o"
    done
    echo "${expr% -o}"
}

PRUNE_EXPR=$(prune_expr)

# Run find with prune + pattern. $1 = pattern (e.g. '*.test.ts'), prints relative paths.
search() {
    local pattern="$1"
    local base="."
    if [ -n "$SCOPE" ] && [ -e "$SCOPE" ]; then base="$SCOPE"; fi
    # shellcheck disable=SC2086
    find "$base" -type d \( $PRUNE_EXPR \) -prune -o -type f -name "$pattern" -print 2>/dev/null \
        | sort
}

# --- categories ---------------------------------------------------------------

# Portable mapfile alternative for bash 3.2 (macOS).
# Usage: collect VARNAME 'glob1' 'glob2' ...
# Reads search() output for each glob into VARNAME as a newline-separated string;
# the JSON emitter iterates with `while IFS= read`.
collect() {
    local varname="$1"; shift
    local out=""
    local glob line
    for glob in "$@"; do
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            out="${out}${line}"$'\n'
        done < <(search "$glob")
    done
    # Trim trailing newline.
    out="${out%$'\n'}"
    printf -v "$varname" '%s' "$out"
}

collect TESTS_JS     '*.test.ts' '*.test.tsx' '*.test.js' '*.test.jsx' \
                     '*.spec.ts' '*.spec.tsx' '*.spec.js' '*.spec.jsx'
collect TESTS_PY     'test_*.py' '*_test.py' 'conftest.py'
collect TESTS_GO     '*_test.go'
collect TESTS_JAVA   '*Test.java' '*Tests.java'
collect TESTS_KOTLIN '*Test.kt' '*Tests.kt'
collect TESTS_DOTNET '*Test.cs' '*Tests.cs' '*Spec.cs'
collect TESTS_RUBY   '*_spec.rb' '*_test.rb'
collect TESTS_PHP    '*Test.php'
collect TESTS_SWIFT  '*Tests.swift' '*Test.swift'

# Rust tests live in tests/*.rs files and #[test] inline. We list rs files in
# tests/ folders only; inline-test discovery is the LLM's job.
TESTS_RUST=""
while IFS= read -r d; do
    [ -z "$d" ] && continue
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        TESTS_RUST="${TESTS_RUST}${f}"$'\n'
    done < <(find "$d" -type f -name '*.rs' 2>/dev/null)
done < <(find "${SCOPE:-.}" -type d -name 'tests' -not -path '*/node_modules/*' 2>/dev/null)
TESTS_RUST="${TESTS_RUST%$'\n'}"

collect FEATURES '*.feature'
collect DOCS 'README*.md' '*.md'

# API route handlers (framework-specific patterns).
collect API_NEXT 'route.ts' 'route.js'
# FastAPI is harder to detect by name alone; LLM grep'd for app.include_router/@router downstream.
API_FASTAPI=""

# s5d artefacts (anchor scenarios to use_cases/capabilities).
S5D_MAP=""
[ -f .s5d/discovery/architecture-map.md ] && S5D_MAP=".s5d/discovery/architecture-map.md"
S5D_SOURCE_SURVEY=""
[ -f .s5d/discovery/source-survey.md ] && S5D_SOURCE_SURVEY=".s5d/discovery/source-survey.md"
collect S5D_PACKAGES 's5d.yaml'

# --- JSON output --------------------------------------------------------------

# Escape for JSON.
json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

# emit_array takes a LABEL and a NEWLINE-SEPARATED string of items.
emit_array() {
    local label="$1"
    local data="$2"
    printf '    "%s": [' "$label"
    local first=1
    if [ -n "$data" ]; then
        while IFS= read -r v; do
            [ -z "$v" ] && continue
            if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
            printf '      "%s"' "$(json_str "$v")"
        done <<< "$data"
    fi
    if [ "$first" -eq 0 ]; then printf '\n    '; fi
    printf ']'
}

emit_string_or_null() {
    local val="$1"
    if [ -z "$val" ]; then printf 'null'; else printf '"%s"' "$(json_str "$val")"; fi
}

printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "scope": "%s",\n' "$(json_str "${SCOPE:-<repo>}")"
printf '  "tests": {\n'
emit_array js     "$TESTS_JS";    printf ',\n'
emit_array python "$TESTS_PY";    printf ',\n'
emit_array go     "$TESTS_GO";    printf ',\n'
emit_array rust   "$TESTS_RUST";  printf ',\n'
emit_array java   "$TESTS_JAVA";  printf ',\n'
emit_array kotlin "$TESTS_KOTLIN";printf ',\n'
emit_array dotnet "$TESTS_DOTNET";printf ',\n'
emit_array ruby   "$TESTS_RUBY";  printf ',\n'
emit_array php    "$TESTS_PHP";   printf ',\n'
emit_array swift  "$TESTS_SWIFT"; printf '\n'
printf '  },\n'
  emit_array features "$FEATURES"; printf ',\n'
  emit_array docs "$DOCS";         printf ',\n'
printf '  "api": {\n'
emit_array next    "$API_NEXT";    printf ',\n'
emit_array fastapi "$API_FASTAPI"; printf '\n'
printf '  },\n'
printf '  "s5d": {\n'
printf '    "architecture_map": '; emit_string_or_null "$S5D_MAP";          printf ',\n'
printf '    "source_survey": ';    emit_string_or_null "$S5D_SOURCE_SURVEY"; printf ',\n'
emit_array packages "$S5D_PACKAGES"; printf '\n'
printf '  }\n'
printf '}\n'
