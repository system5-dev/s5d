#!/usr/bin/env bash
# detect.sh — inventory the project's documentation surfaces. Read-only. JSON out.
#
# Goal of the skill: docs in ONE place, consistent, cross-linked, machine-readable.
# detect.sh measures where docs live today and which machine-readable anchors exist.

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"
json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }

# all markdown, pruned (exclude vendored / tooling / report dirs)
md_list() {
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
        -o -name build -o -name dist -o -name .next -o -name .venv -o -name __pycache__ \
        -o -name .claude -o -name .codex -o -name .agents -o -name test-reports \
        -o -name .playwright-browsers \) -prune \
        -o -type f -name '*.md' -print 2>/dev/null | sed 's|^\./||'
}

ALL_MD="$(md_list)"
md_total=$(printf '%s\n' "$ALL_MD" | sed '/^$/d' | wc -l | tr -d ' ')
# docs that live OUTSIDE a docs/ tree (scattered)
scattered=$(printf '%s\n' "$ALL_MD" | sed '/^$/d' | grep -vE '^docs/|^\.s5d/' | grep -vE '^(README|CHANGELOG|CONTRIBUTING|LICENSE|CODE_OF_CONDUCT|SECURITY)\.md$' | wc -l | tr -d ' ')
# distinct directories holding markdown
md_dirs=$(printf '%s\n' "$ALL_MD" | sed '/^$/d' | sed 's|/[^/]*$||;s|^[^/]*$|.|' | sort -u | wc -l | tr -d ' ')
# docs/ tree?
has_docs_dir=$([ -d docs ] && echo true || echo false)
# machine-readable anchors
has_llms=$([ -f llms.txt ] && echo true || echo false)
has_index=$({ [ -f docs/index.md ] || [ -f docs/README.md ] || [ -f docs/INDEX.md ]; } && echo true || echo false)
# how many md carry yaml front-matter (--- on line 1)
frontmatter=$(printf '%s\n' "$ALL_MD" | sed '/^$/d' | while read -r f; do head -1 "$f" 2>/dev/null | grep -q '^---$' && echo 1; done | wc -l | tr -d ' ')
# ADR / decision docs
adr=$(printf '%s\n' "$ALL_MD" | sed '/^$/d' | grep -ciE 'adr|decision|/rfc' || true)
# root entry docs present
root_readme=$([ -f README.md ] && echo true || echo false)

printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "markdown_total": %s,\n' "$md_total"
printf '  "markdown_dirs": %s,\n' "$md_dirs"
printf '  "scattered_outside_docs": %s,\n' "$scattered"
printf '  "anchors": {\n'
printf '    "docs_dir": %s,\n' "$has_docs_dir"
printf '    "root_readme": %s,\n' "$root_readme"
printf '    "docs_index": %s,\n' "$has_index"
printf '    "llms_txt": %s,\n' "$has_llms"
printf '    "adr_docs": %s\n' "$adr"
printf '  },\n'
printf '  "frontmatter_docs": %s,\n' "$frontmatter"
printf '  "machine_readable": %s\n' "$([ "$has_llms" = true ] && [ "$has_index" = true ] && echo true || echo false)"
printf '}\n'
