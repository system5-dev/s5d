#!/usr/bin/env bash
# adr.sh — write an Architecture Decision Record skeleton for the chosen
# variant. Lands as a fresh s5d decision spec in .s5d/packages/ so it
# can be reviewed via the s5d skill (s5d_validate, s5d_preview, ...).
#
# Usage:
#   bash adr.sh queue --chose nats_jetstream --decided-by Roman \
#                     --rationale "low ops, sub-ms latency, cross-region edges"
#
# Required:
#   <commodity>     e.g. queue, cache, search
#   --chose <id>    variant id from templates/decisions/<commodity>.yaml
#   --decided-by    name of the human who confirmed
#   --rationale     one-paragraph justification

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DECISIONS_DIR="$SKILL_DIR/templates/decisions"

if [ $# -lt 1 ]; then
    echo "usage: $0 <commodity> --chose <variant-id> --decided-by <name> --rationale <text>" >&2
    exit 2
fi

COMMODITY="$1"; shift
CHOSEN=""
DECIDED_BY=""
RATIONALE=""
OUTPUT=""
ANSWERS_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --chose)      CHOSEN="$2"; shift 2 ;;
        --decided-by) DECIDED_BY="$2"; shift 2 ;;
        --rationale)  RATIONALE="$2"; shift 2 ;;
        --output)     OUTPUT="$2"; shift 2 ;;
        --answers)    ANSWERS_FILE="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

[ -z "$CHOSEN" ]     && { echo "--chose is required" >&2; exit 2; }
[ -z "$DECIDED_BY" ] && { echo "--decided-by is required" >&2; exit 2; }
[ -z "$RATIONALE" ]  && { echo "--rationale is required" >&2; exit 2; }

YAML="$DECISIONS_DIR/${COMMODITY}.yaml"
[ ! -f "$YAML" ] && { echo "no decision frame: $COMMODITY" >&2; exit 2; }

# Pull the chosen variant's metadata (name, kind, notes) via python.
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

DEC_ID="decision.${COMMODITY}-choice"
PRODUCT=$(basename "$(pwd)")

# ── s5d CLI path ───────────────────────────────────────────────────────────────

if command -v s5d >/dev/null 2>&1; then
    # Create the spec scaffold via the real CLI.
    QUESTION="Which ${COMMODITY} variant should we use, given the constraints captured during the system-design interview?"
    S5D_NEW_OUT=$(s5d new "$DEC_ID" --tier decision --product "$PRODUCT" --question "$QUESTION" 2>&1)
    SPEC_PATH=$(printf '%s\n' "$S5D_NEW_OUT" | grep "^ok Created spec:" | sed 's/^ok Created spec: //')

    if [ -z "$SPEC_PATH" ]; then
        echo "error: s5d new did not emit a spec path. Output was:" >&2
        echo "$S5D_NEW_OUT" >&2
        exit 1
    fi

    # Extract all variant ids from the commodity yaml, winner first, then rejected.
    # For each variant: pull name and notes for --title and --content.
    python3 - "$YAML" "$CHOSEN" "$SPEC_PATH" <<'PYEOF'
import sys, subprocess, re

yaml_file, chosen, spec_path = sys.argv[1], sys.argv[2], sys.argv[3]

with open(yaml_file) as f:
    text = f.read()

# Parse variant blocks: collect (id, name, kind, notes) for each variant.
variants = []
lines = text.split('\n')
i = 0
while i < len(lines):
    stripped = lines[i].lstrip()
    if stripped.startswith('- id:'):
        vid = stripped.split(':', 1)[1].strip()
        vname = ''
        vkind = ''
        vnotes_lines = []
        j = i + 1
        indent_base = len(lines[i]) - len(stripped)
        in_notes = False
        notes_indent = None
        while j < len(lines):
            s = lines[j].lstrip()
            cur_indent = len(lines[j]) - len(s) if lines[j].strip() else 999
            # Next top-level variant item?
            if lines[j].strip() and cur_indent <= indent_base and s.startswith('- '):
                break
            # Top-level section outside variants list?
            if lines[j].strip() and cur_indent <= indent_base and not lines[j][indent_base:indent_base+1] == ' ':
                break
            if s.startswith('name:'):
                vname = s.split(':', 1)[1].strip()
            elif s.startswith('kind:'):
                vkind = s.split(':', 1)[1].strip()
            elif s.startswith('notes:'):
                in_notes = True
                notes_indent = None
                inline = s[len('notes:'):].strip().lstrip('|').strip()
                if inline:
                    vnotes_lines.append(inline)
            elif in_notes:
                if notes_indent is None and lines[j].strip():
                    notes_indent = cur_indent
                if lines[j].strip() and cur_indent < (notes_indent or 0):
                    in_notes = False
                else:
                    stripped_line = lines[j].strip().lstrip('- ').strip()
                    if stripped_line:
                        vnotes_lines.append(stripped_line)
            j += 1
        notes_text = '; '.join(l for l in vnotes_lines if l).strip()
        if not notes_text:
            notes_text = f"{vname} ({vkind})"
        variants.append({'id': vid, 'name': vname, 'kind': vkind, 'notes': notes_text})
        i = j
    else:
        i += 1

# Emit hypotheses: chosen first, then the rest.
def add_hypothesis(vid, vname, vnotes, spec):
    title = vname if vname else vid
    content = vnotes if vnotes else vid
    scope = f"{yaml_file.split('/')[-1].replace('.yaml','')} commodity selection"
    cmd = ['s5d', 'decision', 'add-hypothesis', spec,
           '--title', title, '--content', content, '--scope', scope]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  warning: add-hypothesis for {vid} failed: {result.stderr.strip()}", file=sys.stderr)
    else:
        print(f"  + hypothesis: {result.stdout.strip()}", file=sys.stderr)

chosen_variant = next((v for v in variants if v['id'] == chosen), None)
if chosen_variant:
    add_hypothesis(chosen_variant['id'], chosen_variant['name'], chosen_variant['notes'], spec_path)
else:
    print(f"  warning: chosen variant '{chosen}' not found in yaml", file=sys.stderr)

for v in variants:
    if v['id'] != chosen:
        add_hypothesis(v['id'], v['name'], v['notes'], spec_path)
PYEOF

    echo "✓ wrote $SPEC_PATH" >&2
    if [ -n "$ANSWERS_FILE" ] && [ -f "$ANSWERS_FILE" ]; then
        echo "  interview answers: $ANSWERS_FILE" >&2
    fi
    echo "  decided-by: $DECIDED_BY" >&2
    echo "  rationale: $RATIONALE" >&2
    echo "" >&2
    echo "  next: review with the s5d skill — s5d_preview / s5d_decide via MCP" >&2
    echo "  (s5d_decide requires human confirmation — not called by this script)" >&2
    echo "$SPEC_PATH"

else
    # s5d CLI not on PATH. The previous fallback wrote a raw, UNVALIDATED YAML spec
    # here — it interpolated user input into a YAML heredoc with no escaping (YAML
    # injection) and honored an arbitrary --output (arbitrary file overwrite). Both
    # were HIGH findings (tribunal 2026-05-30, .s5d/tribunal/cluster-runtime-20260530.md).
    # The suite ships WITH the s5d CLI, so the safe behavior is to require it rather
    # than emit an injectable draft to a caller-chosen path.
    echo "error: s5d CLI not found on PATH — cannot emit a schema-valid decision spec." >&2
    echo "  adr.sh routes through the s5d CLI by design. Install it first:" >&2
    echo "    git clone https://github.com/system5-dev/s5d && cd s5d && ./install.sh" >&2
    exit 127
fi
