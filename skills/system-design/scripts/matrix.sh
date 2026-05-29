#!/usr/bin/env bash
# matrix.sh — render a decision frame as a comparison matrix in markdown.
# Pure deterministic: takes a commodity id, renders templates/decisions/<id>.yaml.
#
# Usage:
#   bash matrix.sh queue                          # all variants
#   bash matrix.sh queue --variants sqs,rabbitmq  # subset
#   bash matrix.sh queue --format md              # default
#   bash matrix.sh queue --format text            # console-friendly
#   bash matrix.sh list                           # list all available commodities

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DECISIONS_DIR="$SKILL_DIR/templates/decisions"

if [ $# -lt 1 ]; then
    echo "usage: $0 <commodity-id> | list" >&2
    exit 2
fi

OP="$1"; shift || true

if [ "$OP" = "list" ]; then
    echo "Available decision frames:"
    for f in "$DECISIONS_DIR"/*.yaml; do
        id=$(basename "$f" .yaml)
        title=$(grep -m1 '^title:' "$f" | sed 's/^title:[[:space:]]*//')
        printf '  %-25s  %s\n' "$id" "$title"
    done
    exit 0
fi

COMMODITY="$OP"
FORMAT="md"
VARIANT_FILTER=""

while [ $# -gt 0 ]; do
    case "$1" in
        --format)   FORMAT="$2"; shift 2 ;;
        --variants) VARIANT_FILTER="$2"; shift 2 ;;
        *)          echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

YAML="$DECISIONS_DIR/${COMMODITY}.yaml"
[ ! -f "$YAML" ] && { echo "no decision frame for '$COMMODITY' (see: $0 list)" >&2; exit 2; }

command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

python3 - "$YAML" "$FORMAT" "$VARIANT_FILTER" <<'PYEOF'
import sys, os

# Minimal YAML parser — no PyYAML dependency. Handles the limited shapes
# we use in our decision YAML files (mappings, lists, scalar key:value,
# pipe-folded multiline strings).

def parse_yaml(text):
    lines = text.split('\n')
    pos = [0]

    def cur(): return lines[pos[0]] if pos[0] < len(lines) else None
    def peek_indent(line):
        if line is None: return -1
        if line.strip() == '' or line.lstrip().startswith('#'): return -2
        return len(line) - len(line.lstrip())

    def parse_scalar(s):
        s = s.strip()
        # Strip inline comments — '# comment' after a value, but only when
        # not inside a quoted string.
        if not (s.startswith('"') or s.startswith("'")):
            # naive: drop everything from the first '#' preceded by space
            import re
            m = re.search(r'\s+#', s)
            if m: s = s[:m.start()].strip()
        if s == '' or s == '~' or s == 'null': return None
        if s == 'true': return True
        if s == 'false': return False
        if (s.startswith('"') and s.endswith('"')) or (s.startswith("'") and s.endswith("'")):
            return s[1:-1]
        try: return int(s)
        except: pass
        try: return float(s)
        except: pass
        return s

    def parse_block(indent):
        out = None
        first = True
        while True:
            line = cur()
            if line is None: break
            ind = peek_indent(line)
            if ind == -2:
                pos[0] += 1
                continue
            if ind < indent: break
            if ind > indent and not first:
                pos[0] += 1
                continue
            stripped = line.lstrip()
            if stripped.startswith('- '):
                if out is None: out = []
                if not isinstance(out, list): break
                # list item
                item_indent = ind + 2
                # parse the item content
                content = stripped[2:]
                pos[0] += 1
                if ':' in content and not content.startswith('{'):
                    # inline mapping start
                    key, _, val = content.partition(':')
                    item = {}
                    item[key.strip()] = parse_scalar(val) if val.strip() else parse_block(item_indent)
                    # Continue parsing subsequent same-indent keys.
                    while True:
                        nl = cur()
                        if nl is None: break
                        nind = peek_indent(nl)
                        if nind == -2: pos[0] += 1; continue
                        if nind < item_indent: break
                        if nind > item_indent: pos[0] += 1; continue
                        nstripped = nl.lstrip()
                        if nstripped.startswith('- '): break
                        if ':' in nstripped:
                            k, _, v = nstripped.partition(':')
                            pos[0] += 1
                            if v.strip().startswith('|'):
                                # pipe-folded
                                block = []
                                while True:
                                    nl2 = cur()
                                    if nl2 is None: break
                                    nind2 = peek_indent(nl2)
                                    if nind2 == -2: pos[0] += 1; continue
                                    if nind2 <= item_indent: break
                                    block.append(nl2[item_indent+2:])
                                    pos[0] += 1
                                item[k.strip()] = '\n'.join(block).rstrip()
                            else:
                                item[k.strip()] = parse_scalar(v) if v.strip() else parse_block(item_indent + 2)
                        else:
                            break
                    out.append(item)
                else:
                    out.append(parse_scalar(content))
                first = False
            elif ':' in stripped:
                if out is None: out = {}
                if not isinstance(out, dict): break
                key, _, val = stripped.partition(':')
                pos[0] += 1
                if val.strip() == '':
                    out[key.strip()] = parse_block(ind + 2)
                elif val.strip().startswith('|'):
                    block = []
                    base = None
                    while True:
                        nl = cur()
                        if nl is None: break
                        nind = peek_indent(nl)
                        if nind == -2: pos[0] += 1; continue
                        if nind <= ind: break
                        if base is None: base = nind
                        block.append(nl[base:])
                        pos[0] += 1
                    out[key.strip()] = '\n'.join(block).rstrip()
                else:
                    out[key.strip()] = parse_scalar(val)
                first = False
            else:
                pos[0] += 1
        return out

    return parse_block(0)


