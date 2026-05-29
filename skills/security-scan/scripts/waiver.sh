#!/usr/bin/env bash
# waiver.sh — add / list / check / expire security findings waivers.
#
# Waivers store: .s5d/security-waivers.yaml
# Each waiver is a versioned entry: rule_id + reason + condition +
# approved_by + approved_at + expires_at.
#
# Usage:
#   bash waiver.sh add <rule-id> --reason "..." --expires 2026-08-01 \
#                                --approved-by Roman --condition "..."
#   bash waiver.sh list                    # all waivers
#   bash waiver.sh list --active           # currently active only
#   bash waiver.sh list --expired          # past expiry
#   bash waiver.sh check <rule-id>         # is this rule_id waived right now?

set -uo pipefail

WAIVERS=".s5d/security-waivers.yaml"

usage() {
    sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
    exit "${1:-0}"
}

[ $# -lt 1 ] && usage 1
OP="$1"; shift

ensure_waivers_file() {
    if [ ! -f "$WAIVERS" ]; then
        mkdir -p "$(dirname "$WAIVERS")"
        cat > "$WAIVERS" <<'YAML'
# .s5d/security-waivers.yaml — security finding waivers.
# Append-only. Expired waivers stay for the audit trail; security-scan/run.sh
# treats them as inactive. Re-justify by adding a new entry, never edit history.

waivers:
YAML
    fi
}

today=$(date -u +%Y-%m-%d)

case "$OP" in
    add)
        [ $# -lt 1 ] && { echo "usage: waiver.sh add <rule-id> --reason X --expires YYYY-MM-DD --approved-by NAME [--condition X]" >&2; exit 1; }
        RULE_ID="$1"; shift
        REASON=""; EXPIRES=""; APPROVER=""; CONDITION="see linked spec"

        while [ $# -gt 0 ]; do
            case "$1" in
                --reason)       REASON="$2"; shift 2 ;;
                --expires)      EXPIRES="$2"; shift 2 ;;
                --approved-by)  APPROVER="$2"; shift 2 ;;
                --condition)    CONDITION="$2"; shift 2 ;;
                *) echo "unknown arg: $1" >&2; exit 2 ;;
            esac
        done

        [ -z "$REASON" ] && { echo "✗ --reason required" >&2; exit 1; }
        [ -z "$EXPIRES" ] && { echo "✗ --expires required (YYYY-MM-DD)" >&2; exit 1; }
        [ -z "$APPROVER" ] && { echo "✗ --approved-by required" >&2; exit 1; }
        echo "$EXPIRES" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' || { echo "✗ --expires must be YYYY-MM-DD" >&2; exit 1; }
        if [ "$EXPIRES" \< "$today" ]; then
            echo "✗ --expires is in the past" >&2; exit 1
        fi

        ensure_waivers_file
        # Pre-escape embedded double quotes BEFORE heredoc (avoids nested-quote pain).
        REASON_ESC=$(printf '%s' "$REASON" | sed 's/"/\\"/g')
        CONDITION_ESC=$(printf '%s' "$CONDITION" | sed 's/"/\\"/g')
        cat >> "$WAIVERS" <<YAML
  - rule_id: "$RULE_ID"
    reason: "$REASON_ESC"
    condition: "$CONDITION_ESC"
    approved_by: "$APPROVER"
    approved_at: "$today"
    expires_at: "$EXPIRES"
YAML
        echo "✓ waiver added for $RULE_ID (expires $EXPIRES)" >&2
        ;;

    list)
        FILTER=""
        while [ $# -gt 0 ]; do
            case "$1" in
                --active)  FILTER="active"; shift ;;
                --expired) FILTER="expired"; shift ;;
                *) echo "unknown arg: $1" >&2; exit 2 ;;
            esac
        done

        [ ! -f "$WAIVERS" ] && { echo "(no waivers file yet)"; exit 0; }

        python3 - "$WAIVERS" "$FILTER" "$today" <<'PYEOF'
import sys, re

path, filt, today = sys.argv[1:]
waivers = []
cur = {}
for line in open(path):
    s = line.strip()
    if s.startswith('- rule_id:'):
        if cur: waivers.append(cur)
        cur = {'rule_id': s.split(':', 1)[1].strip().strip('"\'')}
    elif ':' in s and cur and not s.startswith('#'):
        k, v = s.split(':', 1)
        cur[k.strip()] = v.strip().strip('"\'')
if cur: waivers.append(cur)

print(f"\n{'STATUS':8}  {'RULE':35}  {'EXPIRES':12}  {'APPROVED-BY':15}  REASON")
print(f"{'------':8}  {'-' * 35}  {'-' * 12}  {'-' * 15}  ------")
for w in waivers:
    expires = w.get('expires_at', '')
    rule = w.get('rule_id', '')
    approver = w.get('approved_by', '')
    reason = w.get('reason', '')[:60]
    expired = expires and expires < today
    status = "expired" if expired else "active"
    if filt and ((filt == 'active' and expired) or (filt == 'expired' and not expired)):
        continue
    print(f"{status:8}  {rule:35}  {expires:12}  {approver:15}  {reason}")
PYEOF
        ;;

    check)
        [ $# -lt 1 ] && { echo "usage: waiver.sh check <rule-id>" >&2; exit 1; }
        RULE_ID="$1"
        [ ! -f "$WAIVERS" ] && { echo "no waivers file — rule $RULE_ID is not waived"; exit 1; }
        # Use temp Python file (avoids heredoc-inside-$() parser pain in bash 3.2).
        TMPPY=$(mktemp -t waiver-check.XXXXXX.py)
        cat > "$TMPPY" <<'PYEOF'
import sys
path, rule, today = sys.argv[1:]
cur = {}
for line in open(path):
    s = line.strip()
    if s.startswith('- rule_id:'):
        if cur and cur.get('rule_id') == rule:
            if cur.get('expires_at', '0000-00-00') >= today:
                print('1'); raise SystemExit
        cur = {'rule_id': s.split(':', 1)[1].strip().strip('"\'')}
    elif ':' in s and cur and not s.startswith('#'):
        k, v = s.split(':', 1)
        cur[k.strip()] = v.strip().strip('"\'')
if cur and cur.get('rule_id') == rule:
    if cur.get('expires_at', '0000-00-00') >= today:
        print('1')
PYEOF
        active=$(python3 "$TMPPY" "$WAIVERS" "$RULE_ID" "$today")
        rm -f "$TMPPY"
        if [ "$active" = "1" ]; then
            echo "✓ $RULE_ID is actively waived"
            exit 0
        else
            echo "✗ $RULE_ID is not waived"
            exit 1
        fi
        ;;

    *)
        echo "unknown op: $OP" >&2
        usage 1
        ;;
esac
