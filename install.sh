#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/bin"

# ── Auto-update: pull latest if in a git repo ────────────────────────────────
if [ -d "$SCRIPT_DIR/.git" ]; then
    CURRENT=$(git -C "$SCRIPT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
    echo "→ Pulling latest from git..."
    git -C "$SCRIPT_DIR" pull --ff-only 2>/dev/null && {
        UPDATED=$(git -C "$SCRIPT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
        if [ "$CURRENT" != "$UPDATED" ]; then
            echo "  ✓ Updated $CURRENT → $UPDATED"
            # Re-exec with the updated install.sh
            exec "$SCRIPT_DIR/install.sh" "$@"
        else
            echo "  ✓ Already up to date ($CURRENT)"
        fi
    } || echo "  ⚠ git pull failed (offline?), installing from local copy"
fi

echo "Installing S5D..."

# 1. Skills — symlink into vendor-agnostic location + agent runtimes
#    Source of truth: $SCRIPT_DIR/skills/
SKILLS="s5d fpf"

link_skill() {
    local target_dir="$1"
    local skill_name="$2"
    local src="$SCRIPT_DIR/skills/$skill_name"
    # Remove old copy/link, replace with symlink
    rm -rf "$target_dir/$skill_name"
    ln -sf "$src" "$target_dir/$skill_name"
}

# Vendor-agnostic canonical location (always installed)
mkdir -p "${HOME}/.agents/skills"
for skill in $SKILLS; do
    link_skill "${HOME}/.agents/skills" "$skill"
done
echo "✓ Skills linked to ~/.agents/skills/ (vendor-agnostic)"

# Claude Code — symlink from vendor-agnostic
if [ -d "${HOME}/.claude" ] || command -v claude &>/dev/null; then
    mkdir -p "${HOME}/.claude/skills"
    for skill in $SKILLS; do
        link_skill "${HOME}/.claude/skills" "$skill"
    done
    echo "✓ Skills linked for Claude Code (~/.claude/skills/)"
fi

# Codex CLI
if [ -d "${HOME}/.codex" ] || command -v codex &>/dev/null; then
    mkdir -p "${HOME}/.codex/skills"
    for skill in $SKILLS; do
        link_skill "${HOME}/.codex/skills" "$skill"
    done
    echo "✓ Skills linked for Codex CLI (~/.codex/skills/)"
fi

# Gemini CLI
if [ -d "${HOME}/.gemini" ] || command -v gemini &>/dev/null; then
    mkdir -p "${HOME}/.gemini/skills"
    for skill in $SKILLS; do
        link_skill "${HOME}/.gemini/skills" "$skill"
    done
    echo "✓ Skills linked for Gemini CLI (~/.gemini/skills/)"
fi

# Diana
if [ -d "${HOME}/.diana/src/skills" ]; then
    for skill in $SKILLS; do
        link_skill "${HOME}/.diana/src/skills" "$skill"
    done
    echo "✓ Skills linked for Diana (~/.diana/src/skills/)"
fi

echo ""
echo "  Canonical location: ~/.agents/skills/"
echo "  Agent runtimes get symlinks for compatibility."

# 2. Binary — prebuilt or build from source
mkdir -p "$BIN_DIR"
ARCH="$(uname -s)-$(uname -m)"
PREBUILT="$SCRIPT_DIR/bin/s5d-$(echo "$ARCH" | tr '[:upper:]' '[:lower:]')"

if [ -f "$PREBUILT" ]; then
    cp "$PREBUILT" "$BIN_DIR/s5d"
    chmod +x "$BIN_DIR/s5d"
    echo "✓ Binary installed from prebuilt ($ARCH)"
elif command -v cargo &> /dev/null; then
    echo "No prebuilt for $ARCH, building from source..."
    cd "$SCRIPT_DIR/rust" && cargo build --release
    cp "$SCRIPT_DIR/rust/target/release/s5d" "$BIN_DIR/s5d"
    echo "✓ Binary built and installed"
else
    echo "⚠ No prebuilt binary for $ARCH and no Rust toolchain. Skills installed, CLI skipped."
    echo "  Install Rust (rustup.rs) and re-run. Skills are installed, CLI skipped."
fi

echo ""
echo "What was installed:"
echo "  s5d binary — CLI for decisions, features, gates"
echo "  s5d skills — /s5d and /fpf for agent runtimes"
echo ""
echo "MCP server is registered per-project by 's5d init' (.mcp.json)."
echo "Marketplace installs (Claude/Gemini/Codex) register MCP globally via manifest."
echo ""
echo "Optional: install pre-commit hook in any project:"
echo "  cp $SCRIPT_DIR/hooks/s5d-validate.sh <project>/.git/hooks/pre-commit"
echo ""
echo "Usage: /s5d <problem>  or  s5d --help"
