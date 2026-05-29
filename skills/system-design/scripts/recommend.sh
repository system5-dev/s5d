#!/usr/bin/env bash
# recommend.sh — score variants against interview answers and rank.
#
# Input: commodity id + answers.json (axis → answer value).
# Output: ranked variants with weighted score, applied hard constraints,
# disqualifications, warnings.
#
# Usage:
#   bash recommend.sh queue answers.json                    # text
#   bash recommend.sh queue answers.json --format md        # markdown
#   bash recommend.sh queue answers.json --top 3            # only top N

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DECISIONS_DIR="$SKILL_DIR/templates/decisions"
AXES="$SKILL_DIR/templates/interview-axes.yaml"

if [ $# -lt 2 ]; then
    echo "usage: $0 <commodity-id> <answers.json> [--format text|md] [--top N]" >&2
    exit 2
fi

COMMODITY="$1"; ANSWERS="$2"; shift 2
FORMAT="text"
TOP=0

while [ $# -gt 0 ]; do
    case "$1" in
        --format) FORMAT="$2"; shift 2 ;;
        --top)    TOP="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

YAML="$DECISIONS_DIR/${COMMODITY}.yaml"
[ ! -f "$YAML" ] && { echo "no decision frame for '$COMMODITY'" >&2; exit 2; }
[ ! -f "$ANSWERS" ] && { echo "no answers file at $ANSWERS" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

python3 - "$YAML" "$AXES" "$ANSWERS" "$FORMAT" "$TOP" "$COMMODITY" <<'PYEOF'
import sys, json, re

# Same slim YAML parser as matrix.sh / interview.sh.
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
                    item = {}; item_lvl = i + 2
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
                    inner = c[1:-1]; item = {}
                    for pair in inner.split(','):
                        if ':' in pair:
                            kk, _, vv = pair.partition(':')
                            item[kk.strip()] = scalar(vv)
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
with open(sys.argv[2]) as f: axdef = parse_yaml(f.read()) or {}
with open(sys.argv[3]) as f: ans_raw = json.load(f)
ans = ans_raw.get('answers', ans_raw)
fmt = sys.argv[4]
top = int(sys.argv[5]) if sys.argv[5] != '0' else None
commodity = sys.argv[6]

weights = dec.get('axes', {}) or {}
axes_def = axdef.get('axes', {}) or {}
variants = dec.get('variants', []) or []
constraints = dec.get('hard_constraints', []) or []

# For each axis, normalize the answer:
# - enum axis: answer == enum id → use that variant's score for that axis
# - numeric axis: answer in band → band's score / not used; we use the
#   variant's score for the axis directly. The answer's band determines
#   whether a constraint rule fires.
# Simplification for v1: variants carry pre-baked scores per axis;
# answers don't change the variant score, they only weight axes and
# fire constraints. This matches the design — weights ARE the decision
# input, not per-axis variant adjustments.

def evaluate_constraint(rule, ans, variant):
    """Tiny rule mini-language. Each rule is a string like:
        "ops_capacity:zero AND variant.kind == 'self-hosted'"
        "compliance contains 'pci' AND variant.id == 'inngest'"
       Operators: AND, OR (left-to-right), comparators per term.
       Terms: 'axis:value', 'compliance contains X', 'variant.field OP value',
              'axis:LEVEL' where LEVEL is the enum id or numeric band label."""
    def eval_term(t):
        t = t.strip()
        # variant.X OP 'val'
        m = re.match(r"variant\.(\w+)\s*(==|!=|IN)\s*(.+)", t)
        if m:
            field, op, val = m.group(1), m.group(2), m.group(3).strip()
            v = variant.get(field)
            if op == '==':
                return v == val.strip("'\"")
            if op == '!=':
                return v != val.strip("'\"")
            if op == 'IN':
                # val like ['a', 'b']
                vals = [x.strip(" '\"") for x in val.strip('[]').split(',')]
                return v in vals
        # compliance contains 'X'
        m = re.match(r"compliance\s+contains\s+'(\w+)'", t)
        if m:
            req = m.group(1)
            ans_val = ans.get('compliance', [])
            if isinstance(ans_val, str): ans_val = [ans_val]
            return req in (ans_val or [])
        # axis:VALUE
        m = re.match(r"(\w+):(\w+)", t)
        if m:
            axis, target = m.group(1), m.group(2)
            ans_val = ans.get(axis)
            if ans_val is None: return False
            # If target is numeric (1-5), compare to score band
            if target.isdigit():
                # numeric axis: find the band for the answer
                axis_info = axes_def.get(axis, {}) or {}
                bands = axis_info.get('bands') or []
                for b in bands:
                    if isinstance(b, dict):
                        mx = b.get('max')
                        if mx is None or (isinstance(ans_val, (int, float)) and ans_val <= mx):
                            return str(b.get('score', '')) == target
                return False
            else:
                return str(ans_val) == target
        return False

    # Split on AND/OR — naive but enough for our rules.
    if ' AND ' in rule:
        return all(eval_term(t) for t in rule.split(' AND '))
    if ' OR ' in rule:
        return any(eval_term(t) for t in rule.split(' OR '))
    return eval_term(rule)


def variant_score(v):
    scores = v.get('scores', {}) or {}
    total = 0
    for axis, w in weights.items():
        s = scores.get(axis, 0)
        if not isinstance(s, (int, float)):
            try: s = int(s)
            except: s = 0
        if not isinstance(w, (int, float)):
            try: w = int(w)
            except: w = 0
        total += w * s
    return total

ranked = []
for v in variants:
    flags = []
    for c in constraints:
        rule = c.get('rule', '')
        if evaluate_constraint(rule, ans, v):
            flags.append((c.get('action', 'warn'), c.get('reason', '')))
    score = variant_score(v)
    disqualified = any(a == 'disqualify' for a, _ in flags)
    ranked.append({
        'variant': v,
        'score': score,
        'flags': flags,
        'disqualified': disqualified,
    })

# Sort: qualified by score desc; disqualified last
ranked.sort(key=lambda r: (r['disqualified'], -r['score']))

if top is not None:
    ranked = ranked[:top]

if fmt == 'text':
    print(f"═══ Recommendation: {commodity} ═══\n")
    for i, r in enumerate(ranked, 1):
        v = r['variant']
        status = ' (DISQUALIFIED)' if r['disqualified'] else ''
        print(f"  {i}. {v.get('name', v['id'])}  score={r['score']}{status}")
        for action, reason in r['flags']:
            sym = '⛔' if action == 'disqualify' else '⚠'
            print(f"     {sym} {reason}")
elif fmt == 'md':
    print(f"# Recommendation: {dec.get('title', commodity)}\n")
    print("| Rank | Variant | Kind | Score | Flags |")
    print("|---:|---|---|---:|---|")
    for i, r in enumerate(ranked, 1):
        v = r['variant']
        flag_str = '; '.join(f"{'⛔' if a == 'disqualify' else '⚠'} {reason}" for a, reason in r['flags']) or ''
        status = ' **(DISQUALIFIED)**' if r['disqualified'] else ''
        print(f"| {i} | **{v.get('name', v['id'])}**{status} | {v.get('kind', '—')} | {r['score']} | {flag_str} |")
PYEOF
