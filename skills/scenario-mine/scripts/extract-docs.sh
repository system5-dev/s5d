#!/usr/bin/env bash
# extract-docs.sh — pull H2/H3/H4 headings and checklist items from markdown.
# Output: JSON array of {file, line, kind, level, text}.
#
# kind ∈ heading | checklist
# level ∈ 2 | 3 | 4 (for headings) | null (for checklists)
#
# Usage:
#   bash extract-docs.sh README.md docs/api.md
#   echo -e "a.md\nb.md" | bash extract-docs.sh

set -uo pipefail   # NO -e: grep returns 1 on zero matches.

if [ $# -gt 0 ]; then
    FILES="$*"
else
    FILES="$(cat)"
fi

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

# Emit one object per line. Post-processor wraps in [...].
emit_heading() {
    local file="$1" line="$2" level="$3" text="$4"
    printf '{"file":"%s","line":%d,"kind":"heading","level":%d,"text":"%s"}\n' \
        "$(json_str "$file")" "$line" "$level" "$(json_str "$text")"
}
emit_checklist() {
    local file="$1" line="$2" text="$3"
    printf '{"file":"%s","line":%d,"kind":"checklist","level":null,"text":"%s"}\n' \
        "$(json_str "$file")" "$line" "$(json_str "$text")"
}

scan_markdown() {
    local f="$1"

    grep -nE '^#{2,4}[[:space:]]+' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local hashes level text
            hashes="${rest%%[![:space:]#]*}"
            case "$hashes" in
                '####'*) level=4 ;;
                '###'*)  level=3 ;;
                '##'*)   level=2 ;;
                *) continue ;;
            esac
            text="${rest#"$hashes"}"
            text="${text# }"
            text="${text%% #*}"
            emit_heading "$f" "$ln" "$level" "$text"
        done

    grep -nE '^[[:space:]]*[-*+][[:space:]]+\[[ xX]\][[:space:]]+' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local text
            text=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]*[-*+][[:space:]]+\[[ xX]\][[:space:]]+//')
            emit_checklist "$f" "$ln" "$text"
        done
}

{
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        [ ! -f "$f" ] && continue
        case "$f" in
            *.md|*.markdown) scan_markdown "$f" ;;
            *) ;;
        esac
    done <<< "$FILES"
} | awk '
    BEGIN { print "[" }
    NR == 1 { printf "  %s", $0; next }
    { printf ",\n  %s", $0 }
    END { if (NR > 0) printf "\n"; print "]" }
'
