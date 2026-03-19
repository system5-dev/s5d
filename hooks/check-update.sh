#!/bin/bash
# S5D update checker — runs on SessionStart
# Compares installed version against latest GitHub release
# Non-blocking: outputs systemMessage if update available, silent otherwise

REPO="system5-dev/s5d"
CURRENT_VERSION="2.0.0"

# Quick check — skip if no network or gh not available
if ! command -v gh &>/dev/null; then
    exit 0
fi

LATEST=$(gh api "repos/$REPO/releases/latest" --jq '.tag_name' 2>/dev/null | sed 's/^v//')

if [ -z "$LATEST" ]; then
    exit 0
fi

if [ "$CURRENT_VERSION" != "$LATEST" ]; then
    cat <<EOF
{"systemMessage":"S5D update available: $CURRENT_VERSION → $LATEST. Run: curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash"}
EOF
fi
