#!/usr/bin/env bash
# analyze.sh — operational posture findings across infra surfaces. Read-only.
# Output: JSON { root, findings:[{surface,kind,severity,path,detail}], summary }.
#
# This is NOT a security scanner (that's security-scan, which runs trivy/checkov for
# CVEs and misconfig). infra-scan reads the SAME files for OPERATIONAL posture:
#   - topology: how many distinct deploy planes, are they consistent
#   - containers: pinned base, multistage, non-root, healthcheck, .dockerignore
#   - helm/k8s: resource requests+limits, liveness/readiness probes, replicas
#   - ci-cd: does deploy gate on tests, how are secrets injected
#   - env: declared (.env.example) vs used (process.env / os.getenv) drift
#
# Severities: high (operational risk / drift that breaks prod), medium (missing
# guardrail), low (best-practice), info (topology note).

set -uo pipefail
ROOT="${1:-$(pwd)}"
cd "$ROOT"

json_str() { local s="$1"; s="${s//\\/\\\\}"; s="${s//\"/\\\"}"; printf '%s' "$s"; }

FINDINGS=""
add() {  # 1=surface 2=kind 3=severity 4=path 5=detail
    FINDINGS="${FINDINGS}${1}	${2}	${3}	${4}	${5}"$'\n'
}

prune_find() {  # $1=name $2=limit
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
        -o -name build -o -name dist -o -name .next -o -name .venv -o -name __pycache__ \
        -o -name .claude -o -name .codex -o -name .agents -o -name test-reports \
        -o -name .playwright-browsers \) -prune \
        -o -type f -name "$1" -print 2>/dev/null | sed 's|^\./||' | head -n "${2:-50}"
}

# ───────────────────────── containers ─────────────────────────
dockerfiles="$(prune_find 'Dockerfile*' 20; prune_find 'Containerfile' 20)"
df_count=0
while IFS= read -r df; do
    [ -z "$df" ] && continue
    df_count=$((df_count+1))
    # base image pinned? (no :latest, no bare FROM image without tag)
    if grep -qiE '^FROM[[:space:]]+[^[:space:]]+:latest' "$df" 2>/dev/null; then
        add containers unpinned-base high "$df" "base image uses :latest — non-reproducible builds"
    elif grep -qiE '^FROM[[:space:]]+[^:@[:space:]]+([[:space:]]|$)' "$df" 2>/dev/null; then
        add containers unpinned-base medium "$df" "base image has no tag — defaults to :latest"
    fi
    # multistage?
    from_n="$(grep -ciE '^FROM ' "$df" 2>/dev/null || echo 0)"
    [ "$from_n" -lt 2 ] && add containers no-multistage low "$df" "single-stage build (from=$from_n) — image likely ships build deps"
    # non-root USER?
    grep -qiE '^USER[[:space:]]+' "$df" 2>/dev/null || \
        add containers runs-as-root medium "$df" "no USER directive — container runs as root"
    # healthcheck?
    grep -qiE '^HEALTHCHECK' "$df" 2>/dev/null || \
        add containers no-healthcheck low "$df" "no HEALTHCHECK — orchestrator can't detect a wedged process"
done <<< "$dockerfiles"
# .dockerignore present where Dockerfiles exist?
if [ "$df_count" -gt 0 ] && [ ! -f .dockerignore ]; then
    for d in $(printf '%s\n' "$dockerfiles" | sed '/^$/d' | xargs -n1 dirname 2>/dev/null | sort -u); do
        [ -f "$d/.dockerignore" ] || add containers no-dockerignore low "$d" "no .dockerignore — build context may leak secrets/bloat"
    done
fi

# ───────────────────────── helm / k8s ─────────────────────────
values_files="$(prune_find 'values.yaml' 10; prune_find 'values.yml' 10)"
while IFS= read -r vf; do
    [ -z "$vf" ] && continue
    grep -qE '(requests|limits):' "$vf" 2>/dev/null || \
        add helm no-resources high "$vf" "no resource requests/limits — pod can be OOM-killed or starve neighbours"
    grep -qiE 'livenessProbe|readinessProbe' "$vf" 2>/dev/null || \
        add helm no-probes medium "$vf" "no liveness/readiness probe in values — k8s can't gate traffic on health"
    grep -qiE 'replicaCount|replicas' "$vf" 2>/dev/null || \
        add helm no-replicas low "$vf" "no replicaCount — defaults to 1, no HA"
done <<< "$values_files"

