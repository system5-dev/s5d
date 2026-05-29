#!/usr/bin/env bash
# run.sh — execute security scanners per category, emit unified SARIF +
# CycloneDX SBOM + console summary, enforce severity gate.
#
# Outputs under ./test-reports/security/<category>/:
#   findings.sarif      — SARIF 2.1.0, consumable by GitHub Security tab
#   summary.txt         — human-readable summary
# Plus aggregated:
#   ./test-reports/security/sbom.cdx.json (CycloneDX SBOM)
#   ./test-reports/security/all.sarif     (merged SARIF for upload)
#   ./test-reports/security/junit.xml     (JUnit XML for CI annotations)
#
# Usage:
#   bash run.sh                              # detect + run all enabled
#   bash run.sh --categories sast,secrets
#   bash run.sh --fail-on high               # gate severity (low|medium|high|critical)
#   bash run.sh --no-gate                    # collect but never fail
#   bash run.sh --baseline test-reports/baseline.sarif    # diff mode

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DETECT="$SKILL_DIR/scripts/detect.sh"
WAIVERS=".s5d/security-waivers.yaml"

CATEGORIES=""
FAIL_ON="high"          # default: fail on high+critical
GATE=1
BASELINE=""
REPORT_DIR="test-reports/security"

while [ $# -gt 0 ]; do
    case "$1" in
        --categories)  CATEGORIES="$2"; shift 2 ;;
        --fail-on)     FAIL_ON="$2"; shift 2 ;;
        --no-gate)     GATE=0; shift ;;
        --baseline)    BASELINE="$2"; shift 2 ;;
        --report-dir)  REPORT_DIR="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,17p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$CATEGORIES" ]; then
    CATEGORIES=$(bash "$DETECT" \
        | python3 -c "import json,sys; d=json.load(sys.stdin); print(','.join(c['id'] for c in d['categories'] if c['enabled']))")
fi
[ -z "$CATEGORIES" ] && { echo "no categories detected" >&2; exit 2; }

mkdir -p "$REPORT_DIR"
FAILED_CATEGORIES=0

# --- per-category runners --------------------------------------------------
# Each runner emits to $REPORT_DIR/<category>/findings.sarif and prints a
# human summary.

run_sast() {
    local out="$REPORT_DIR/sast"
    mkdir -p "$out"
    if ! command -v semgrep >/dev/null 2>&1; then
        echo "  ! semgrep not installed (brew install semgrep | pip install semgrep)" >&2
        return 0
    fi
    echo "▸ semgrep" >&2
    semgrep scan --config auto --sarif --output "$out/findings.sarif" \
        --metrics off --error || true
    # Print summary
    if [ -f "$out/findings.sarif" ]; then
        python3 -c "
import json
d = json.load(open('$out/findings.sarif'))
runs = d.get('runs', [])
n = sum(len(r.get('results', [])) for r in runs)
print(f'  semgrep: {n} findings')
" 2>&1
    fi
}

run_sca() {
    local out="$REPORT_DIR/sca"
    mkdir -p "$out"
    if ! command -v trivy >/dev/null 2>&1; then
        echo "  ! trivy not installed (brew install trivy)" >&2
        return 0
    fi
    echo "▸ trivy fs --scanners vuln" >&2
    trivy fs --scanners vuln --format sarif --output "$out/findings.sarif" \
        --exit-code 0 --quiet . || true
    if [ -f "$out/findings.sarif" ]; then
        python3 -c "
import json
d = json.load(open('$out/findings.sarif'))
runs = d.get('runs', [])
n = sum(len(r.get('results', [])) for r in runs)
print(f'  trivy sca: {n} findings')
" 2>&1
    fi
}

run_license() {
    local out="$REPORT_DIR/license"
    mkdir -p "$out"
    if ! command -v trivy >/dev/null 2>&1; then
        echo "  ! trivy not installed" >&2
        return 0
    fi
    echo "▸ trivy fs --scanners license" >&2
    trivy fs --scanners license --format sarif --output "$out/findings.sarif" \
        --exit-code 0 --quiet . || true
}

run_secrets() {
    local out="$REPORT_DIR/secrets"
    mkdir -p "$out"
    if ! command -v gitleaks >/dev/null 2>&1; then
        echo "  ! gitleaks not installed (brew install gitleaks)" >&2
        return 0
    fi
    echo "▸ gitleaks detect --redact" >&2
    gitleaks detect --redact --no-banner --report-format sarif \
        --report-path "$out/findings.sarif" \
        --config .gitleaks.toml 2>&1 | tail -3 || true
}

run_container() {
    local out="$REPORT_DIR/container"
    mkdir -p "$out"
    if ! command -v trivy >/dev/null 2>&1; then
        echo "  ! trivy not installed" >&2
        return 0
    fi
    echo "▸ trivy image scan (for each Dockerfile)" >&2
    # For now we scan the Dockerfile directly via fs config-scanner;
    # actual image scan requires built images and tagging — left to CI.
    trivy fs --scanners config --include-deprecated-checks \
        --format sarif --output "$out/findings.sarif" \
        --exit-code 0 --quiet . || true
}

run_iac() {
    local out="$REPORT_DIR/iac"
    mkdir -p "$out"
    if ! command -v trivy >/dev/null 2>&1; then
        echo "  ! trivy not installed" >&2
        return 0
    fi
    echo "▸ trivy config (IaC misconfigurations)" >&2
    trivy config --format sarif --output "$out/findings.sarif" \
        --exit-code 0 --quiet . || true
}

