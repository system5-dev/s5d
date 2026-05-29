#!/usr/bin/env bash
# interview.sh — emit the question set for a commodity, ready for the
# agent to ask interactively.
#
# Output:
#   text — human-readable numbered questions
#   json — machine-readable answer template (axis → null), suitable
#          for piping into recommend.sh after filling in.
#
# Usage:
#   bash interview.sh queue                          # text
#   bash interview.sh queue --format json            # answers template
#   bash interview.sh queue > /tmp/queue-q.md        # save to file

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DECISIONS_DIR="$SKILL_DIR/templates/decisions"
AXES="$SKILL_DIR/templates/interview-axes.yaml"

if [ $# -lt 1 ]; then
    echo "usage: $0 <commodity-id> [--format text|json]" >&2
    exit 2
fi

COMMODITY="$1"; shift || true
FORMAT="text"

while [ $# -gt 0 ]; do
    case "$1" in
        --format) FORMAT="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

YAML="$DECISIONS_DIR/${COMMODITY}.yaml"
[ ! -f "$YAML" ] && { echo "no decision frame for '$COMMODITY'" >&2; exit 2; }

command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

python3 - "$YAML" "$AXES" "$FORMAT" "$COMMODITY" <<'PYEOF'
import sys, json

# Reuse the same minimal YAML parser from matrix.sh by inlining a slim version.
def parse_yaml(text):
    lines = text.split('\n'); pos = [0]
    def cur(): return lines[pos[0]] if pos[0] < len(lines) else None
    def ind(line):
        if line is None: return -1
        if line.strip() == '' or line.lstrip().startswith('#'): return -2
        return len(line) - len(line.lstrip())
    def scalar(s):
        s = s.strip()
        if not (s.startswith('"') or s.startswith("'")):
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
    def block(level):
        out = None; first = True
        while True:
            line = cur()
            if line is None: break
            i = ind(line)
            if i == -2: pos[0] += 1; continue
            if i < level: break
            if i > level and not first: pos[0] += 1; continue
            s = line.lstrip()
            if s.startswith('- '):
                if out is None: out = []
                if not isinstance(out, list): break
                c = s[2:]; pos[0] += 1
                if ':' in c and not c.startswith('{'):
                    k, _, v = c.partition(':')
                    item = {}
                    item_lvl = i + 2
                    item[k.strip()] = scalar(v) if v.strip() else block(item_lvl)
                    while True:
                        nl = cur()
                        if nl is None: break
                        ni = ind(nl)
                        if ni == -2: pos[0] += 1; continue
                        if ni < item_lvl: break
                        if ni > item_lvl: pos[0] += 1; continue
                        ns = nl.lstrip()
                        if ns.startswith('- '): break
                        if ':' in ns:
                            kk, _, vv = ns.partition(':')
                            pos[0] += 1
                            if vv.strip().startswith('|'):
                                bl = []
                                while True:
                                    n2 = cur()
                                    if n2 is None: break
                                    ni2 = ind(n2)
                                    if ni2 == -2: pos[0] += 1; continue
                                    if ni2 <= item_lvl: break
                                    bl.append(n2[item_lvl+2:])
                                    pos[0] += 1
                                item[kk.strip()] = '\n'.join(bl).rstrip()
                            else:
                                item[kk.strip()] = scalar(vv) if vv.strip() else block(item_lvl+2)
                        else: break
                    out.append(item)
                elif c.startswith('{') and c.endswith('}'):
                    # inline flow mapping
                    inner = c[1:-1]
                    item = {}
                    for pair in inner.split(','):
                        if ':' in pair:
                            k, _, v = pair.partition(':')
                            item[k.strip()] = scalar(v)
                    out.append(item)
                else:
                    out.append(scalar(c))
                first = False
            elif ':' in s:
                if out is None: out = {}
                if not isinstance(out, dict): break
                k, _, v = s.partition(':')
                pos[0] += 1
                if v.strip() == '':
                    out[k.strip()] = block(i + 2)
                elif v.strip().startswith('|'):
                    bl = []; base = None
                    while True:
                        nl = cur()
                        if nl is None: break
                        ni = ind(nl)
                        if ni == -2: pos[0] += 1; continue
                        if ni <= i: break
                        if base is None: base = ni
                        bl.append(nl[base:])
                        pos[0] += 1
                    out[k.strip()] = '\n'.join(bl).rstrip()
                else:
                    out[k.strip()] = scalar(v)
                first = False
            else: pos[0] += 1
        return out
    return block(0)

with open(sys.argv[1]) as f: dec = parse_yaml(f.read())
with open(sys.argv[2]) as f: axdef = parse_yaml(f.read())

fmt = sys.argv[3]; commodity = sys.argv[4]

# Order axes by weight descending so the most-important questions come first.
weights = dec.get('axes', {}) or {}
axes = axdef.get('axes', {}) or {}
ordered = sorted(weights.keys(), key=lambda a: -weights.get(a, 0))

if fmt == 'json':
    template = {a: None for a in ordered}
    out = {"commodity": commodity, "answers": template, "_note": "fill in each axis with an id from interview-axes.yaml or a numeric value"}
    print(json.dumps(out, indent=2))
elif fmt == 'text':
    title = dec.get('title', commodity)
    print(f"# Interview: {title}\n")
    print(f"Weights for this commodity, descending — answer the high-weight ones carefully:\n")
    for i, a in enumerate(ordered, 1):
        info = axes.get(a) or {}
        q = info.get('question', a) if isinstance(info, dict) else a
        w = weights[a]
        print(f"{i}. **{a}** (weight {w}): {q}")
        if isinstance(info, dict) and info.get('values'):
            for v in info['values']:
                if isinstance(v, dict):
                    vid = v.get('id', '?')
                    vlabel = v.get('label', '')
                    print(f"   - `{vid}` — {vlabel}")
        elif isinstance(info, dict) and info.get('bands'):
            for b in info['bands']:
                if isinstance(b, dict):
                    label = b.get('label', '?')
                    mx = b.get('max', '∞')
                    print(f"   - ≤ {mx} → {label}")
        print()
    print("---")
    print("When done, save answers as JSON (use `--format json` for a template) and run:")
    print(f"  bash <skill>/scripts/recommend.sh {commodity} <answers.json>")
else:
    sys.stderr.write(f"unknown format: {fmt}\n"); sys.exit(2)
PYEOF
