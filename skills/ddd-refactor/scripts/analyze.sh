#!/usr/bin/env bash
# analyze.sh — DDD modeling findings. Read-only. JSON: findings[{check,severity,path,detail,fix,validate}].
#
# Heuristic [INFERRED] leads, not runtime truth. Every finding pairs a Fix (the DDD move)
# with a Validate (how to prove the move landed — a test, a grep that must return empty,
# a dependency-direction check). Distinct from domain-refactor (boundary-vs-map).

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"
json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }
INCL="--include=*.ts --include=*.tsx"
prune() { grep -vE 'node_modules|/\.next/|test-reports|/\.git/'; }

FINDINGS=""
add() { FINDINGS="${FINDINGS}${1}	${2}	${3}	${4}	${5}	${6}"$'\n'; }  # check sev path detail fix validate

# 1 — anemic domain: ORM models are pure data, no behavior; logic elsewhere
models=0; [ -f prisma/schema.prisma ] && models=$(grep -cE '^model ' prisma/schema.prisma 2>/dev/null || echo 0)
has_domain=0
for d in domain src/domain app/domain lib/domain; do [ -d "$d" ] && has_domain=1; done
if [ "$models" -gt 0 ] && [ "$has_domain" -eq 0 ]; then
    add anemic-domain high "prisma/schema.prisma" \
        "$models Prisma models are data-only records with no behavior; there is no domain/ layer — business rules live in controllers/utils (anemic domain model)" \
        "Introduce a domain layer: wrap key aggregates (Policy, Quote, Payout) in rich types that own their invariants (state transitions, premium rules), keep Prisma as persistence only" \
        "grep for business rules (premium/bind/refund logic) inside app/api — after the move it returns 0 hits in route handlers; invariants are unit-tested on the domain type, not the route."
fi

# 2 — transaction-script: large handlers doing everything
big=$(grep -rlE 'export (async )?function (GET|POST|PUT|PATCH|DELETE)' $INCL app/api 2>/dev/null | prune | while read -r f; do
    lc=$(wc -l < "$f" 2>/dev/null | tr -d ' '); [ "${lc:-0}" -gt 300 ] && echo "$f:$lc"; done | sort -t: -k2 -rn | head -12)
