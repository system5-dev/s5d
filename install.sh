#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="${HOME}/bin"
REQUIRED_PATHS=(
    "$SCRIPT_DIR/skills"
    "$SCRIPT_DIR/rust"
    "$SCRIPT_DIR/hooks"
)

for required in "${REQUIRED_PATHS[@]}"; do
    if [ ! -e "$required" ]; then
        cat >&2 <<EOF
ERROR: install.sh must be run from a checked-out S5D repository copy.

This script does not support 'curl | bash' because it needs the local repo files:
  - skills/
  - rust/
  - hooks/

Safe install:
  git clone https://github.com/system5-dev/s5d.git
  cd s5d
  ./install.sh
EOF
        exit 1
    fi
done

echo "Installing S5D..."

# 1. Skills — symlink into vendor-agnostic location + agent runtimes
#    Source of truth: $SCRIPT_DIR/skills/
SKILLS="s5d code-quality unit-tests e2e-tests security-scan domain-refactor scaling-review system-design scenario-mine infra-scan ai-technical-writer ddd-refactor"
RETIRED_SKILLS="fpf fpf-modules domain-capability-design"

link_skill() {
    local target_dir="$1"
    local skill_name="$2"
    local src="$SCRIPT_DIR/skills/$skill_name"
    # Remove old copy/link, replace with symlink
    rm -rf "$target_dir/$skill_name"
    ln -sf "$src" "$target_dir/$skill_name"
}

cleanup_retired_skills() {
    local target_dir="$1"
    for skill_name in $RETIRED_SKILLS; do
        rm -rf "$target_dir/$skill_name"
    done
}

# Vendor-agnostic canonical location (always installed)
mkdir -p "${HOME}/.agents/skills"
cleanup_retired_skills "${HOME}/.agents/skills"
for skill in $SKILLS; do
    link_skill "${HOME}/.agents/skills" "$skill"
done
echo "✓ Skills linked to ~/.agents/skills/ (vendor-agnostic)"

# Claude Code — symlink from vendor-agnostic
if [ -d "${HOME}/.claude" ] || command -v claude &>/dev/null; then
    mkdir -p "${HOME}/.claude/skills"
    cleanup_retired_skills "${HOME}/.claude/skills"
    for skill in $SKILLS; do
        link_skill "${HOME}/.claude/skills" "$skill"
    done
    echo "✓ Skills linked for Claude Code (~/.claude/skills/)"
fi

# Codex CLI + Gemini CLI: skipped intentionally — both read ~/.agents/skills
# natively. Duplicating into ~/.codex/skills and ~/.gemini/skills triggers
# "Skill conflict detected" warnings via double-load. Use codex-plugin /
# gemini-extension.json manifests for native discovery instead.

# Diana
if [ -d "${HOME}/.diana/src/skills" ]; then
    cleanup_retired_skills "${HOME}/.diana/src/skills"
    for skill in $SKILLS; do
        link_skill "${HOME}/.diana/src/skills" "$skill"
    done
    echo "✓ Skills linked for Diana (~/.diana/src/skills/)"
fi

echo ""
echo "  Canonical location: ~/.agents/skills/"
echo "  Agent runtimes get symlinks for compatibility."

# 1b. Agents shipped with the skill — symlink agent definitions into runtimes
#     that auto-discover them. Agents live next to SKILL.md in skills/<name>/agents/.
link_skill_agents() {
    local target_dir="$1"
    local skill_name="$2"
    local agents_src="$SCRIPT_DIR/skills/$skill_name/agents"
    [ -d "$agents_src" ] || return 0
    mkdir -p "$target_dir"
    local agent_file
    for agent_file in "$agents_src"/*.md; do
        [ -f "$agent_file" ] || continue
        local name="$(basename "$agent_file")"
        rm -f "$target_dir/$name"
        ln -sf "$agent_file" "$target_dir/$name"
    done
}

# Vendor-agnostic agents location
for skill in $SKILLS; do
    link_skill_agents "${HOME}/.agents/agents" "$skill"
done

# Claude Code auto-discovers agents in ~/.claude/agents/
if [ -d "${HOME}/.claude" ] || command -v claude &>/dev/null; then
    for skill in $SKILLS; do
        link_skill_agents "${HOME}/.claude/agents" "$skill"
    done
    echo "✓ Agents linked for Claude Code (~/.claude/agents/)"
fi

# 2. Binary — prebuilt or build from source
mkdir -p "$BIN_DIR"
ARCH="$(uname -s)-$(uname -m)"
PREBUILT="$SCRIPT_DIR/bin/s5d-$(echo "$ARCH" | tr '[:upper:]' '[:lower:]')"

# Build from the checked-out source first: it always matches this revision.
# The tracked prebuilt refreshes only at release time and has shipped stale
# binaries silently — it is a fallback for hosts without a Rust toolchain
# (same priority as `s5d admin update apply`).
if command -v cargo &> /dev/null; then
    echo "Building from source..."
    cd "$SCRIPT_DIR/rust" && cargo build --release
    rm -f "$BIN_DIR/s5d"
    cp "$SCRIPT_DIR/rust/target/release/s5d" "$BIN_DIR/s5d"
    echo "✓ Binary built and installed"
elif [ -f "$PREBUILT" ]; then
    echo "⚠ No Rust toolchain — installing tracked prebuilt ($ARCH); it may lag this checkout"
    rm -f "$BIN_DIR/s5d"
    cp "$PREBUILT" "$BIN_DIR/s5d"
    chmod +x "$BIN_DIR/s5d"
    echo "✓ Binary installed from prebuilt ($ARCH)"
else
    echo "⚠ No prebuilt binary for $ARCH and no Rust toolchain. Skills installed, CLI skipped."
    echo "  Install Rust (rustup.rs) and re-run. Skills are installed, CLI skipped."
fi

# macOS hardening: a cp'd adhoc-signed Mach-O is SIGKILL'd by AMFI on macOS 15+/26
# because copying invalidates the original inode's in-kernel code-signing trust.
# Re-sign the installed copy in place with a fresh adhoc signature so the kernel
# accepts it. Without this, `cp`-install produces a binary that dies with signal 9
# on every invocation (diagnosed 2026-05-30 on Darwin 25 / macOS 26).
if [ -f "$BIN_DIR/s5d" ] && [ "$(uname -s)" = "Darwin" ]; then
    if codesign --force --sign - "$BIN_DIR/s5d" 2>/dev/null; then
        echo "✓ Binary re-signed (adhoc) for macOS AMFI"
    else
        echo "⚠ codesign re-sign failed — if 's5d' is killed (signal 9), run:"
        echo "    codesign --force --sign - $BIN_DIR/s5d"
    fi
fi

echo ""
echo "What was installed:"
echo "  s5d binary — CLI for decisions, features, gates"
echo "  s5d skill — /s5d for agent runtimes (FPF references live inside /s5d)"
echo ""
echo "MCP server is registered per-project by 's5d init' (.mcp.json)."
echo "Marketplace installs (Claude/Gemini/Codex) register MCP globally via manifest."
echo ""
echo "Rust pre-commit hook:"
echo "  s5d init installs .git/hooks/pre-commit when run inside a git repo"
echo "  manual entrypoint: s5d hook pre-commit"
echo ""
echo "Self-update:"
echo "  s5d admin update check"
echo "  s5d admin update apply"
echo ""
echo "Usage: /s5d <problem>  or  s5d --help"
