#!/usr/bin/env bash
# setup.sh — install SoTA e2e framework per axis, write configs with
# mandatory reports (HTML, JUnit XML, traces+videos on failure), wire CI.
#
# Usage:
#   bash setup.sh --auto                         # detect + setup all axes
#   bash setup.sh --axes web,api                 # subset
#   bash setup.sh --auto --ci github             # + workflow
#   bash setup.sh --auto --base-url http://localhost:3000
#   bash setup.sh --axes web --force             # overwrite existing config

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$SKILL_DIR/templates"
DETECT="$SKILL_DIR/scripts/detect.sh"

AXES=""
AUTO=0
FORCE=0
CI_PROVIDER=""
BASE_URL="http://localhost:3000"

usage() {
    sed -n '2,10p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --auto)       AUTO=1; shift ;;
        --axes)       AXES="$2"; shift 2 ;;
        --force)      FORCE=1; shift ;;
        --ci)         CI_PROVIDER="$2"; shift 2 ;;
        --base-url)   BASE_URL="$2"; shift 2 ;;
        -h|--help)    usage 0 ;;
        *) echo "unknown arg: $1" >&2; usage 1 ;;
    esac
done

if [ "$AUTO" -eq 1 ] && [ -z "$AXES" ]; then
    AXES=$(bash "$DETECT" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | grep -E '^(web|api|mobile)$' | paste -sd, -)
fi
[ -z "$AXES" ] && { echo "no axes specified — pass --axes or --auto" >&2; usage 1; }

echo "→ setup for axes: $AXES (base url $BASE_URL)" >&2

write_template() {
    local axis="$1" src_rel="$2" dst="$3"
    local src="$TEMPLATE_DIR/$axis/$src_rel"
    [ ! -f "$src" ] && { echo "  ! missing template: $src" >&2; return 0; }
    if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
        dst="${dst}.new"
        echo "  → exists; writing to $dst (use --force to overwrite)" >&2
    fi
    mkdir -p "$(dirname "$dst")"
    sed -e "s|{{BASE_URL}}|$BASE_URL|g" "$src" > "$dst"
    echo "  ✓ wrote $dst" >&2
}

print_install() {
    local axis="$1"
    local note="$TEMPLATE_DIR/$axis/install.md"
    [ ! -f "$note" ] && return 0
    echo ""
    echo "── ${axis}: install notes ──"
    cat "$note"
    echo ""
}

setup_axis() {
    local axis="$1"
    echo "" >&2
    echo "── ${axis} ──" >&2
    case "$axis" in
        web)
            write_template web playwright.config.ts playwright.config.ts
            mkdir -p e2e
            write_template web stub.e2e.spec.ts e2e/example.spec.ts
            print_install web
            ;;
        api)
            # If web/playwright already chosen, hurl is an optional adjunct.
            mkdir -p e2e/api
            write_template api stub.hurl e2e/api/example.hurl
            write_template api stub.api.spec.ts e2e/api/example.api.spec.ts
            print_install api
            ;;
        mobile)
            mkdir -p .maestro
            write_template mobile flow.yaml .maestro/example.yaml
            print_install mobile
            ;;
        *)
            echo "  ! unknown axis: $axis" >&2 ;;
    esac
}

IFS=',' read -ra axis_arr <<< "$AXES"
for a in "${axis_arr[@]}"; do
    [ -z "$a" ] && continue
    setup_axis "$a"
done

# CI workflow
if [ -n "$CI_PROVIDER" ]; then
    src="$TEMPLATE_DIR/ci/$CI_PROVIDER.yaml"
    if [ ! -f "$src" ]; then
        echo "  ! unsupported CI: $CI_PROVIDER" >&2
    else
        case "$CI_PROVIDER" in
            github) dst=".github/workflows/e2e-tests.yml"; mkdir -p .github/workflows ;;
            gitlab) dst=".gitlab-ci.e2e-tests.yml" ;;
            *) dst="";;
        esac
        if [ -n "$dst" ]; then
            if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
                dst="${dst}.new"
                echo "→ CI file exists; writing to $dst" >&2
            fi
            sed -e "s|{{BASE_URL}}|$BASE_URL|g" "$src" > "$dst"
            echo "✓ wrote $dst" >&2
        fi
    fi
fi

echo "" >&2
echo "next:" >&2
echo "  bash $SKILL_DIR/scripts/run.sh                     # run + emit reports" >&2
echo "  bash $SKILL_DIR/scripts/coverage.sh                # use-case coverage report" >&2
echo "  bash $SKILL_DIR/scripts/write.sh <use-case-id>     # scaffold spec from .s5d/scenarios" >&2
