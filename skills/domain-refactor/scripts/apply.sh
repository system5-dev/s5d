#!/usr/bin/env bash
# apply.sh — execute ONE refactor slice from refactor-plan.md.
#
# This script is deliberately conservative. v1 supports just two
# fully-mechanical operations:
#   - `map-update <path-to-map-edit-script>`
#     Apply a sed/awk edit script to .s5d/discovery/architecture-map.md only.
#   - `move-component <src> <dst>`
#     Move a single source file, update all imports (best-effort grep+sed),
#     update the architecture-map row in the same operation.
#
# Anything more complex (god-component decomposition) is NOT auto-applied —
# print a plan and ask the human to do the move manually with apply.sh
# move-component calls per resulting file.
#
# Usage:
#   bash apply.sh map-update path/to/edit-script.sed
#   bash apply.sh move-component utils/old.ts utils/sub/new.ts
#
# After every apply.sh call, you MUST run:
#   bash ~/.agents/skills/domain-refactor/scripts/verify.sh

set -uo pipefail

MAP=".s5d/discovery/architecture-map.md"

usage() {
    sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    exit "${1:-0}"
}

[ $# -lt 1 ] && usage 1
OP="$1"; shift

# Gate: working tree must be clean — refuse to apply on top of unrelated changes.
gate_clean_tree() {
    if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
        echo "✗ working tree is dirty — commit or stash first" >&2
        git status --short >&2
        exit 1
    fi
}

# Gate: baseline must exist — verify.sh comparison depends on it.
gate_baseline() {
    if [ ! -f .s5d/refactor-baseline/state.txt ]; then
        echo "✗ no refactor baseline captured" >&2
        echo "  run: bash ~/.agents/skills/domain-refactor/scripts/verify.sh --capture" >&2
        exit 1
    fi
}

case "$OP" in
    map-update)
        [ $# -ne 1 ] && { echo "usage: apply.sh map-update <sed-script>" >&2; exit 1; }
        gate_clean_tree
        local_script="$1"
        [ ! -f "$local_script" ] && { echo "no script at $local_script" >&2; exit 1; }
        echo "→ applying sed script to $MAP" >&2
        sed -i.bak -f "$local_script" "$MAP"
        rm -f "${MAP}.bak"
        echo "✓ map updated; review with: git diff -- $MAP" >&2
        ;;

    move-component)
        [ $# -ne 2 ] && { echo "usage: apply.sh move-component <src> <dst>" >&2; exit 1; }
        SRC="$1"; DST="$2"
        gate_clean_tree
        gate_baseline
        [ ! -f "$SRC" ] && { echo "source not found: $SRC" >&2; exit 1; }
        [ -e "$DST" ] && { echo "destination exists: $DST — refusing to overwrite" >&2; exit 1; }
        mkdir -p "$(dirname "$DST")"
        git mv "$SRC" "$DST"

        # Best-effort import rewrite for JS/TS. Bash 3.2-safe.
        echo "→ rewriting imports (best-effort)" >&2
        old_no_ext="${SRC%.*}"
        new_no_ext="${DST%.*}"
        # Search for imports of the old path; rewrite to the new path.
        # Limit to source dirs, prune build/agent-tooling.
        find app components lib utils hooks store -type f \
            \( -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' \) 2>/dev/null \
        | while IFS= read -r f; do
            if grep -q -F "$old_no_ext" "$f" 2>/dev/null; then
                sed -i.bak "s|${old_no_ext}|${new_no_ext}|g" "$f" && rm -f "$f.bak"
                echo "  ↻ rewrote import in $f" >&2
            fi
        done

        # Update architecture-map row, if present.
        if grep -q -F "\`$SRC\`" "$MAP" 2>/dev/null; then
            sed -i.bak "s|\`$SRC\`|\`$DST\`|g" "$MAP" && rm -f "$MAP.bak"
            echo "  ↻ updated map row" >&2
        fi

        echo "✓ moved $SRC → $DST" >&2
        echo "  next: bash ~/.agents/skills/domain-refactor/scripts/verify.sh" >&2
        ;;

    *)
        echo "unknown operation: $OP" >&2
        usage 1
        ;;
esac
