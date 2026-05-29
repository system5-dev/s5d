#!/usr/bin/env bash
# triage.sh — dedup CVE findings across scanners + severity grouping.
#
# Multiple scanners often report the same CVE (Trivy + Snyk + Grype all find
# CVE-2024-XXXXX). triage collapses by CVE id, picks the highest severity
# reported, lists the scanners that flagged it.
#
# Output:
#   --format text     human readable
#   --format md       markdown for PR comments
#   --format json     machine readable
#
# Usage:
#   bash triage.sh                                  # text
#   bash triage.sh --format md > triage.md
#   bash triage.sh --severity high                  # filter

set -uo pipefail

REPORT_DIR="test-reports/security"
FORMAT="text"
MIN_SEVERITY=""

while [ $# -gt 0 ]; do
    case "$1" in
        --report-dir) REPORT_DIR="$2"; shift 2 ;;
        --format)     FORMAT="$2"; shift 2 ;;
        --severity)   MIN_SEVERITY="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

[ ! -d "$REPORT_DIR" ] && { echo "no report dir — run run.sh first" >&2; exit 2; }

python3 - "$REPORT_DIR" "$FORMAT" "$MIN_SEVERITY" <<'PYEOF'
import json, sys, os, glob, re

report_dir, fmt, min_sev = sys.argv[1:]
SEVERITY_RANK = {"critical": 4, "high": 3, "medium": 2, "low": 1, "info": 0, "none": 0}

# Collect findings grouped by CVE id.
by_cve = {}

for sarif in glob.glob(os.path.join(report_dir, '*/findings.sarif')):
    cat = os.path.basename(os.path.dirname(sarif))
    try:
        with open(sarif) as f: d = json.load(f)
    except Exception:
        continue
    for run in d.get('runs', []):
        tool = run.get('tool', {}).get('driver', {}).get('name', 'unknown')
        for res in run.get('results', []):
            rule = res.get('ruleId', '')
            msg = res.get('message', {}).get('text', '')
            # Extract CVE id from rule or message
            cve_match = re.search(r'CVE-\d{4}-\d+', rule + ' ' + msg)
            cve = cve_match.group(0) if cve_match else rule
            sev = (res.get('level') or 'warning').lower()
            sec_sev = res.get('properties', {}).get('security-severity', '')
            if sec_sev:
                try:
                    score = float(sec_sev)
                    sev_name = ('critical' if score >= 9 else
                                'high' if score >= 7 else
                                'medium' if score >= 4 else 'low')
                except: sev_name = sev
            else:
                sev_name = {'error': 'high', 'warning': 'medium', 'note': 'info'}.get(sev, sev)

            key = cve if cve_match else f"{tool}:{rule}"
            entry = by_cve.setdefault(key, {
                'cve': cve if cve_match else None,
                'rule': rule,
                'message': msg[:200],
                'severity': sev_name,
                'severity_rank': SEVERITY_RANK.get(sev_name, 0),
                'scanners': set(),
                'categories': set(),
            })
            entry['scanners'].add(tool)
            entry['categories'].add(cat)
            # Pick highest severity seen
            if SEVERITY_RANK.get(sev_name, 0) > entry['severity_rank']:
                entry['severity'] = sev_name
                entry['severity_rank'] = SEVERITY_RANK.get(sev_name, 0)

# Filter
if min_sev:
    threshold = SEVERITY_RANK.get(min_sev.lower(), 0)
    by_cve = {k: v for k, v in by_cve.items() if v['severity_rank'] >= threshold}

# Sort: severity desc, then by id
ordered = sorted(by_cve.values(), key=lambda v: (-v['severity_rank'], v.get('cve') or v.get('rule')))

if fmt == 'text':
    print(f"═══ triaged findings ({len(ordered)} unique) ═══\n")
    print(f"{'SEVERITY':10} {'ID':25} {'SCANNERS':30} CATEGORIES")
    print(f"{'-' * 10} {'-' * 25} {'-' * 30} {'-' * 30}")
    for e in ordered:
        ident = (e['cve'] or e['rule'])[:25]
        scs = ', '.join(sorted(e['scanners']))[:30]
        cats = ', '.join(sorted(e['categories']))
        print(f"{e['severity']:10} {ident:25} {scs:30} {cats}")
elif fmt == 'md':
    print(f"# Security Triage\n\n**Unique findings:** {len(ordered)}\n")
    print("| Severity | ID | Scanners | Categories | Message |")
    print("|---|---|---|---|---|")
    for e in ordered:
        ident = e['cve'] or e['rule']
        scs = ', '.join(sorted(e['scanners']))
        cats = ', '.join(sorted(e['categories']))
        msg = (e['message'] or '')[:80].replace('|', '\\|')
        print(f"| {e['severity']} | `{ident}` | {scs} | {cats} | {msg} |")
elif fmt == 'json':
    out = []
    for e in ordered:
        out.append({
            'severity': e['severity'],
            'cve': e['cve'],
            'rule': e['rule'],
            'message': e['message'],
            'scanners': sorted(e['scanners']),
            'categories': sorted(e['categories']),
        })
    print(json.dumps({'total': len(out), 'findings': out}, indent=2))
PYEOF
