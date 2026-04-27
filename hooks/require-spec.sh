#!/bin/bash
# S5D spec-required hook for Claude Code
# Blocks git commit if change is non-trivial (>30 LOC or >1 source file)
# and no S5D spec exists for the project.
#
# "Non-trivial" = not just config/docs/tests
# Input: JSON on stdin { "tool_input": { "command": "..." } }
# Output: JSON { "decision": "approve" } or { "decision": "block", "reason": "..." }

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('command',''))" 2>/dev/null)

# Only care about git commit
if ! echo "$COMMAND" | grep -qE '^\s*git\s+commit'; then
    echo '{"decision":"approve"}'
    exit 0
fi

# Skip if no .s5d/ directory (S5D not initialized in this project)
if [ ! -d ".s5d/packages" ]; then
    echo '{"decision":"approve"}'
    exit 0
fi

# Count staged source files (exclude config, docs, tests, lockfiles)
SOURCE_FILES=$(git diff --cached --name-only 2>/dev/null | grep -vE '\.(md|txt|json|yaml|yml|toml|lock|config|env)$|^\.|(test|spec|fixture)' | wc -l | tr -d ' ')

# Count staged LOC changes
LOC_DELTA=$(git diff --cached --stat 2>/dev/null | tail -1 | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+')
LOC_DELTA=${LOC_DELTA:-0}

# Trivial change → allow
if [ "$SOURCE_FILES" -le 1 ] && [ "$LOC_DELTA" -le 30 ]; then
    echo '{"decision":"approve"}'
    exit 0
fi

# Non-trivial: check if any non-proposed/non-decision spec exists
# (proposed specs are scaffolds — we need at least one with artifacts)
HAS_FEATURE_SPEC=$(find .s5d/packages -name "feat.*.s5d.yaml" 2>/dev/null | head -1)
HAS_RECORD_APPLIED=$(find .s5d/records -name "*.record.yaml" -exec grep -l "status: applied\|status: operated\|status: approved" {} \; 2>/dev/null | head -1)

# Check commit message for spec reference
COMMIT_MSG=""
if echo "$COMMAND" | grep -qE '\-m\s'; then
    COMMIT_MSG=$(echo "$COMMAND" | sed 's/.*-m[[:space:]]*["'"'"']//' | sed 's/["'"'"'].*//')
fi
HAS_SPEC_REF=$(echo "$COMMIT_MSG" | grep -oE 'S5D-Spec:|spec://|Implements:|feat\.' | head -1)

if [ -n "$HAS_FEATURE_SPEC" ] || [ -n "$HAS_RECORD_APPLIED" ] || [ -n "$HAS_SPEC_REF" ]; then
    echo '{"decision":"approve"}'
    exit 0
fi

echo "{\"decision\":\"block\",\"reason\":\"Non-trivial change (${SOURCE_FILES} source files, +${LOC_DELTA} LOC) without S5D spec. Run /s5d to create a spec first, or add 'S5D-Spec: <spec-id>' to commit message if spec exists elsewhere.\"}"
exit 0