big_n=$(printf '%s\n' "$big" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$big_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$big" | head -4 | paste -sd', ' -)
    add transaction-script high "$sample" \
        "$big_n route handler(s) exceed 300 LOC — they orchestrate DB + external calls + business rules inline (transaction script, not a domain model)" \
        "Extract the procedure into a use-case/application service that calls domain methods + a repository; the handler should only parse input, call the use case, and map output" \
        "Handler shrinks to < ~60 LOC and contains no premium/bind/branching logic; the extracted use case has unit tests; behavior unchanged (same e2e)."
fi

# 3 — domain logic in controllers
dl=$(grep -rlE 'calculatePremium|computePrice|premium|taxRate|bindPolicy|refund.*amount|bps' $INCL app/api 2>/dev/null | prune | head -15)
dl_n=$(printf '%s\n' "$dl" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$dl_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$dl" | head -4 | paste -sd', ' -)
    add domain-logic-in-controllers high "$sample" \
        "$dl_n controller(s) contain domain calculations (premium/tax/bind/refund) — money rules are scattered across the HTTP layer instead of one domain module" \
        "Move all pricing/binding rules into a single domain service (e.g. lib/domain/pricing, lib/domain/policy); controllers call it" \
        "A grep for premium/tax math in app/api returns 0; the domain service is the single source of pricing truth and is unit-tested with known ratios."
fi

# 4 — value objects as primitives (primitive obsession on money/identifiers)
vo=$(grep -rhoE '(amount|premium|price|total|payout)[A-Za-z]*\s*:\s*number|policyNumber\s*:\s*string|email\s*:\s*string' $INCL lib utils app 2>/dev/null | prune | sort -u | head -8)
if grep -rqE ':\s*number' $INCL lib/server-pricing.ts utils/pricing.ts 2>/dev/null; then
    add value-objects-as-primitives medium "lib/server-pricing.ts, utils/pricing.ts, utils/payment.ts" \
        "Money is passed as bare \`number\` and identifiers (policyNumber, email) as bare \`string\` — primitive obsession; no Money/Email/PolicyNumber value objects, so rounding/currency/format rules aren't enforced by the type" \
        "Introduce value objects: a Money type (integer cents + currency, with add/multiply) and typed identifiers; replace raw number arithmetic on currency" \
        "All currency math goes through Money (grep for raw \`* 100\`/\`/ 100\` on amounts returns 0 outside Money); a test asserts Money rejects fractional cents and mismatched currencies."
fi

# 5 — missing anti-corruption layer at integration seams
for seam in tmhcc stripe box; do
    files=$(grep -rliE "$seam" $INCL lib app 2>/dev/null | prune | grep -ivE "lib/$seam/|adapter|/acl/" | head -30)
    fn=$(printf '%s\n' "$files" | sed '/^$/d' | wc -l | tr -d ' ')
    if [ "$fn" -gt 5 ]; then
        add missing-acl high "(${fn} files touch $seam)" \
            "The $seam integration is referenced directly in $fn files outside a dedicated adapter — no anti-corruption layer; the external vendor's vocabulary/shape leaks into the domain" \
            "Put one adapter/ACL module (lib/$seam/adapter) that translates $seam DTOs ↔ domain types; everything else depends on the domain interface, not the vendor SDK" \
            "Only the adapter imports the $seam SDK (grep for the SDK import outside lib/$seam returns 0); domain code references domain types, swapping the vendor touches one module."
    fi
done

# 6 — repository pattern absence: prisma called directly in controllers
repo=$(grep -rlE 'prisma\.[a-zA-Z]+\.(find|create|update|delete|upsert)' $INCL app/api 2>/dev/null | prune | head -40)
repo_n=$(printf '%s\n' "$repo" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$repo_n" -gt 5 ]; then
    add repository-absence medium "(${repo_n} handlers call prisma directly)" \
        "$repo_n controllers call Prisma directly — persistence concerns are spread across the HTTP layer, no repository abstraction; aggregates can be mutated from anywhere" \
        "Introduce repositories per aggregate (PolicyRepository, QuoteRepository) that encapsulate Prisma; controllers/use-cases depend on the repository interface" \
        "Route handlers contain no \`prisma.\` calls (grep returns 0 in app/api); persistence is swappable behind the repository interface; aggregate writes go through one place."
fi

# 7 — aggregate boundary leak: a handler writes 2+ distinct models in one go w/o an aggregate
agg=$(grep -rlE 'prisma\.[a-zA-Z]+\.(create|update|upsert|delete)' $INCL app/api 2>/dev/null | prune | while read -r f; do
    distinct=$(grep -oE 'prisma\.[a-zA-Z]+\.' "$f" 2>/dev/null | sort -u | wc -l | tr -d ' '); [ "${distinct:-0}" -ge 3 ] && echo "$f:${distinct}models"; done | head -10)
agg_n=$(printf '%s\n' "$agg" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$agg_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$agg" | head -4 | paste -sd', ' -)
    add aggregate-boundary-leak medium "$sample" \
        "$agg_n handler(s) write 3+ different models in one operation with no aggregate root coordinating them — invariants spanning those entities aren't enforced together; partial writes leave inconsistent state" \
        "Define aggregate roots (e.g. Policy owns AdditionalInsured, Coi) and mutate child entities only through the root inside a single transaction" \
        "Cross-entity writes happen inside one \`prisma.\$transaction\` via the aggregate; a test forces a mid-write failure and asserts no partial state persists."
fi

# 8 — ubiquitous-language drift (parallel TS + Go models of the same domain)
if [ -d gigg-insurance-master ] && grep -rqiE 'type Policy|interface Policy|model Policy' prisma lib 2>/dev/null; then
    add ubiquitous-language-drift medium "lib/tmhcc + gigg-insurance-master/app/tmhcc" \
        "The same domain (Policy, TMHCC quote/bind, COI) is modeled twice — a TypeScript implementation and the orphaned Go backend — with independent definitions; one ubiquitous language has forked into two dialects" \
        "Pick one canonical model (the live TS one) and retire/extract the duplicate (see system-design ADR on the Go backend); align names to a single glossary" \
        "Only one definition of each aggregate exists in the live codebase; a glossary doc maps each term to one type; the duplicate is removed or in a separate repo."
fi

# --- JSON ---
ch=0; cm=0; cl=0
printf '{\n  "root": "%s",\n  "findings": [' "$(json_str "$ROOT")"
first=1
while IFS=$'\t' read -r check sev path detail fix validate; do
    [ -z "$check" ] && continue
    case "$sev" in high) ch=$((ch+1));; medium) cm=$((cm+1));; low) cl=$((cl+1));; esac
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"check":"%s","severity":"%s","path":"%s","detail":"%s","fix":"%s","validate":"%s"}' \
        "$(json_str "$check")" "$(json_str "$sev")" "$(json_str "$path")" \
        "$(json_str "$detail")" "$(json_str "$fix")" "$(json_str "$validate")"
done <<< "$FINDINGS"
[ "$first" -eq 0 ] && printf '\n  '
printf '],\n  "summary": {"high": %d, "medium": %d, "low": %d, "total": %d}\n}\n' \
    "$ch" "$cm" "$cl" "$((ch+cm+cl))"
