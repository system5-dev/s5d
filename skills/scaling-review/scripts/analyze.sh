#!/usr/bin/env bash
# analyze.sh — scalability/performance findings. Read-only.
# Output: JSON { root, findings:[{dimension,kind,severity,path,detail,fix,validate}], summary }.
#
# Every finding carries BOTH a `fix` (the change) and a `validate` (how to prove the fix
# actually holds under scale) — a change without a validation is not actionable.
#
# Heuristic, not a profiler: it greps for known scaling anti-patterns. Treat findings as
# [INFERRED] leads to confirm with the validation method, not [OBSERVED] runtime truth.

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"
json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }

FINDINGS=""
add() {  # dimension kind severity path detail fix validate
    FINDINGS="${FINDINGS}${1}	${2}	${3}	${4}	${5}	${6}	${7}"$'\n'
}

# count helper (grep -c that never aborts under set -u/pipefail)
gcount() { grep -rE "$1" $2 . 2>/dev/null | grep -vE 'node_modules|/\.git/|/\.next/|test-reports|/vendor/' | wc -l | tr -d ' '; }

INCL_TS="--include=*.ts --include=*.tsx --include=*.js"

# ─────────── db: N+1 (await inside a map/loop) ───────────
np1=$(grep -rEn '\.map\([^)]*await (prisma|db)\.|for *\([^)]*\) *\{[^}]*await (prisma|db)\.' $INCL_TS . 2>/dev/null \
    | grep -vE 'node_modules|/\.next/|test-reports' | head -40)
