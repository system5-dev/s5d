#!/bin/bash
# S5D pre-commit validation hook
# Validates any changed .s5d.yaml files before commit
# Install: cp hooks/s5d-validate.sh .git/hooks/pre-commit

CHANGED=$(git diff --cached --name-only 2>/dev/null | grep '\.s5d\.yaml$')

if [ -z "$CHANGED" ]; then
    exit 0
fi

FAILED=0
for spec in $CHANGED; do
    if [ -f "$spec" ]; then
        if ! s5d validate "$spec" > /dev/null 2>&1; then
            echo "S5D validation failed: $spec" >&2
            FAILED=1
        fi
    fi
done

exit $FAILED
