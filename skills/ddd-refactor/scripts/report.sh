#!/usr/bin/env bash
# report.sh — render detect.sh + analyze.sh into a markdown DDD refactoring plan.
# Read-only. Stdout, or --save → test-reports/ddd/report.md.

set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(pwd)"; OUTPUT=""; SAVE=0
while [ $# -gt 0 ]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --save) SAVE=1; shift ;;
        --root) ROOT="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done
[ "$SAVE" -eq 1 ] && [ -z "$OUTPUT" ] && OUTPUT="test-reports/ddd/report.md"
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }
DETECT="$(bash "$HERE/detect.sh" "$ROOT" 2>/dev/null)"
ANALYZE="$(bash "$HERE/analyze.sh" "$ROOT" 2>/dev/null)"

render() {
python3 - "$DETECT" "$ANALYZE" <<'PYEOF'
import json, sys
d = json.loads(sys.argv[1]); a = json.loads(sys.argv[2])
print("# DDD Refactoring Plan — ddd-refactor\n")
print(f"Root: `{d['root']}`\n")
s = a["summary"]
print("## Summary\n\n| Severity | Count |\n|---|---:|")
for k in ("high","medium","low"): print(f"| {k} | {s.get(k,0)} |")
print(f"| **total** | **{s.get('total',0)}** |\n")
print("## DDD signals detected\n\n| Signal | Present | Evidence |\n|---|---|---|")
for sg in d["signals"]:
    print(f"| {sg['id']} | {'yes' if sg['present'] else 'no'} | {sg['evidence']} |")
print()
order = {"high":0,"medium":1,"low":2}
fs = sorted(a["findings"], key=lambda f:(order.get(f["severity"],9), f["check"]))
lbl = {"high":"High","medium":"Medium","low":"Low"}
print("## Findings\n")
print("_Each finding is a DDD move (Fix) paired with a way to prove it landed (Validate). "
      "Heuristic leads — confirm before acting. This skill assesses DDD modeling quality; "
      "boundary-vs-architecture-map drift is `domain-refactor`'s job._\n")
cur=None
for f in fs:
    if f["severity"]!=cur:
        cur=f["severity"]; print(f"\n### {lbl.get(cur,cur)}\n")
    print(f"#### {f['check']} — `{f['path']}`")
    print(f"- **Problem:** {f['detail']}")
    print(f"- **Fix:** {f['fix']}")
    print(f"- **Validate:** {f['validate']}\n")
print("## Suggested sequencing\n")
print("1. **Value objects first** (Money, typed ids) — small, high-leverage, no behavior change.")
print("2. **Anti-corruption layers** at the carrier/payment/storage seams — isolates vendor churn.")
print("3. **Repositories** per aggregate — pulls Prisma out of controllers.")
print("4. **Use-case extraction** — shrink transaction-script handlers onto domain + repo.")
print("5. **Aggregates** own their invariants and child writes.")
print("6. **Resolve language drift** — one canonical model (ties to the Go-backend ADR).\n")
print("Each step needs a test safety-net first (see unit-tests / e2e coverage). Read-only report — no source changed.")
PYEOF
}
if [ -n "$OUTPUT" ]; then
    mkdir -p "$(dirname "$OUTPUT")"; render > "$OUTPUT"; echo "✓ wrote $OUTPUT" >&2
    if [ "$SAVE" -eq 0 ]; then cat "$OUTPUT"; fi
else
    render
fi