np1_n=$(printf '%s\n' "$np1" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$np1_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$np1" | head -3 | sed 's/:.*//' | sort -u | paste -sd, -)
    add db n-plus-1 high "$sample" \
        "$np1_n site(s) await a DB call inside a loop/map — query count grows with row count" \
        "Batch with a single findMany({where:{id:{in:[...]}}}) or use include/select to join in one query" \
        "Enable Prisma query logging; the endpoint issues 1-2 queries regardless of N rows. Load-test with 10x rows: p95 must stay flat, not climb linearly."
fi

# ─────────── db: unbounded query (findMany without take) ───────────
unb=$(grep -rEn 'findMany\(' $INCL_TS . 2>/dev/null | grep -vE 'node_modules|/\.next/|test-reports' | grep -vE 'take:' | head -40)
unb_n=$(printf '%s\n' "$unb" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$unb_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$unb" | head -3 | sed 's/:.*//' | sort -u | paste -sd, -)
    add db unbounded-query high "$sample" \
        "$unb_n findMany call(s) without take/cursor — full-table reads that grow unbounded with data" \
        "Add take + cursor/offset pagination; cap default page size; index the order-by column" \
        "Seed 100k rows; endpoint returns <= page size and p95 stays < 300ms with flat memory. EXPLAIN shows an index scan, not a seq scan."
fi

# ─────────── api: heavy/sync work in the request path ───────────
heavy=$(grep -rlEn 'setContent\(|puppeteer|generatePdf|pdfkit|sharp\(' $INCL_TS . 2>/dev/null \
    | grep -E '/api/|/app/.*route\.' | grep -vE 'node_modules|/\.next/|test-reports' | head -20)
heavy_n=$(printf '%s\n' "$heavy" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$heavy_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$heavy" | head -3 | paste -sd, -)
    add api sync-heavy-in-request high "$sample" \
        "$heavy_n route handler(s) run heavy work (PDF/Puppeteer/image) inline — ties up the request, no backpressure, serverless timeout risk" \
        "Move to a background job/queue; return 202 + a status URL; render off the hot path" \
        "Load-test the endpoint at concurrency C: request p95 drops to the enqueue time; queue depth is observable; a failed job retries without losing the request."
fi

# ─────────── api: external call in request path without timeout ───────────
noto=$(grep -rEn 'fetch\(|axios\.' $INCL_TS . 2>/dev/null | grep -E '/api/|route\.' \
    | grep -vE 'node_modules|/\.next/|test-reports|AbortController|signal:|timeout' | head -40)
noto_n=$(printf '%s\n' "$noto" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$noto_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$noto" | head -3 | sed 's/:.*//' | sort -u | paste -sd, -)
    add concurrency missing-timeout medium "$sample" \
        "$noto_n outbound call(s) in request handlers with no visible timeout/AbortController — a slow upstream (TMHCC/Stripe) hangs the worker and cascades" \
        "Wrap every outbound call in AbortController with a budget (e.g. 5s); fail fast; add a circuit breaker on repeated upstream failure" \
        "Inject a 30s-slow upstream stub: the request must fail at the timeout, not hang; worker pool stays available for other requests."
fi

# ─────────── runtime: local filesystem write (breaks stateless/serverless) ───────────
fsw=$(grep -rEn "writeFile|createWriteStream|os\.tmpdir|/app/tmp|/tmp/" $INCL_TS . 2>/dev/null \
    | grep -E '/api/|route\.|lib/|utils/' | grep -vE 'node_modules|/\.next/|test-reports' | head -30)
fsw_n=$(printf '%s\n' "$fsw" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$fsw_n" -gt 0 ]; then
    sample=$(printf '%s\n' "$fsw" | head -3 | sed 's/:.*//' | sort -u | paste -sd, -)
    add runtime local-fs-write medium "$sample" \
        "$fsw_n local-disk write(s) on a request/serverless path — state is lost between invocations and not shared across instances" \
        "Write to object storage (S3/GCS) and stream from there; treat local disk as ephemeral scratch only" \
        "Deploy 2+ instances behind the LB; an artifact created on instance A is retrievable via instance B; redeploy mid-flight loses nothing."
fi

# ─────────── cache: no cache layer despite many read endpoints ───────────
cache_dep=$(grep -qiE 'ioredis|"redis"|node-redis|go-redis' package.json go.mod 2>/dev/null && echo yes || echo no)
api_handlers=$(grep -rlE 'export (async )?function (GET|POST)' $INCL_TS . 2>/dev/null | grep -E '/api/' | grep -vE 'node_modules|/\.next/' | wc -l | tr -d ' ')
if [ "$cache_dep" = "no" ] && [ "$api_handlers" -gt 20 ]; then
    add cache no-cache-layer medium "(repo)" \
        "No cache layer detected (no redis) but $api_handlers API handlers — every read hits the DB; repeated quote/pricing/lookup reads aren't memoized" \
        "Add Redis (or Vercel KV / Prisma Accelerate) for hot reads: pricing tables, TMHCC tokens, session lookups; set TTLs per data volatility" \
        "Measure DB QPS before/after under load: origin QPS drops, cache hit ratio > 70% on hot keys, p95 improves; cache-miss path still correct."
fi

# ─────────── db: serverless connection pooling ───────────
if grep -qiE 'provider *= *"postgresql"|postgresql://' prisma/schema.prisma .env.example 2>/dev/null; then
    if [ -f vercel.json ] && ! grep -qiE 'pgbouncer|connection_limit|accelerate|prisma://|DataProxy|pooler' prisma/schema.prisma .env.example 2>/dev/null; then
        add db serverless-conn-pool high "prisma/schema.prisma" \
            "Postgres + Prisma on Vercel serverless with no visible pooler (pgbouncer/Accelerate/connection_limit) — each cold function opens its own connection; bursts exhaust the DB connection cap" \
            "Front Postgres with a pooler (pgbouncer in transaction mode, Supabase pooler, or Prisma Accelerate); set connection_limit=1 per function" \
            "Load-test at concurrency >> DB max_connections: no 'too many clients' errors; active connections stay bounded at the pooler, not the function count."
    fi
fi

# ─────────── JSON output ───────────
c_high=0; c_med=0; c_low=0
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "findings": ['
first=1
while IFS=$'\t' read -r dim kind sev path detail fix validate; do
    [ -z "$dim" ] && continue
    case "$sev" in high) c_high=$((c_high+1));; medium) c_med=$((c_med+1));; low) c_low=$((c_low+1));; esac
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"dimension":"%s","kind":"%s","severity":"%s","path":"%s","detail":"%s","fix":"%s","validate":"%s"}' \
        "$(json_str "$dim")" "$(json_str "$kind")" "$(json_str "$sev")" "$(json_str "$path")" \
        "$(json_str "$detail")" "$(json_str "$fix")" "$(json_str "$validate")"
done <<< "$FINDINGS"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf '],\n'
printf '  "summary": {"high": %d, "medium": %d, "low": %d, "total": %d}\n' \
    "$c_high" "$c_med" "$c_low" "$((c_high+c_med+c_low))"
printf '}\n'
