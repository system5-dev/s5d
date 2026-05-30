#!/usr/bin/env bash
# detect.sh — what DDD-relevant structure does this repo have? Read-only. JSON out.
#
# Distinct from `domain-refactor` (which scans boundary violations vs the architecture
# map). This skill assesses DDD MODELING: is the domain rich or anemic, are there
# aggregates / value objects / an anti-corruption layer, or is logic a transaction
# script in the controllers.
#
# Signals detected:
#   orm-entities   — Prisma/TypeORM/SQLAlchemy/GORM models (the candidate "entities")
#   domain-layer   — explicit domain/ application/ infrastructure/ separation present?
#   controllers    — HTTP handlers (where domain logic tends to leak)
#   integ-seams    — external systems (carrier/payment/storage SDKs) = ACL candidates
#   value-types    — presence of dedicated value-object types (Money, Email, …) vs primitives
#   events         — domain-event / event-bus presence

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"
json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }
dep_seen() { grep -qiE "$1" package.json go.mod gigg-insurance-master/go.mod pyproject.toml requirements.txt 2>/dev/null; }
prune_find() {
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
        -o -name build -o -name dist -o -name .next -o -name .venv -o -name __pycache__ \
        -o -name .claude -o -name .codex -o -name .agents -o -name test-reports \
        -o -name .playwright-browsers \) -prune \
        -o -type f -name "$1" -print 2>/dev/null | sed 's|^\./||' | head -n "${2:-500}"
}

ENTRIES=""
add() { ENTRIES="${ENTRIES}${1}	${2}	${3}"$'\n'; }   # signal present evidence

# --- orm-entities (the candidate aggregates/entities) ---
ent=""
if [ -f prisma/schema.prisma ]; then
    n=$(grep -cE '^model ' prisma/schema.prisma 2>/dev/null || echo 0)
    ent="prisma:${n} models"
fi
dep_seen 'gorm.io|jinzhu/gorm' && ent="${ent:+$ent,}gorm"
dep_seen 'sqlalchemy' && ent="${ent:+$ent,}sqlalchemy"
[ -n "$ent" ] && add orm-entities yes "$ent" || add orm-entities no ""

# --- domain-layer separation (DDD layering) ---
dl=""
for d in domain src/domain app/domain application src/application infrastructure src/infrastructure core/domain; do
    [ -d "$d" ] && dl="${dl:+$dl,}$d"
done
# Go ddd-ish layout
[ -d gigg-insurance-master/app/module ] && dl="${dl:+$dl,}go:app/module"
[ -n "$dl" ] && add domain-layer yes "$dl" || add domain-layer no "flat (logic likely in controllers/services)"

# --- controllers (where logic leaks) ---
ctrl=$(( $(prune_find 'route.ts' 2000 | grep -cE '/api/' || true) + $(prune_find 'route.js' 2000 | grep -cE '/api/' || true) ))
go_h=$(grep -rlE 'http\.(HandleFunc|Handle)|chi\.|gin\.' --include='*.go' . 2>/dev/null | grep -v vendor | wc -l | tr -d ' ')
total_ctrl=$((ctrl + go_h))
[ "$total_ctrl" -gt 0 ] && add controllers yes "${total_ctrl} handlers" || add controllers no ""

# --- integration seams (anti-corruption-layer candidates) ---
seams=""
grep -rqiE 'tmhcc|carrier' --include='*.ts' lib app 2>/dev/null && seams="${seams:+$seams,}tmhcc-carrier"
dep_seen '"stripe"|stripe-node' && seams="${seams:+$seams,}stripe"
grep -rqiE 'box-node-sdk|box.com|/lib/box' --include='*.ts' . 2>/dev/null && seams="${seams:+$seams,}box"
dep_seen 'aws-sdk|@aws-sdk' && seams="${seams:+$seams,}aws"
[ -n "$seams" ] && add integ-seams yes "$seams" || add integ-seams no ""

# --- value-object types (vs primitive obsession) ---
vo=""
grep -rqiE 'class Money|type Money|Money =|class Email|value-object|ValueObject' --include='*.ts' --include='*.go' . 2>/dev/null \
    | grep -vq node_modules && vo="found"
[ -n "$vo" ] && add value-types yes "$vo" || add value-types no "domain concepts likely primitives (string/number)"

# --- domain events ---
ev=""
dep_seen 'eventemitter|mitt|event-bus' && ev="event-lib"
grep -rqiE 'domainEvent|DomainEvent|publishEvent|emitEvent' --include='*.ts' --include='*.go' . 2>/dev/null \
    | grep -vq node_modules && ev="${ev:+$ev,}domain-events"
[ -n "$ev" ] && add events yes "$ev" || add events no "no explicit domain events"

# --- JSON ---
present=0
printf '{\n  "root": "%s",\n  "signals": [' "$(json_str "$ROOT")"
first=1
while IFS=$'\t' read -r sig pres ev; do
    [ -z "$sig" ] && continue
    [ "$pres" = "yes" ] && present=$((present+1))
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","present":%s,"evidence":"%s"}' "$sig" \
        "$([ "$pres" = "yes" ] && echo true || echo false)" "$(json_str "$ev")"
done <<< "$ENTRIES"
[ "$first" -eq 0 ] && printf '\n  '
printf '],\n  "summary": {"present": %d, "signals_total": 6}\n}\n' "$present"
