#!/usr/bin/env bash
# analyze.sh — documentation health findings. Read-only. JSON.
# Findings target: one home, consistent, cross-linked, machine-readable.
# Every finding pairs a Fix with a Validate. Heuristic [INFERRED] leads.

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"
json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }

md_list() {
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
        -o -name build -o -name dist -o -name .next -o -name .venv -o -name __pycache__ \
        -o -name .claude -o -name .codex -o -name .agents -o -name test-reports \
        -o -name .playwright-browsers \) -prune \
        -o -type f -name '*.md' -print 2>/dev/null | sed 's|^\./||'
}
ALL_MD="$(md_list | sed '/^$/d')"

FINDINGS=""
add() { FINDINGS="${FINDINGS}${1}	${2}	${3}	${4}	${5}	${6}"$'\n'; }  # check sev path detail fix validate

total=$(printf '%s\n' "$ALL_MD" | wc -l | tr -d ' ')
dirs=$(printf '%s\n' "$ALL_MD" | sed 's|/[^/]*$||;s|^[^/]*$|.|' | sort -u | wc -l | tr -d ' ')
scattered=$(printf '%s\n' "$ALL_MD" | grep -vE '^docs/|^\.s5d/' | grep -vE '^(README|CHANGELOG|CONTRIBUTING|LICENSE|SECURITY)\.md$' | wc -l | tr -d ' ')

# 1 — no single home
if [ "$scattered" -gt 5 ] || { [ ! -f docs/index.md ] && [ ! -f docs/README.md ]; }; then
    add no-single-home high "(repo)" \
        "$total markdown docs across $dirs directories ($scattered outside docs/); no docs/ index ties them together — readers and agents have no canonical entry point" \
        "Adopt a single docs/ tree with docs/index.md as the table of contents; move scattered topic docs in (leave README.md at root pointing to docs/index.md)" \
        "Every doc is reachable by following links from docs/index.md (a link-graph walk visits all $total files, 0 unreachable)."
fi

# 2 — no machine-readable entry (llms.txt)
if [ ! -f llms.txt ]; then
    add no-llms-txt high "llms.txt (missing)" \
        "No llms.txt — there is no machine-readable index of the project's docs for LLMs/agents to consume (the llms.txt standard: title + summary + curated links)" \
        "Generate llms.txt at repo root: H1 project name, a one-paragraph blockquote summary, then sections linking each canonical doc with a short description (use this skill's generate.sh)" \
        "llms.txt exists, parses as the llms.txt format, and lists every canonical doc; a link-checker confirms 0 broken links in it."
fi

# 3 — no front-matter (machine-readable metadata)
fm=$(printf '%s\n' "$ALL_MD" | while read -r f; do head -1 "$f" 2>/dev/null | grep -q '^---$' && echo 1; done | wc -l | tr -d ' ')
if [ "$fm" -lt "$total" ]; then
    miss=$((total - fm))
    add no-frontmatter medium "($miss of $total docs)" \
        "$miss docs have no YAML front-matter — no machine-readable title/status/owner/updated metadata, so docs can't be sorted, filtered, or staleness-checked programmatically" \
        "Add a front-matter block (title, status, owner, last_reviewed, tags) to every canonical doc; enforce with a CI check" \
        "A script asserting every docs/**/*.md starts with a valid front-matter block exits 0; the index can render owner/updated columns from it."
fi

