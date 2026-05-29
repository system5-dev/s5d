#!/usr/bin/env bash
# report.sh — render detect.sh + analyze.sh into a markdown scalability report.
# Read-only. Stdout, or --save → test-reports/scaling/report.md.
#
#   bash report.sh            # markdown to stdout
#   bash report.sh --save     # also write test-reports/scaling/report.md

set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(pwd)"
OUTPUT=""; SAVE=0
while [ $# -gt 0 ]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --save)   SAVE=1; shift ;;
        --root)   ROOT="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done
[ "$SAVE" -eq 1 ] && [ -z "$OUTPUT" ] && OUTPUT="test-reports/scaling/report.md"
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

DETECT="$(bash "$HERE/detect.sh" "$ROOT" 2>/dev/null)"
ANALYZE="$(bash "$HERE/analyze.sh" "$ROOT" 2>/dev/null)"

render() {
python3 - "$DETECT" "$ANALYZE" <<'PYEOF'
import json, sys
detect = json.loads(sys.argv[1]); analyze = json.loads(sys.argv[2])
print("# Scalability & Performance Review — scaling-review\n")
print(f"Root: `{detect['root']}`\n")
s = analyze["summary"]
print("## Summary\n")
print("| Severity | Count |\n|---|---:|")
for k in ("high","medium","low"): print(f"| {k} | {s.get(k,0)} |")
print(f"| **total** | **{s.get('total',0)}** |\n")
print("## Dimensions detected\n")
print("| Dimension | Present | Evidence |\n|---|---|---|")
for d in detect["dimensions"]:
    ev = ", ".join(f"`{e}`" for e in d["evidence"]) or "—"
    print(f"| {d['id']} | {'✓' if d['present'] else '—'} | {ev} |")
print()
order = {"high":0,"medium":1,"low":2}
fs = sorted(analyze["findings"], key=lambda f:(order.get(f["severity"],9), f["dimension"]))
lbl = {"high":"🔴 High","medium":"🟠 Medium","low":"🟡 Low"}
print("## Findings\n")
print("_Each finding pairs a **fix** (the change) with a **validate** (how to prove it holds under scale). "
      "Findings are heuristic leads — confirm each via its validation method, not at face value._\n")
if not fs: print("_No findings._\n")
cur=None
for f in fs:
    if f["severity"]!=cur:
        cur=f["severity"]; print(f"\n### {lbl.get(cur,cur)}\n")
    print(f"#### [{f['dimension']}/{f['kind']}] `{f['path']}`")
    print(f"- **Problem:** {f['detail']}")
    print(f"- **Fix:** {f['fix']}")
    print(f"- **Validate:** {f['validate']}\n")
print("## Notes\n")
print("- scaling-review is heuristic (grep-based), not a profiler. Treat findings as `[INFERRED]` leads.")
print("- It does not gate a build. Read-only — no source modified.")
print("- Security of these same paths is owned by `security-scan`; operational deploy posture by `infra-scan`.")
PYEOF
}

if [ -n "$OUTPUT" ]; then
    mkdir -p "$(dirname "$OUTPUT")"
    render > "$OUTPUT"
    echo "✓ wrote $OUTPUT" >&2
    if [ "$SAVE" -eq 0 ]; then cat "$OUTPUT"; fi
else
    render
fi
