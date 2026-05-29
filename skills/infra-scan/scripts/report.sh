#!/usr/bin/env bash
# report.sh — render detect.sh + analyze.sh into a markdown infra-posture report.
# Read-only. Writes to stdout, or to --output PATH (default test-reports/infra/report.md
# when --save is passed). Never mutates project source.
#
#   bash report.sh                         # markdown to stdout
#   bash report.sh --save                  # also write test-reports/infra/report.md
#   bash report.sh --output infra.md       # write to a chosen path

set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(pwd)"
OUTPUT=""
SAVE=0
while [ $# -gt 0 ]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        --save)   SAVE=1; shift ;;
        --root)   ROOT="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done
[ "$SAVE" -eq 1 ] && [ -z "$OUTPUT" ] && OUTPUT="test-reports/infra/report.md"

command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

DETECT="$(bash "$HERE/detect.sh" "$ROOT" 2>/dev/null)"
ANALYZE="$(bash "$HERE/analyze.sh" "$ROOT" 2>/dev/null)"

render() {
python3 - "$DETECT" "$ANALYZE" <<'PYEOF'
import json, sys
detect = json.loads(sys.argv[1])
analyze = json.loads(sys.argv[2])

print("# Infrastructure Posture — infra-scan\n")
print(f"Root: `{detect['root']}`\n")

s = analyze["summary"]
print("## Summary\n")
print("| Severity | Count |")
print("|---|---:|")
for k in ("high", "medium", "low", "info"):
    print(f"| {k} | {s.get(k,0)} |")
print(f"| **total** | **{s.get('total',0)}** |\n")

# surfaces
print("## Deploy surfaces detected\n")
print("| Surface | Present | Evidence |")
print("|---|---|---|")
for su in detect["surfaces"]:
    ev = ", ".join(f"`{e}`" for e in su["evidence"]) or "—"
    print(f"| {su['id']} | {'✓' if su['present'] else '—'} | {ev} |")
print()

# findings grouped by severity
order = {"high": 0, "medium": 1, "low": 2, "info": 3}
fs = sorted(analyze["findings"], key=lambda f: (order.get(f["severity"], 9), f["surface"]))
sev_label = {"high": "🔴 High", "medium": "🟠 Medium", "low": "🟡 Low", "info": "ℹ️ Info"}
cur = None
print("## Findings\n")
if not fs:
    print("_No findings._\n")
for f in fs:
    if f["severity"] != cur:
        cur = f["severity"]
        print(f"\n### {sev_label.get(cur, cur)}\n")
    print(f"- **[{f['surface']}/{f['kind']}]** `{f['path']}` — {f['detail']}")
print()

# decision pointer
print("## Notes\n")
print("- infra-scan reports **operational** posture (topology, drift, sizing, env-flow).")
print("  Security misconfig + CVEs in these same files are owned by `security-scan` (trivy config / IaC).")
print("- This report is read-only. No deploy configs were modified.")
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
