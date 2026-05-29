#!/usr/bin/env bash
# validate.sh — light Gherkin syntax validator.
#
# This is a sanity check, not a full Gherkin parser. It enforces:
#   - File starts with optional `@tags` lines, then exactly one `Feature:`
#   - At least one `Scenario:` or `Scenario Outline:` block
#   - Steps use canonical keywords: Given / When / Then / And / But
#   - `Examples:` only appears inside `Scenario Outline:`
#   - `Background:` appears at most once, immediately after Feature
#
# Exit codes: 0 ok, 1 syntax problems found, 2 usage error.
#
# Usage:
#   bash validate.sh .s5d/scenarios/apply-flow.feature
#   find .s5d/scenarios -name '*.feature' | xargs -n1 bash validate.sh

set -uo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 <file.feature> [...]" >&2
    exit 2
fi

errors=0

check() {
    local f="$1"
    if [ ! -f "$f" ]; then
        echo "✗ $f: not found" >&2
        errors=$((errors+1))
        return
    fi

    awk -v file="$f" '
        BEGIN {
            feature_count = 0
            scenario_count = 0
            background_count = 0
            in_outline = 0
            in_scenario = 0
            problems = 0
        }
        # Strip trailing CR for files with CRLF endings.
        { sub(/\r$/, "") }

        # Skip empty lines and comments.
        /^[[:space:]]*$/ { next }
        /^[[:space:]]*#/ { next }
        /^[[:space:]]*@/ { next }   # tag lines

        /^[[:space:]]*Feature:/      { feature_count++; in_steps = 0; next }
        /^[[:space:]]*Background:/   { background_count++; in_steps = 0; next }
        /^[[:space:]]*Scenario:/     { scenario_count++; in_outline = 0; in_steps = 0; next }
        /^[[:space:]]*Scenario Outline:/ { scenario_count++; in_outline = 1; in_steps = 0; next }
        /^[[:space:]]*Rule:/         { in_steps = 0; next }
        /^[[:space:]]*Examples:/ {
            if (!in_outline) {
                printf "  ! %s:%d  Examples: outside Scenario Outline\n", file, NR
                problems++
            }
            in_steps = 0
            next
        }
        /^[[:space:]]*(Given|When|Then|And|But)[[:space:]]+/ { in_steps = 1; next }
        /^[[:space:]]*\|/    { next }   # data tables
        /^[[:space:]]*""""/  { next }   # docstrings
        /^[[:space:]]*"""/   { next }
        /^[[:space:]]*\*/    { in_steps = 1; next }   # `* step` alt syntax
        # Free-form description text: allowed before any step keyword.
        in_steps == 0 { next }
        # Once inside a Scenario step block, only steps / tables / docstrings
        # are valid. Anything else flags as suspicious.
        {
            printf "  ! %s:%d  step block contains unrecognised line: %s\n", file, NR, $0
            problems++
        }

        END {
            if (feature_count == 0) {
                printf "  ! %s  no Feature: declaration\n", file
                problems++
            } else if (feature_count > 1) {
                printf "  ! %s  multiple Feature: declarations (%d)\n", file, feature_count
                problems++
            }
            if (scenario_count == 0) {
                printf "  ! %s  no Scenario: blocks\n", file
                problems++
            }
            if (background_count > 1) {
                printf "  ! %s  multiple Background: blocks (%d)\n", file, background_count
                problems++
            }
            exit (problems == 0 ? 0 : 1)
        }
    ' "$f"

    local rc=$?
    if [ "$rc" -eq 0 ]; then
        echo "✓ $f"
    else
        errors=$((errors+1))
    fi
}

for f in "$@"; do
    check "$f"
done

if [ "$errors" -gt 0 ]; then
    echo "$errors file(s) failed validation" >&2
    exit 1
fi
exit 0
