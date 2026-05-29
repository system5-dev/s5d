#!/usr/bin/env bash
# verify.sh — run pre-commit against all files, surface findings.
#
# Usage:
#   bash verify.sh                  # run all hooks on all files
#   bash verify.sh --staged         # run on staged files only
#   bash verify.sh --hook biome     # run a single hook id

set -euo pipefail

MODE="all"
HOOK_ID=""

while [ $# -gt 0 ]; do
    case "$1" in
        --staged) MODE="staged"; shift ;;
        --hook)   HOOK_ID="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,8p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ ! -f .pre-commit-config.yaml ]; then
    echo "no .pre-commit-config.yaml at repo root. Run setup.sh first." >&2
    exit 2
fi

PRECOMMIT_BIN=""
for cand in pre-commit "uvx pre-commit" "pipx run pre-commit"; do
    if command -v ${cand%% *} >/dev/null 2>&1; then
        PRECOMMIT_BIN="$cand"
        break
    fi
done
[ -z "$PRECOMMIT_BIN" ] && { echo "pre-commit not installed. Run setup.sh." >&2; exit 2; }

args=()
case "$MODE" in
    all)    args+=(run --all-files) ;;
    staged) args+=(run) ;;
esac
[ -n "$HOOK_ID" ] && args+=("$HOOK_ID")

echo "→ $PRECOMMIT_BIN ${args[*]}" >&2
$PRECOMMIT_BIN "${args[@]}"
