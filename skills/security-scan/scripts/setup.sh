#!/usr/bin/env bash
# setup.sh — install configs per security category, wire CI workflow.
#
# Each category gets a tool-native config (semgrep.yml, trivy.yaml,
# .gitleaks.toml, etc.). The skill does NOT install the binaries —
# install instructions printed per category, with brew / curl one-liners.
#
# Usage:
#   bash setup.sh --auto                    # detect + setup all enabled
#   bash setup.sh --categories sast,secrets
#   bash setup.sh --auto --ci github
#   bash setup.sh --auto --force            # overwrite existing configs

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$SKILL_DIR/templates"
DETECT="$SKILL_DIR/scripts/detect.sh"

CATEGORIES=""
AUTO=0
FORCE=0
CI_PROVIDER=""

while [ $# -gt 0 ]; do
    case "$1" in
        --auto)        AUTO=1; shift ;;
        --categories)  CATEGORIES="$2"; shift 2 ;;
        --force)       FORCE=1; shift ;;
        --ci)          CI_PROVIDER="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,10p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ "$AUTO" -eq 1 ] && [ -z "$CATEGORIES" ]; then
    CATEGORIES=$(bash "$DETECT" \
        | python3 -c "import json,sys; d=json.load(sys.stdin); print(','.join(c['id'] for c in d['categories'] if c['enabled']))")
fi
[ -z "$CATEGORIES" ] && { echo "no categories — pass --categories or --auto" >&2; exit 2; }

echo "→ setup for: $CATEGORIES" >&2

write_template() {
    local cat="$1" src_rel="$2" dst="$3"
    local src="$TEMPLATE_DIR/$cat/$src_rel"
    [ ! -f "$src" ] && { echo "  ! missing template: $src" >&2; return 0; }
    if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
        dst="${dst}.new"
        echo "  → exists; writing to $dst" >&2
    fi
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    echo "  ✓ wrote $dst" >&2
}

print_install() {
    local cat="$1"
    local note="$TEMPLATE_DIR/$cat/install.md"
    [ ! -f "$note" ] && return 0
    echo ""
    echo "── ${cat}: install ──"
    cat "$note"
    echo ""
}

setup_category() {
    local cat="$1"
    echo "" >&2
    echo "── ${cat} ──" >&2
    case "$cat" in
        sast)
            write_template sast semgrep.yml .semgrep.yml
            print_install sast
            ;;
        sca)
            write_template sca trivy.yaml .trivy.yaml
            print_install sca
            ;;
        secrets)
            write_template secrets gitleaks.toml .gitleaks.toml
            print_install secrets
            ;;
        container)
            write_template container trivy-image.yaml .trivy-image.yaml
            print_install container
            ;;
        iac)
            write_template iac trivy-config.yaml .trivy-config.yaml
            print_install iac
            ;;
        license)
            write_template license trivy-license.yaml .trivy-license.yaml
            print_install license
            ;;
        sbom)
            write_template sbom syft.yaml .syft.yaml
            print_install sbom
            ;;
        *)
            echo "  ! unknown category: $cat" >&2 ;;
    esac
}

IFS=',' read -ra cat_arr <<< "$CATEGORIES"
for c in "${cat_arr[@]}"; do
    [ -z "$c" ] && continue
    setup_category "$c"
done

# Waivers store — write once, never overwrite
if [ ! -f .s5d/security-waivers.yaml ]; then
    mkdir -p .s5d
    write_template "" waivers.yaml .s5d/security-waivers.yaml 2>/dev/null || \
        cp "$TEMPLATE_DIR/waivers.yaml" .s5d/security-waivers.yaml 2>/dev/null
    [ -f .s5d/security-waivers.yaml ] && echo "" >&2 && echo "✓ wrote .s5d/security-waivers.yaml" >&2
fi

# CI workflow
if [ -n "$CI_PROVIDER" ]; then
    src="$TEMPLATE_DIR/ci/$CI_PROVIDER.yaml"
    if [ ! -f "$src" ]; then
        echo "  ! unsupported CI: $CI_PROVIDER" >&2
    else
        case "$CI_PROVIDER" in
            github) dst=".github/workflows/security-scan.yml"; mkdir -p .github/workflows ;;
            gitlab) dst=".gitlab-ci.security-scan.yml" ;;
            *) dst="";;
        esac
        if [ -n "$dst" ]; then
            if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
                dst="${dst}.new"
                echo "→ CI file exists; writing to $dst" >&2
            fi
            cp "$src" "$dst"
            echo "✓ wrote $dst" >&2
        fi
    fi
fi

echo "" >&2
echo "next:" >&2
echo "  bash $SKILL_DIR/scripts/run.sh                  # run + emit SARIF + severity gate" >&2
echo "  bash $SKILL_DIR/scripts/waiver.sh list          # see active waivers" >&2
echo "  bash $SKILL_DIR/scripts/triage.sh               # dedup CVE across scanners" >&2
