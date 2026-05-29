#!/usr/bin/env bash
# setup.sh — install SoTA test framework + coverage tool per detected stack,
# write framework config with coverage thresholds, wire CI workflow.
#
# Coverage and reports are mandatory: every stack gets line-coverage threshold
# enforcement (default 70%) and a JUnit + LCOV report path so CI can consume them.
#
# Usage:
#   bash setup.sh --auto                              # detect + setup all
#   bash setup.sh --stacks js,python                  # subset
#   bash setup.sh --auto --ci github                  # + workflow
#   bash setup.sh --auto --threshold 80               # custom coverage gate
#   bash setup.sh --stacks js --force                 # overwrite existing config

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$SKILL_DIR/templates"
DETECT="$SKILL_DIR/scripts/detect.sh"

STACKS=""
AUTO=0
FORCE=0
CI_PROVIDER=""
THRESHOLD=70

usage() {
    sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --auto)       AUTO=1; shift ;;
        --stacks)     STACKS="$2"; shift 2 ;;
        --force)      FORCE=1; shift ;;
        --ci)         CI_PROVIDER="$2"; shift 2 ;;
        --threshold)  THRESHOLD="$2"; shift 2 ;;
        -h|--help)    usage 0 ;;
        *) echo "unknown arg: $1" >&2; usage 1 ;;
    esac
done

if [ "$AUTO" -eq 1 ] && [ -z "$STACKS" ]; then
    STACKS=$(bash "$DETECT" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | paste -sd, -)
fi
[ -z "$STACKS" ] && { echo "no stacks specified — pass --stacks or --auto" >&2; usage 1; }

echo "→ setup for: $STACKS (coverage threshold ${THRESHOLD}%)" >&2

# write_template <stack> <template-relative-path> <dest-path>
write_template() {
    local stack="$1" src_rel="$2" dst="$3"
    local src="$TEMPLATE_DIR/$stack/$src_rel"
    [ ! -f "$src" ] && { echo "  ! missing template: $src" >&2; return 0; }
    if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
        dst="${dst}.new"
        echo "  → exists; writing to $dst (use --force to overwrite)" >&2
    fi
    mkdir -p "$(dirname "$dst")"
    # Substitute {{THRESHOLD}} with the configured value.
    sed "s/{{THRESHOLD}}/${THRESHOLD}/g" "$src" > "$dst"
    echo "  ✓ wrote $dst" >&2
}

print_install_note() {
    local stack="$1"
    local note="$TEMPLATE_DIR/$stack/install.md"
    [ ! -f "$note" ] && return 0
    echo ""
    echo "── ${stack}: manual install steps ──"
    cat "$note"
    echo ""
}

setup_stack() {
    local stack="$1"
    echo "" >&2
    echo "── ${stack} ──" >&2
    case "$stack" in
        js|ts)
            write_template js config.vitest.ts vitest.config.ts
            print_install_note js
            ;;
        python)
            write_template python pytest.ini pytest.ini
            write_template python coveragerc .coveragerc
            print_install_note python
            ;;
        go)
            # No config file; `go test -cover` is built in. Just print the run cmd.
            print_install_note go
            ;;
        rust)
            write_template rust llvm-cov.toml .config/cargo-llvm-cov.toml
            print_install_note rust
            ;;
        java)
            write_template java jacoco-pom-snippet.xml ut-jacoco-snippet.xml
            print_install_note java
            ;;
        kotlin)
            write_template kotlin kover-snippet.gradle.kts ut-kover-snippet.gradle.kts
            print_install_note kotlin
            ;;
        dotnet)
            write_template dotnet runsettings.xml coverlet.runsettings
            print_install_note dotnet
            ;;
        ruby)
            write_template ruby simplecov.rb .simplecov
            print_install_note ruby
            ;;
        php)
            write_template php phpunit.xml phpunit.xml
            print_install_note php
            ;;
        swift)
            print_install_note swift
            ;;
        shell)
            print_install_note shell
            ;;
        *)
            echo "  ! unknown stack: $stack" >&2
            ;;
    esac
}

IFS=',' read -ra stack_arr <<< "$STACKS"
for s in "${stack_arr[@]}"; do
    [ -z "$s" ] && continue
    setup_stack "$s"
done

# CI workflow
if [ -n "$CI_PROVIDER" ]; then
    src="$TEMPLATE_DIR/ci/$CI_PROVIDER.yaml"
    if [ ! -f "$src" ]; then
        echo "  ! unsupported CI provider: $CI_PROVIDER" >&2
    else
        case "$CI_PROVIDER" in
            github) dst=".github/workflows/unit-tests.yml"; mkdir -p .github/workflows ;;
            gitlab) dst=".gitlab-ci.unit-tests.yml" ;;
            *) dst=""; ;;
        esac
        if [ -n "$dst" ]; then
            if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
                dst="$dst.new"
                echo "→ CI file exists; writing to $dst (use --force to overwrite)" >&2
            fi
            sed "s/{{THRESHOLD}}/${THRESHOLD}/g" "$src" > "$dst"
            echo "✓ wrote $dst" >&2
        fi
    fi
fi

echo "" >&2
echo "next:" >&2
echo "  bash $SKILL_DIR/scripts/run.sh            # run + emit reports" >&2
echo "  bash $SKILL_DIR/scripts/write.sh <src>    # scaffold a test for one source file" >&2
