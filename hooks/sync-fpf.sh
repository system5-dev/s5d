#!/bin/bash
# Sync FPF content from upstream into S5D SKILL.md
# Usage: bash hooks/sync-fpf.sh [s5d-root]
#
# Pulls latest FPF markdown from GitHub, extracts the thinking discipline
# section, and patches it into skills/s5d/SKILL.md

set -e

S5D_ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILL_FILE="$S5D_ROOT/skills/s5d/SKILL.md"
FPF_REPO="ailev/FPF"
FPF_BRANCH="main"

if [ ! -f "$SKILL_FILE" ]; then
    echo "Error: SKILL.md not found at $SKILL_FILE" >&2
    exit 1
fi

echo "→ Fetching latest FPF from $FPF_REPO..."

# Fetch FPF README
FPF_CONTENT=$(gh api "repos/$FPF_REPO/contents/README.md" --jq '.content' 2>/dev/null | base64 -d 2>/dev/null)

if [ -z "$FPF_CONTENT" ]; then
    echo "⚠ Could not fetch FPF content (offline or repo private?)" >&2
    exit 1
fi

# Extract current FPF section hash from SKILL.md for comparison
CURRENT_HASH=$(sed -n '/^## Thinking discipline/,/^## [^T]/p' "$SKILL_FILE" | shasum -a 256 | cut -d' ' -f1)

echo "  Current FPF section hash: ${CURRENT_HASH:0:12}"
echo "  FPF upstream fetched: $(echo "$FPF_CONTENT" | wc -l | tr -d ' ') lines"
echo ""
echo "  FPF content saved to: /tmp/fpf-latest.md"
echo "  Review and manually update the Thinking discipline section in SKILL.md"
echo "  (Auto-patch not implemented — FPF structure varies too much for safe automation)"

echo "$FPF_CONTENT" > /tmp/fpf-latest.md