# ───────────────────────── ci-cd ─────────────────────────
ci_files=""
[ -d .github/workflows ] && ci_files="$(find .github/workflows -type f \( -name '*.yml' -o -name '*.yaml' \) 2>/dev/null | sed 's|^\./||')"
gitlab_files="$(prune_find '.gitlab-ci.yml' 10)"
[ -f .gitlab-ci.yml ] && gitlab_files=".gitlab-ci.yml"$'\n'"$gitlab_files"
all_ci="$(printf '%s\n%s\n' "$ci_files" "$gitlab_files" | sed '/^$/d' | sort -u)"
ci_planes=0
while IFS= read -r cf; do
    [ -z "$cf" ] && continue
    ci_planes=$((ci_planes+1))
    # does it deploy?
    if grep -qiE '(deploy|kubectl|helm |vercel|fly deploy|gitlab.*pages|environment:)' "$cf" 2>/dev/null; then
        # does deploy gate on tests in the same file?
        grep -qiE '(test|jest|pytest|go test|npm test|playwright)' "$cf" 2>/dev/null || \
            add ci-cd deploy-without-test-gate high "$cf" "pipeline deploys but has no visible test stage — untested code can ship"
    fi
done <<< "$all_ci"

# ───────────────────────── topology / fragmentation ─────────────────────────
planes=""
[ -f vercel.json ] && planes="${planes}vercel "
[ -n "$gitlab_files" ] && planes="${planes}gitlab-ci "
[ -n "$values_files" ] && planes="${planes}helm "
[ "$df_count" -gt 0 ] && planes="${planes}docker "
plane_n="$(printf '%s' "$planes" | wc -w | tr -d ' ')"
if [ "$plane_n" -ge 2 ]; then
    add topology split-deploy-planes high "(repo)" "$plane_n deploy systems coexist: $planes — no single source of deploy truth; drift risk across them"
fi

# ───────────────────────── env / secrets flow ─────────────────────────
env_decl=""
for f in .env.example .env.sample .env.template; do
    [ -f "$f" ] && env_decl="$f" && break
done
if [ -n "$env_decl" ]; then
    # declared keys (LHS of =)
    decl_keys="$(grep -vE '^[[:space:]]*#' "$env_decl" 2>/dev/null | grep -E '=' | sed -E 's/[[:space:]]*=.*//' | sed 's/^export //' | sed '/^$/d' | sort -u)"
    decl_n="$(printf '%s\n' "$decl_keys" | sed '/^$/d' | wc -l | tr -d ' ')"
    # keys USED in code: process.env.X / process.env['X'] / os.getenv("X")
    used_keys="$(grep -rhoE "process\.env\.[A-Z0-9_]+|process\.env\[['\"][A-Z0-9_]+['\"]\]|os\.getenv\(['\"][A-Z0-9_]+['\"]\)|os\.Getenv\(\"[A-Z0-9_]+\"\)" . \
        --include='*.ts' --include='*.tsx' --include='*.js' --include='*.jsx' --include='*.py' --include='*.go' 2>/dev/null \
        | grep -vE 'node_modules|/\.next/|test-reports' \
        | sed -E "s/.*[\.\(\[]['\"]?([A-Z0-9_]+)['\"]?\]?\)?$/\1/" | sort -u)"
    # used-but-undeclared = drift (prod will crash on missing env)
    undeclared="$(comm -13 <(printf '%s\n' "$decl_keys") <(printf '%s\n' "$used_keys") 2>/dev/null | sed '/^$/d')"
    und_n="$(printf '%s\n' "$undeclared" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [ "$und_n" -gt 0 ]; then
        sample="$(printf '%s\n' "$undeclared" | head -8 | paste -sd, - | sed 's/,$//')"
        add env undeclared-env-vars high "$env_decl" "$und_n env vars used in code but absent from $env_decl (e.g. $sample) — undocumented prod config"
    fi
    add env declared-surface info "$env_decl" "$decl_n env vars declared in $env_decl"
fi

# ───────────────────────── JSON output ─────────────────────────
c_high=0; c_med=0; c_low=0; c_info=0
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "findings": ['
first=1
while IFS=$'\t' read -r surface kind sev path detail; do
    [ -z "$surface" ] && continue
    case "$sev" in high) c_high=$((c_high+1));; medium) c_med=$((c_med+1));; low) c_low=$((c_low+1));; info) c_info=$((c_info+1));; esac
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"surface":"%s","kind":"%s","severity":"%s","path":"%s","detail":"%s"}' \
        "$(json_str "$surface")" "$(json_str "$kind")" "$(json_str "$sev")" \
        "$(json_str "$path")" "$(json_str "$detail")"
done <<< "$FINDINGS"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf '],\n'
printf '  "summary": {"high": %d, "medium": %d, "low": %d, "info": %d, "total": %d}\n' \
    "$c_high" "$c_med" "$c_low" "$c_info" "$((c_high+c_med+c_low+c_info))"
printf '}\n'