# 4 — broken cross-references (relative md links to missing files)
broken=""
bn=0
while IFS= read -r f; do
    [ -z "$f" ] && continue
    base=$(dirname "$f")
    # extract relative .md link targets: [..](path.md) not http
    grep -oE '\]\(([^)#]+\.md)(#[^)]*)?\)' "$f" 2>/dev/null | sed -E 's/\]\(//; s/(#[^)]*)?\)$//' | while read -r tgt; do
        case "$tgt" in http*|/*) continue ;; esac
        target="$base/$tgt"
        [ -f "$target" ] || printf '%s -> %s\n' "$f" "$tgt"
    done
done <<< "$ALL_MD" > /tmp/_xref_$$ 2>/dev/null || true
bn=$(sed '/^$/d' /tmp/_xref_$$ 2>/dev/null | wc -l | tr -d ' ')
if [ "${bn:-0}" -gt 0 ]; then
    sample=$(head -3 /tmp/_xref_$$ | paste -sd'; ' -)
    add broken-xref high "($bn broken links)" \
        "$bn markdown cross-reference(s) point to files that don't exist (e.g. $sample) — the doc graph is broken; following a link 404s" \
        "Fix or remove each dangling link; after consolidation, re-point links to the new docs/ locations" \
        "A markdown link-checker (lychee / markdown-link-check) over all docs returns 0 broken internal links, wired into CI."
fi
rm -f /tmp/_xref_$$ 2>/dev/null || true

# 5 — orphan docs (no inbound links from any other md)
linked_tmp=/tmp/_linked_$$
grep -rhoE '\]\(([^)#]+\.md)' $ALL_MD 2>/dev/null | sed -E 's/.*\(//; s|.*/||' | sort -u > "$linked_tmp" 2>/dev/null || true
orphans=0
while IFS= read -r f; do
    [ -z "$f" ] && continue
    bn2=$(basename "$f")
    case "$bn2" in README.md|index.md|INDEX.md|llms.txt) continue ;; esac
    grep -qxF "$bn2" "$linked_tmp" 2>/dev/null || orphans=$((orphans+1))
done <<< "$ALL_MD"
rm -f "$linked_tmp" 2>/dev/null || true
if [ "$orphans" -gt 3 ]; then
    add orphan-docs medium "($orphans orphans)" \
        "$orphans docs are not linked from any other doc — they're unreachable by navigation; readers only find them by full-text search or luck" \
        "Link every doc from the index (or a parent topic page); delete genuinely dead docs" \
        "0 orphan docs: every file (except the index) has ≥1 inbound link in the doc graph."
fi

# 6 — naming inconsistency
upper=$(printf '%s\n' "$ALL_MD" | sed 's|.*/||' | grep -cE '^[A-Z0-9_]+\.md$' || true)
kebab=$(printf '%s\n' "$ALL_MD" | sed 's|.*/||' | grep -cE '^[a-z0-9-]+\.md$' || true)
if [ "$upper" -gt 0 ] && [ "$kebab" -gt 0 ]; then
    add naming-inconsistency low "($upper UPPER_SNAKE, $kebab kebab-case)" \
        "Doc filenames mix conventions ($upper UPPER_SNAKE like TMHCC_SETUP.md, $kebab kebab-case) — inconsistent, hard to predict, ugly in an index" \
        "Pick one convention (kebab-case recommended) and rename; add redirects/aliases if any are externally linked" \
        "All docs/**/*.md match a single filename regex (a lint check exits 0)."
fi

# --- JSON ---
ch=0; cm=0; cl=0
printf '{\n  "root": "%s",\n  "findings": [' "$(json_str "$ROOT")"
first=1
while IFS=$'\t' read -r check sev path detail fix validate; do
    [ -z "$check" ] && continue
    case "$sev" in high) ch=$((ch+1));; medium) cm=$((cm+1));; low) cl=$((cl+1));; esac
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"check":"%s","severity":"%s","path":"%s","detail":"%s","fix":"%s","validate":"%s"}' \
        "$(json_str "$check")" "$(json_str "$sev")" "$(json_str "$path")" \
        "$(json_str "$detail")" "$(json_str "$fix")" "$(json_str "$validate")"
done <<< "$FINDINGS"
[ "$first" -eq 0 ] && printf '\n  '
printf '],\n  "summary": {"high": %d, "medium": %d, "low": %d, "total": %d}\n}\n' \
    "$ch" "$cm" "$cl" "$((ch+cm+cl))"
