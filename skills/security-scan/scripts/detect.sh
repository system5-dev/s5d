#!/usr/bin/env bash
# detect.sh — figure out which security-scan categories apply to this repo.
# Output: JSON.
#
# Categories detected:
#   sast       — always (Semgrep is polyglot)
#   sca        — per stack: package.json / pyproject.toml / Cargo.toml / go.mod /
#                Gemfile / composer.json / *.csproj / build.gradle*
#   secrets    — always (we're in a git repo); pulls .git history
#   container  — Dockerfile* / docker-compose* / compose.*.yml present
#   iac        — *.tf / k8s manifests / helm charts
#   license    — same triggers as sca (license scanned via Trivy alongside CVE)
#   sbom       — same triggers as sca (SBOM emitted via Syft from same lockfiles)

set -uo pipefail

ROOT="${1:-$(pwd)}"
cd "$ROOT"

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

has() { [ -e "$1" ]; }

count_files() {
    local pattern="$1"
    local limit="${2:-1}"
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
                       -o -name build -o -name dist -o -name .next -o -name .venv \
                       -o -name __pycache__ -o -name .gradle -o -name .mvn \
                       -o -name .claude -o -name .codex -o -name .codex-plugin \
                       -o -name .claude-plugin -o -name .s5d -o -name .agents \
                       -o -name .playwright-browsers -o -name test-results \
                       -o -name test-reports \) -prune \
        -o -type f -name "$pattern" -print 2>/dev/null | head -n "$limit" | wc -l | tr -d ' '
}

ENTRIES=""
add() {
    # 1=category 2=enabled 3=evidence_csv 4=tool
    ENTRIES="${ENTRIES}${1}	${2}	${3}	${4}"$'\n'
}

# --- sast (always, but record what languages we'll target) ---
sast_evidence=""
has package.json    && sast_evidence="${sast_evidence:+$sast_evidence,}js/ts"
has pyproject.toml || has setup.py || has requirements.txt && \
    sast_evidence="${sast_evidence:+$sast_evidence,}python"
has go.mod          && sast_evidence="${sast_evidence:+$sast_evidence,}go"
has Cargo.toml      && sast_evidence="${sast_evidence:+$sast_evidence,}rust"
has pom.xml || has build.gradle || has build.gradle.kts && \
    sast_evidence="${sast_evidence:+$sast_evidence,}java/kotlin"
[ "$(count_files '*.csproj' 1)" -gt 0 ] && \
    sast_evidence="${sast_evidence:+$sast_evidence,}dotnet"
has Gemfile         && sast_evidence="${sast_evidence:+$sast_evidence,}ruby"
has composer.json   && sast_evidence="${sast_evidence:+$sast_evidence,}php"
[ -z "$sast_evidence" ] && sast_evidence="generic"
add sast yes "$sast_evidence" semgrep

# --- sca (per language) ---
sca_targets=""
has package.json    && sca_targets="${sca_targets:+$sca_targets,}package.json"
has pyproject.toml  && sca_targets="${sca_targets:+$sca_targets,}pyproject.toml"
has requirements.txt && sca_targets="${sca_targets:+$sca_targets,}requirements.txt"
has go.mod          && sca_targets="${sca_targets:+$sca_targets,}go.mod"
has Cargo.toml      && sca_targets="${sca_targets:+$sca_targets,}Cargo.toml"
has Gemfile         && sca_targets="${sca_targets:+$sca_targets,}Gemfile"
has composer.json   && sca_targets="${sca_targets:+$sca_targets,}composer.json"
has pom.xml         && sca_targets="${sca_targets:+$sca_targets,}pom.xml"
has build.gradle    && sca_targets="${sca_targets:+$sca_targets,}build.gradle"
has build.gradle.kts && sca_targets="${sca_targets:+$sca_targets,}build.gradle.kts"

if [ -n "$sca_targets" ]; then
    add sca yes "$sca_targets" trivy
    add license yes "$sca_targets" trivy
    add sbom yes "$sca_targets" syft
else
    add sca no "" "—"
    add license no "" "—"
    add sbom no "" "—"
fi

# --- secrets (always when we have .git) ---
if has .git; then
    add secrets yes ".git" gitleaks
else
    add secrets no "" "—"
fi

# --- container ---
container_ev=""
has Dockerfile && container_ev="${container_ev:+$container_ev,}Dockerfile"
[ "$(count_files 'Dockerfile*' 1)" -gt 0 ] && container_ev="${container_ev:+$container_ev,}Dockerfile*"
has docker-compose.yml && container_ev="${container_ev:+$container_ev,}docker-compose.yml"
has docker-compose.yaml && container_ev="${container_ev:+$container_ev,}docker-compose.yaml"
has compose.yml && container_ev="${container_ev:+$container_ev,}compose.yml"
[ "$(count_files 'Containerfile' 1)" -gt 0 ] && container_ev="${container_ev:+$container_ev,}Containerfile"

if [ -n "$container_ev" ]; then
    add container yes "$container_ev" trivy
else
    add container no "" "—"
fi

# --- iac ---
iac_ev=""
[ "$(count_files '*.tf' 1)" -gt 0 ] && iac_ev="${iac_ev:+$iac_ev,}terraform"
[ "$(count_files '*.tfvars' 1)" -gt 0 ] && iac_ev="${iac_ev:+$iac_ev,}tfvars"
has helm/Chart.yaml || [ "$(count_files 'Chart.yaml' 1)" -gt 0 ] && \
    iac_ev="${iac_ev:+$iac_ev,}helm"
# k8s manifests: yaml files with `kind:` field — heuristic; cap search.
if grep -lE '^kind:[[:space:]]*(Deployment|Service|Ingress|StatefulSet|DaemonSet|ConfigMap|Secret)' \
    --include='*.yaml' --include='*.yml' -r . 2>/dev/null | head -1 | grep -q .; then
    iac_ev="${iac_ev:+$iac_ev,}k8s-manifests"
fi
[ "$(count_files 'cloudformation*.yaml' 1)" -gt 0 ] && iac_ev="${iac_ev:+$iac_ev,}cloudformation"

if [ -n "$iac_ev" ]; then
    add iac yes "$iac_ev" trivy
else
    add iac no "" "—"
fi

# --- JSON output ---
enabled_count=0
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "categories": ['
first=1
while IFS=$'\t' read -r cat enabled ev tool; do
    [ -z "$cat" ] && continue
    [ "$enabled" = "yes" ] && enabled_count=$((enabled_count+1))
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","enabled":%s,"evidence":[' "$cat" \
        "$([ "$enabled" = "yes" ] && echo "true" || echo "false")"
    local_first=1
    IFS=',' read -ra arr <<< "$ev"
    for e in "${arr[@]}"; do
        [ -z "$e" ] && continue
        if [ "$local_first" -eq 1 ]; then local_first=0; else printf ','; fi
        printf '"%s"' "$(json_str "$e")"
    done
    printf '],"tool":"%s"}' "$tool"
done <<< "$ENTRIES"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf '],\n'
printf '  "summary": {"enabled": %d, "categories_total": 7}\n' "$enabled_count"
printf '}\n'
