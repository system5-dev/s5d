#!/bin/bash
# S5D pre-commit hook for Claude Code plugin
# Called on PreToolUse(Bash) — checks if command is git commit,
# then validates any staged .s5d.yaml files
#
# Input: JSON on stdin with { "tool_input": { "command": "..." } }
# Output: JSON { "decision": "approve" } or { "decision": "block", "reason": "..." }

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('command',''))" 2>/dev/null)

# Only care about git commit commands
if ! echo "$COMMAND" | grep -qE '^\s*git\s+commit'; then
    echo '{"decision":"approve"}'
    exit 0
fi

# Check for staged .s5d.yaml files
CHANGED=$(git diff --cached --name-only 2>/dev/null | grep '\.s5d\.yaml$')

if [ -z "$CHANGED" ]; then
    echo '{"decision":"approve"}'
    exit 0
fi

# Validate each staged spec
FAILED=""
for spec in $CHANGED; do
    if [ -f "$spec" ]; then
        OUTPUT=$(s5d validate "$spec" 2>&1)
        if [ $? -ne 0 ]; then
            FAILED="$FAILED\n  $spec: $OUTPUT"
        fi
    fi
done

if [ -n "$FAILED" ]; then
    echo "{\"decision\":\"block\",\"reason\":\"S5D spec validation failed:$FAILED\"}"
    exit 0
fi

echo '{"decision":"approve"}'
