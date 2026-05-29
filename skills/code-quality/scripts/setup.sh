#!/usr/bin/env bash
# setup.sh — compose .pre-commit-config.yaml from per-stack templates,
# install pre-commit framework, wire git hook.
#
# Usage:
#   bash setup.sh --stacks js,python,go [--output .pre-commit-config.yaml] [--force]
#   bash setup.sh --auto                          # detect + setup all detected stacks
#   bash setup.sh --auto --skip yaml,markdown     # detect, then drop noisy stacks
#   bash setup.sh --auto --ci github              # also write GitHub Actions workflow
#   bash setup.sh --ci-only github                # only write CI workflow (skip pre-commit)
# Supported --ci values: github, gitlab
#
# Deterministic. Idempotent. Does not install language-specific dev deps (Biome,
# Ruff, etc.) — that's a separate per-stack decision because it touches package.json
# / pyproject.toml / etc. The agent handles those after calling this script.

set -euo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$SKILL_DIR/templates"
DETECT="$SKILL_DIR/scripts/detect.sh"

STACKS=""
OUTPUT=".pre-commit-config.yaml"
FORCE=0
AUTO=0
SKIP_CSV=""
CI_PROVIDER=""
CI_ONLY=0

usage() {
    sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --stacks)  STACKS="$2"; shift 2 ;;
        --output)  OUTPUT="$2"; shift 2 ;;
        --skip)    SKIP_CSV="$2"; shift 2 ;;
        --force)   FORCE=1; shift ;;
        --auto)    AUTO=1; shift ;;
        --ci)      CI_PROVIDER="$2"; shift 2 ;;
        --ci-only) CI_PROVIDER="$2"; CI_ONLY=1; shift 2 ;;
        -h|--help) usage 0 ;;
        *)         echo "unknown arg: $1" >&2; usage 1 ;;
    esac
done

# --- resolve stack list -------------------------------------------------------

extract_ids_from_json() {
    # Pull "id":"..." values from detect.sh output. Pure sed, no jq.
    sed -n 's/.*"id":"\([^"]*\)".*/\1/p'
}

if [ "$AUTO" -eq 1 ] && [ -z "$STACKS" ]; then
    STACKS=$(bash "$DETECT" | extract_ids_from_json | paste -sd, -)
fi

# CI-only flow: skip stack composition, jump straight to CI writer.
if [ "$CI_ONLY" -eq 1 ]; then
    if [ -z "$CI_PROVIDER" ]; then echo "--ci-only requires a provider name" >&2; exit 2; fi
    : # fall through to CI block below
elif [ -z "$STACKS" ]; then
    echo "no stacks specified — pass --stacks or --auto" >&2
    usage 1
fi

# Apply --skip filter
if [ -n "$SKIP_CSV" ]; then
    keep=""
    IFS=',' read -ra wanted <<< "$STACKS"
    IFS=',' read -ra skip <<< "$SKIP_CSV"
    for s in "${wanted[@]}"; do
        keep_it=1
        for sk in "${skip[@]}"; do [ "$s" = "$sk" ] && keep_it=0 && break; done
        [ "$keep_it" -eq 1 ] && keep="${keep:+$keep,}$s"
    done
    STACKS="$keep"
fi

# `ts` is a sub-stack of `js` — both map to the same template.
# De-dup ts when js is present.
case ",$STACKS," in
    *,ts,*) case ",$STACKS," in *,js,*) STACKS=$(echo "$STACKS" | tr ',' '\n' | grep -v '^ts$' | paste -sd, -) ;; esac ;;
esac

echo "stacks: $STACKS" >&2

# --- output guard -------------------------------------------------------------

if [ -e "$OUTPUT" ] && [ "$FORCE" -ne 1 ]; then
    OUTPUT_NEW="$OUTPUT.new"
    echo "→ $OUTPUT exists; writing to $OUTPUT_NEW instead (use --force to overwrite)" >&2
    OUTPUT="$OUTPUT_NEW"
fi

# --- compose ------------------------------------------------------------------

emit() {
    local name="$1"
    local path="$TEMPLATE_DIR/$name.yaml"
    if [ ! -f "$path" ]; then
        echo "  ! template $name.yaml missing — skipping" >&2
        return 0
    fi
    cat "$path"
    printf '\n'
}

{
    emit header
    emit generic
    IFS=',' read -ra stack_arr <<< "$STACKS"
    for stack in "${stack_arr[@]}"; do
        [ -z "$stack" ] && continue
        emit "$stack"
    done
} > "$OUTPUT"

echo "✓ wrote $OUTPUT ($(wc -l < "$OUTPUT" | tr -d ' ') lines)" >&2

# --- pre-commit framework -----------------------------------------------------

PRECOMMIT_BIN=""
if command -v pre-commit >/dev/null 2>&1; then
    PRECOMMIT_BIN="pre-commit"
elif command -v uvx >/dev/null 2>&1; then
    PRECOMMIT_BIN="uvx pre-commit"
elif command -v pipx >/dev/null 2>&1; then
    PRECOMMIT_BIN="pipx run pre-commit"
fi

if [ -z "$PRECOMMIT_BIN" ]; then
    cat >&2 <<EOF

! pre-commit is not installed. Install with one of:
    brew install pre-commit
    uv tool install pre-commit
    pipx install pre-commit
    pip install --user pre-commit
  Then run: pre-commit install
EOF
    exit 0
fi

# Only install hook if config is the live one (not .new) and we're inside a git repo.
if [ "$OUTPUT" = ".pre-commit-config.yaml" ] && git rev-parse --git-dir >/dev/null 2>&1; then
    echo "→ installing git hook via $PRECOMMIT_BIN" >&2
    $PRECOMMIT_BIN install >&2
    echo "✓ git hook installed (run 'pre-commit run --all-files' for baseline)" >&2
else
    echo "→ skipping 'pre-commit install' (config written to $OUTPUT, not active path)" >&2
fi

# --- CI workflow (optional) ---------------------------------------------------

write_ci() {
    local provider="$1"
    local src="$TEMPLATE_DIR/ci/$provider.yaml"
    if [ ! -f "$src" ]; then
        echo "! unknown CI provider: $provider (templates/ci/$provider.yaml missing)" >&2
        return 1
    fi
    local dst
    case "$provider" in
        github) dst=".github/workflows/code-quality.yml"; mkdir -p .github/workflows ;;
        gitlab) dst=".gitlab-ci.code-quality.yml" ;;  # fragment; user merges into main pipeline
        *) echo "! unsupported provider: $provider" >&2; return 1 ;;
    esac
    if [ -e "$dst" ] && [ "$FORCE" -ne 1 ]; then
        dst="$dst.new"
        echo "→ CI file exists; writing to $dst (use --force to overwrite)" >&2
    fi
    cp "$src" "$dst"
    echo "✓ wrote $dst" >&2
}

if [ -n "$CI_PROVIDER" ]; then
    write_ci "$CI_PROVIDER"
    if [ "$CI_PROVIDER" = "gitlab" ]; then
        echo "  → gitlab-ci.yml is a fragment. Merge into your main pipeline via 'include:'." >&2
    fi
fi
