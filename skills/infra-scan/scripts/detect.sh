#!/usr/bin/env bash
# detect.sh — figure out which infrastructure / deployment surfaces this repo has.
# Output: JSON. Read-only — never writes.
#
# Surfaces detected:
#   containers   — Dockerfile* / Containerfile / .docker/
#   compose      — docker-compose* / compose*.y(a)ml
#   helm         — Chart.yaml / values.yaml
#   k8s          — manifests with `kind: Deployment|Service|Ingress|...`
#   terraform    — *.tf / *.tfvars
#   ci-cd        — .github/workflows/* / .gitlab-ci.yml / Jenkinsfile / .circleci / azure-pipelines
#   paas         — vercel.json / netlify.toml / fly.toml / render.yaml / app.yaml / Procfile / railway*
#   env          — .env.example / .env.sample / .env.template (declared env surface)
#
# Unlike security-scan (which scans these for CVEs/misconfig), infra-scan reads them
# for OPERATIONAL posture: topology, drift, env/secrets flow, resource/probe sizing.

set -uo pipefail

ROOT="${1:-$(pwd)}"
cd "$ROOT"

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

# find with the cluster's standard prune set; prints matching file paths.
find_files() {
    local name="$1"
    local limit="${2:-50}"
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
                       -o -name build -o -name dist -o -name .next -o -name .venv \
                       -o -name __pycache__ -o -name .gradle -o -name .mvn \
                       -o -name .claude -o -name .codex -o -name .codex-plugin \
                       -o -name .claude-plugin -o -name .agents \
                       -o -name .playwright-browsers -o -name test-results \
                       -o -name test-reports \) -prune \
        -o -type f -name "$name" -print 2>/dev/null | sed 's|^\./||' | head -n "$limit"
}

ENTRIES=""
add() {
    # 1=surface 2=present 3=evidence_csv(paths or labels)
    ENTRIES="${ENTRIES}${1}	${2}	${3}"$'\n'
}

join_csv() {
    # stdin: lines → csv (cap already applied by caller)
    paste -sd, - 2>/dev/null | sed 's/,$//'
}

# --- containers ---
docker_paths="$(find_files 'Dockerfile*' 20; find_files 'Containerfile' 20)"
docker_csv="$(printf '%s\n' "$docker_paths" | sed '/^$/d' | join_csv)"
[ -n "$docker_csv" ] && add containers yes "$docker_csv" || add containers no ""

# --- compose ---
compose_paths="$(find_files 'docker-compose*.y*ml' 10; find_files 'compose*.y*ml' 10)"
compose_csv="$(printf '%s\n' "$compose_paths" | sed '/^$/d' | sort -u | join_csv)"
[ -n "$compose_csv" ] && add compose yes "$compose_csv" || add compose no ""

# --- helm ---
helm_paths="$(find_files 'Chart.yaml' 10)"
helm_csv="$(printf '%s\n' "$helm_paths" | sed '/^$/d' | join_csv)"
[ -n "$helm_csv" ] && add helm yes "$helm_csv" || add helm no ""

# --- k8s manifests ---
k8s_csv=""
k8s_hit="$(grep -lE '^kind:[[:space:]]*(Deployment|Service|Ingress|StatefulSet|DaemonSet|ConfigMap|Secret|CronJob|Job)' \
    --include='*.yaml' --include='*.yml' -r . 2>/dev/null \
    | grep -vE 'node_modules|/\.git/|/vendor/|/\.next/|test-reports|/\.helm/' | sed 's|^\./||' | head -n 15)"
k8s_csv="$(printf '%s\n' "$k8s_hit" | sed '/^$/d' | join_csv)"
[ -n "$k8s_csv" ] && add k8s yes "$k8s_csv" || add k8s no ""

# --- terraform ---
tf_paths="$(find_files '*.tf' 20)"
tf_csv="$(printf '%s\n' "$tf_paths" | sed '/^$/d' | join_csv)"
[ -n "$tf_csv" ] && add terraform yes "$tf_csv" || add terraform no ""

# --- ci-cd ---
ci_csv=""
ci_lines=""
[ -d .github/workflows ] && ci_lines="${ci_lines}$(find .github/workflows -type f \( -name '*.yml' -o -name '*.yaml' \) 2>/dev/null | sed 's|^\./||' | head -n 15)"$'\n'
[ -f .gitlab-ci.yml ] && ci_lines="${ci_lines}.gitlab-ci.yml"$'\n'
for g in $(find_files '.gitlab-ci.yml' 10); do ci_lines="${ci_lines}${g}"$'\n'; done
[ -f Jenkinsfile ] && ci_lines="${ci_lines}Jenkinsfile"$'\n'
[ -d .circleci ] && ci_lines="${ci_lines}.circleci/config.yml"$'\n'
[ -f azure-pipelines.yml ] && ci_lines="${ci_lines}azure-pipelines.yml"$'\n'
ci_csv="$(printf '%s\n' "$ci_lines" | sed '/^$/d' | sort -u | join_csv)"
[ -n "$ci_csv" ] && add ci-cd yes "$ci_csv" || add ci-cd no ""

# --- paas (platform-as-a-service runtime configs) ---
paas_lines=""
for f in vercel.json netlify.toml fly.toml render.yaml app.yaml Procfile railway.json railway.toml .platform.app.yaml apprunner.yaml; do
    [ -f "$f" ] && paas_lines="${paas_lines}${f}"$'\n'
done
paas_csv="$(printf '%s\n' "$paas_lines" | sed '/^$/d' | join_csv)"
[ -n "$paas_csv" ] && add paas yes "$paas_csv" || add paas no ""

# --- env surface (declared) ---
env_lines=""
for f in .env.example .env.sample .env.template .env.dist env.example; do
    [ -f "$f" ] && env_lines="${env_lines}${f}"$'\n'
done
for e in $(find_files '.env.example' 10; find_files '.env.sample' 10); do env_lines="${env_lines}${e}"$'\n'; done
env_csv="$(printf '%s\n' "$env_lines" | sed '/^$/d' | sort -u | join_csv)"
[ -n "$env_csv" ] && add env yes "$env_csv" || add env no ""

# --- JSON output ---
present_count=0
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "surfaces": ['
first=1
while IFS=$'\t' read -r surface present ev; do
    [ -z "$surface" ] && continue
    [ "$present" = "yes" ] && present_count=$((present_count+1))
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","present":%s,"evidence":[' "$surface" \
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
printf '  "summary": {"present": %d, "surfaces_total": 8}\n' "$present_count"
printf '}\n'
