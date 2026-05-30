#!/usr/bin/env bash
# report.sh — render detect + analyze into a documentation reorganization plan. Read-only.
# Stdout, or --save → test-reports/docs/report.md.

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
[ "$SAVE" -eq 1 ] && [ -z "$OUTPUT" ] && OUTPUT="test-reports/docs/report.md"
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }
DETECT="$(bash "$HERE/detect.sh" "$ROOT" 2>/dev/null)"
ANALYZE="$(bash "$HERE/analyze.sh" "$ROOT" 2>/dev/null)"

render() {
python3 - "$DETECT" "$ANALYZE" <<'PYEOF'
import json, sys
d = json.loads(sys.argv[1]); a = json.loads(sys.argv[2])
print("# Documentation Reorganization Plan — ai-technical-writer\n")
print(f"Root: `{d['root']}`\n")
print("## Inventory\n")
print("| Metric | Value |\n|---|---|")
print(f"| Markdown files | {d['markdown_total']} |")
print(f"| Directories holding docs | {d['markdown_dirs']} |")
print(f"| Scattered outside docs/ | {d['scattered_outside_docs']} |")
print(f"| Front-matter docs | {d['frontmatter_docs']} |")
an = d['anchors']
print(f"| docs/ tree | {'✓' if an['docs_dir'] else '—'} |")
print(f"| docs index | {'✓' if an['docs_index'] else '—'} |")
print(f"| llms.txt | {'✓' if an['llms_txt'] else '—'} |")
print(f"| **machine-readable** | {'✓' if d['machine_readable'] else '**no**'} |\n")
s = a["summary"]
print("## Findings summary\n")
print("| Severity | Count |\n|---|---:|")
for k in ("high","medium","low"): print(f"| {k} | {s.get(k,0)} |")
print(f"| **total** | **{s.get('total',0)}** |\n")
order = {"high":0,"medium":1,"low":2}
fs = sorted(a["findings"], key=lambda f:(order.get(f["severity"],9), f["check"]))
lbl = {"high":"High","medium":"Medium","low":"Low"}
print("## Findings\n")
print("_Each finding pairs a Fix with a Validate (a link-checker run, a front-matter "
      "assertion, a reachability walk). Goal: one home, consistent, cross-linked, "
      "machine-readable._\n")
cur=None
for f in fs:
    if f["severity"]!=cur:
        cur=f["severity"]; print(f"\n### {lbl.get(cur,cur)}\n")
    print(f"#### {f['check']} — {f['path']}")
    print(f"- **Problem:** {f['detail']}")
    print(f"- **Fix:** {f['fix']}")
    print(f"- **Validate:** {f['validate']}\n")
print("## Target shape\n")
print("```")
print("docs/")
print("  index.md          # table of contents — every doc linked from here")
print("  <topic>/...        # docs grouped by domain/topic, kebab-case names")
print("README.md           # short; points to docs/index.md")
print("llms.txt            # machine-readable: title + summary + curated links")
print("```")
print("\nGenerate the index + llms.txt skeleton from the current inventory with:")
print("`bash .claude/skills/ai-technical-writer/scripts/generate.sh --save`\n")
print("Read-only report — no docs were moved or modified.")
PYEOF
}
if [ -n "$OUTPUT" ]; then
    mkdir -p "$(dirname "$OUTPUT")"; render > "$OUTPUT"; echo "✓ wrote $OUTPUT" >&2
    if [ "$SAVE" -eq 0 ]; then cat "$OUTPUT"; fi
else
    render
fi
