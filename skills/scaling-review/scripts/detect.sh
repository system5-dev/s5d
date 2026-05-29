#!/usr/bin/env bash
# detect.sh — which scalability/performance dimensions apply to this repo. Read-only.
# Output: JSON.
#
# Dimensions detected:
#   orm        — Prisma / TypeORM / Sequelize / SQLAlchemy / GORM / ActiveRecord
#   db         — a relational DB is in play (schema files, DATABASE_URL usage)
#   api        — HTTP handlers present (Next route.ts, *_api, Go http, FastAPI/Flask)
#   cache      — a cache layer dep/config (redis / ioredis / memcached / CDN config)
#   queue      — background jobs / message queue (bull(mq) / sqs / kafka / celery / rabbitmq)
#   realtime   — websocket / SSE endpoints
#   runtime    — serverless (vercel/lambda) vs long-running server (affects statelessness rules)
#   heavy-libs — in-request heavy work libs (puppeteer / sharp / pdfkit) that belong off the hot path

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"

json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }
has() { [ -e "$1" ]; }

# grep a dependency name in common dependency manifests.
dep_seen() {
    grep -qiE "$1" package.json go.mod pyproject.toml requirements.txt Gemfile 2>/dev/null
}

prune_find() {
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
        -o -name build -o -name dist -o -name .next -o -name .venv -o -name __pycache__ \
        -o -name .claude -o -name .codex -o -name .agents -o -name test-reports \
        -o -name .playwright-browsers \) -prune \
        -o -type f -name "$1" -print 2>/dev/null | sed 's|^\./||' | head -n "${2:-500}"
}

ENTRIES=""
add() { ENTRIES="${ENTRIES}${1}	${2}	${3}"$'\n'; }   # dimension present evidence_csv

# --- orm ---
orm_ev=""
has prisma/schema.prisma && orm_ev="${orm_ev:+$orm_ev,}prisma"
[ "$(prune_find 'schema.prisma' 1 | wc -l | tr -d ' ')" -gt 0 ] && orm_ev="${orm_ev:+$orm_ev,}prisma"
dep_seen 'typeorm' && orm_ev="${orm_ev:+$orm_ev,}typeorm"
dep_seen 'sequelize' && orm_ev="${orm_ev:+$orm_ev,}sequelize"
dep_seen 'sqlalchemy' && orm_ev="${orm_ev:+$orm_ev,}sqlalchemy"
dep_seen 'gorm.io|jinzhu/gorm' && orm_ev="${orm_ev:+$orm_ev,}gorm"
orm_ev="$(printf '%s' "$orm_ev" | tr ',' '\n' | sed '/^$/d' | sort -u | paste -sd, -)"
[ -n "$orm_ev" ] && add orm yes "$orm_ev" || add orm no ""

# --- db ---
db_ev=""
[ -n "$orm_ev" ] && db_ev="via-orm"
grep -qiE 'DATABASE_URL|POSTGRES|MYSQL|pg\.|psycopg' .env.example package.json 2>/dev/null && db_ev="${db_ev:+$db_ev,}conn-string"
[ -n "$db_ev" ] && add db yes "$db_ev" || add db no ""

# --- api ---
api_n=0
api_n=$(( api_n + $(prune_find 'route.ts' 2000 | grep -cE '/api/' || true) ))
api_n=$(( api_n + $(prune_find 'route.js' 2000 | grep -cE '/api/' || true) ))
go_handlers=$(grep -rlE 'http\.(HandleFunc|Handle)|chi\.|gin\.|echo\.' --include='*.go' . 2>/dev/null | grep -v vendor | wc -l | tr -d ' ')
py_api=$(grep -rlE '@(app|router)\.(get|post|put|delete)|@(app)\.route' --include='*.py' . 2>/dev/null | wc -l | tr -d ' ')
total_api=$(( api_n + go_handlers + py_api ))
[ "$total_api" -gt 0 ] && add api yes "${total_api} handlers" || add api no ""

# --- cache ---
cache_ev=""
dep_seen 'ioredis|"redis"|node-redis' && cache_ev="${cache_ev:+$cache_ev,}redis"
dep_seen 'memcached' && cache_ev="${cache_ev:+$cache_ev,}memcached"
dep_seen 'go-redis|redis/go-redis' && cache_ev="${cache_ev:+$cache_ev,}go-redis"
[ -n "$cache_ev" ] && add cache yes "$cache_ev" || add cache no "none-detected"

# --- queue ---
q_ev=""
dep_seen 'bullmq|"bull"' && q_ev="${q_ev:+$q_ev,}bull"
dep_seen 'aws-sdk.*sqs|@aws-sdk/client-sqs' && q_ev="${q_ev:+$q_ev,}sqs"
dep_seen 'kafkajs|segmentio/kafka|sarama' && q_ev="${q_ev:+$q_ev,}kafka"
dep_seen 'celery' && q_ev="${q_ev:+$q_ev,}celery"
dep_seen 'amqplib|rabbitmq|streadway/amqp' && q_ev="${q_ev:+$q_ev,}rabbitmq"
# cron-as-queue: vercel crons / *.cron
grep -qiE '"crons"' vercel.json 2>/dev/null && q_ev="${q_ev:+$q_ev,}vercel-cron"
[ -n "$q_ev" ] && add queue yes "$q_ev" || add queue no "none-detected"

# --- realtime ---
rt_ev=""
dep_seen 'socket\.io|ws"|websocket' && rt_ev="ws"
grep -rqiE 'text/event-stream|EventSource' --include='*.ts' --include='*.js' . 2>/dev/null && rt_ev="${rt_ev:+$rt_ev,}sse"
[ -n "$rt_ev" ] && add realtime yes "$rt_ev" || add realtime no ""

# --- runtime ---
rt2=""
has vercel.json && rt2="serverless:vercel"
grep -qiE 'lambda|serverless' package.json serverless.yml 2>/dev/null && rt2="${rt2:+$rt2,}serverless"
[ -n "$(prune_find 'Dockerfile*' 1)" ] && rt2="${rt2:+$rt2,}container:long-running"
[ -n "$rt2" ] && add runtime yes "$rt2" || add runtime no ""

# --- heavy-libs ---
hv=""
dep_seen 'puppeteer|playwright-core' && hv="${hv:+$hv,}puppeteer"
dep_seen '"sharp"' && hv="${hv:+$hv,}sharp"
dep_seen 'pdfkit|pdf-lib|@react-pdf' && hv="${hv:+$hv,}pdf"
[ -n "$hv" ] && add heavy-libs yes "$hv" || add heavy-libs no ""

# --- JSON output ---
present_count=0
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "dimensions": ['
first=1
while IFS=$'\t' read -r dim present ev; do
    [ -z "$dim" ] && continue
    [ "$present" = "yes" ] && present_count=$((present_count+1))
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","present":%s,"evidence":[' "$dim" \
        "$([ "$present" = "yes" ] && echo "true" || echo "false")"
    lf=1
    if [ -n "$ev" ]; then
        IFS=',' read -ra arr <<< "$ev"
        for e in "${arr[@]}"; do
            [ -z "$e" ] && continue
            if [ "$lf" -eq 1 ]; then lf=0; else printf ','; fi
            printf '"%s"' "$(json_str "$e")"
        done
    fi
    printf ']}'
done <<< "$ENTRIES"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf '],\n'
printf '  "summary": {"present": %d, "dimensions_total": 8}\n' "$present_count"
printf '}\n'