with open(sys.argv[1]) as f:
    data = parse_yaml(f.read())

fmt = sys.argv[2]
variant_filter = sys.argv[3].split(',') if sys.argv[3] else []

variants = data.get('variants', []) or []
if variant_filter:
    variants = [v for v in variants if v.get('id') in variant_filter]

axes = data.get('axes', {}) or {}
axes_ids = list(axes.keys())

def emit_md():
    print(f"# {data.get('title', data.get('id'))}")
    print()
    if data.get('description'):
        print(data['description'].strip())
        print()
    print("## Variants")
    print()
    headers = ['Variant', 'Kind', 'Self-host'] + axes_ids + ['Weighted']
    weights = [axes.get(a, 0) for a in axes_ids]
    print('| ' + ' | '.join(headers) + ' |')
    print('|' + '|'.join(['---'] * len(headers)) + '|')
    for v in variants:
        scores = v.get('scores', {}) or {}
        row = [
            f"**{v.get('name', v['id'])}**",
            v.get('kind', '—'),
            '✓' if v.get('self_hostable') else '✗',
        ]
        weighted = 0
        for a, w in zip(axes_ids, weights):
            s = scores.get(a, '—')
            if s != '—':
                weighted += w * s
            row.append(str(s))
        row.append(f"**{weighted}**")
        print('| ' + ' | '.join(row) + ' |')
    print()
    print("## Notes per variant")
    print()
    for v in variants:
        print(f"### {v.get('name', v['id'])}")
        notes = v.get('notes', '').strip()
        if notes:
            print()
            print(notes)
        print()
    if 'hard_constraints' in data and data['hard_constraints']:
        print("## Hard constraints")
        print()
        for c in data['hard_constraints']:
            action = c.get('action', '—')
            print(f"- **{action}**: `{c.get('rule', '')}` — {c.get('reason', '')}")
        print()


def emit_text():
    title = data.get('title', data.get('id'))
    print(f"═══ {title} ═══\n")
    if data.get('description'):
        print(data['description'].strip())
        print()
    weights = [axes.get(a, 0) for a in axes_ids]
    print("Variants:\n")
    for v in variants:
        scores = v.get('scores', {}) or {}
        weighted = sum(axes.get(a, 0) * scores.get(a, 0) for a in axes_ids)
        host = 'self' if v.get('self_hostable') else 'managed'
        print(f"  {v.get('name', v['id'])}  [{host}]  weighted={weighted}")
    print()


if fmt == 'md':
    emit_md()
elif fmt == 'text':
    emit_text()
else:
    sys.stderr.write(f"unknown format: {fmt}\n"); sys.exit(2)
PYEOF