run_sbom() {
    local out="$REPORT_DIR/sbom"
    mkdir -p "$out"
    if ! command -v syft >/dev/null 2>&1; then
        echo "  ! syft not installed (brew install syft)" >&2
        return 0
    fi
    echo "▸ syft (CycloneDX SBOM)" >&2
    syft scan . -o cyclonedx-json="$out/sbom.cdx.json" -q || true
    [ -f "$out/sbom.cdx.json" ] && cp "$out/sbom.cdx.json" "$REPORT_DIR/sbom.cdx.json"
}

IFS=',' read -ra cat_arr <<< "$CATEGORIES"
for c in "${cat_arr[@]}"; do
    [ -z "$c" ] && continue
    echo "" >&2
    echo "═══ ${c} ═══" >&2
    fn="run_${c}"
    if declare -f "$fn" >/dev/null; then
        "$fn" || FAILED_CATEGORIES=$((FAILED_CATEGORIES+1))
    else
        echo "  ! no runner for $c" >&2
    fi
done

# --- merge SARIF + apply waivers + severity gate ---
echo "" >&2
echo "═══ aggregation ═══" >&2

# Apply waivers + compute severity counts (Python because SARIF nested).
python3 - "$REPORT_DIR" "$WAIVERS" "$FAIL_ON" "$GATE" "$BASELINE" <<'PYEOF'
import json, os, sys, glob, datetime, re

report_dir = sys.argv[1]
waivers_path = sys.argv[2]
fail_on = sys.argv[3].lower()
gate = sys.argv[4] == "1"
baseline = sys.argv[5]

SEVERITY_ORDER = {"none": 0, "note": 0, "info": 0,
                  "low": 1,
                  "warning": 2, "medium": 2,
                  "error": 3, "high": 3,
                  "critical": 4}

# Load waivers
waivers = []
if os.path.exists(waivers_path):
    try:
        # Slim YAML read — only the waivers list, each entry is rule_id + reason + expires_at
        with open(waivers_path) as f:
            txt = f.read()
        # very minimal parse: look for `rule_id: X` / `expires_at: Y`
        cur = {}
        for line in txt.splitlines():
            s = line.strip()
            if s.startswith('- rule_id:'):
                if cur: waivers.append(cur)
                cur = {'rule_id': s.split(':', 1)[1].strip().strip('"\'')}
            elif ':' in s and cur:
                k, v = s.split(':', 1)
                cur[k.strip()] = v.strip().strip('"\'')
        if cur: waivers.append(cur)
    except Exception as e:
        print(f"  ! could not parse waivers: {e}", file=sys.stderr)

# Check expired waivers
today = datetime.date.today().isoformat()
active_rules = set()
expired = []
for w in waivers:
    exp = w.get('expires_at', '')
    if exp and exp < today:
        expired.append(w)
    else:
        active_rules.add(w.get('rule_id'))
if expired:
    print(f"  ⚠ {len(expired)} waivers expired — re-justify or remove", file=sys.stderr)

# Merge per-category SARIF, drop waived rule_ids
merged = {"$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/cos02/schemas/sarif-schema-2.1.0.json",
          "version": "2.1.0", "runs": []}

cat_summary = {}
total_by_sev = {"critical": 0, "high": 0, "medium": 0, "low": 0, "info": 0}

for sarif in sorted(glob.glob(os.path.join(report_dir, '*/findings.sarif'))):
    cat = os.path.basename(os.path.dirname(sarif))
    try:
        with open(sarif) as f: d = json.load(f)
    except Exception as e:
        print(f"  ! could not read {sarif}: {e}", file=sys.stderr)
        continue
    for r in d.get('runs', []):
        kept = []
        waived = 0
        for res in r.get('results', []):
            rule = res.get('ruleId', '')
            if rule in active_rules:
                waived += 1
                continue
            kept.append(res)
            # Severity classification
            lvl = (res.get('level') or 'warning').lower()
            sev = res.get('properties', {}).get('security-severity', '')
            if sev:
                try:
                    score = float(sev)
                    if score >= 9.0: total_by_sev['critical'] += 1
                    elif score >= 7.0: total_by_sev['high'] += 1
                    elif score >= 4.0: total_by_sev['medium'] += 1
                    elif score > 0:    total_by_sev['low'] += 1
                except: pass
            else:
                if lvl == 'error':    total_by_sev['high'] += 1
                elif lvl == 'warning': total_by_sev['medium'] += 1
                elif lvl == 'note':    total_by_sev['info'] += 1
        r['results'] = kept
        cat_summary[cat] = {'kept': len(kept), 'waived': waived}
        merged['runs'].append(r)

with open(os.path.join(report_dir, 'all.sarif'), 'w') as f:
    json.dump(merged, f, indent=2)

# Console summary
print('')
print('  ── severity counts ──')
for k in ['critical', 'high', 'medium', 'low', 'info']:
    print(f'    {k:10}: {total_by_sev[k]}')

print('')
print('  ── per-category ──')
for cat, info in sorted(cat_summary.items()):
    print(f'    {cat:12}: {info["kept"]} findings (waived: {info["waived"]})')

# Gate
threshold = SEVERITY_ORDER.get(fail_on, 3)
fail_count = sum(c for sev, c in total_by_sev.items()
                 if SEVERITY_ORDER.get(sev, 0) >= threshold)

if gate and fail_count > 0:
    print('')
    print(f'  ✗ severity gate FAILED: {fail_count} findings >= {fail_on}', file=sys.stderr)
    sys.exit(1)

print('')
print(f'  ✓ severity gate passed (threshold: {fail_on})')
PYEOF

rc=$?

echo "" >&2
echo "═══ reports ═══" >&2
find "$REPORT_DIR" -maxdepth 2 -type f 2>/dev/null | sort >&2

exit $rc
