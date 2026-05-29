#!/usr/bin/env bash
# detect.sh — figure out which e2e surfaces this repo has, and which framework
# (if any) is already wired up. Outputs JSON.
#
# E2E has three axes that can coexist in one repo:
#   - web    : there's a frontend that runs in a browser (Next.js, Vite, CRA, ...)
#   - api    : there's an HTTP API (Next.js routes, FastAPI, Express, ...)
#   - mobile : RN, native iOS, or native Android
#
# Each axis is independent. A Next.js full-stack repo gets both web+api.
# A Go HTTP backend gets api only. A React Native app gets mobile+api (when
# the RN app also ships an API).

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

# Count files matching pattern. Prunes build + agent-tooling dirs.
count_files() {
    local pattern="$1"
    local limit="${2:-1}"
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
                       -o -name build -o -name dist -o -name .next -o -name .venv \
                       -o -name __pycache__ -o -name .gradle -o -name .mvn \
                       -o -name .claude -o -name .codex -o -name .codex-plugin \
                       -o -name .claude-plugin -o -name .s5d -o -name .agents \
                       -o -name .playwright-browsers \) -prune \
        -o -type f -name "$pattern" -print 2>/dev/null | head -n "$limit" | wc -l | tr -d ' '
}

has_dep() {
    local file="$1"; local pat="$2"
    [ -f "$file" ] && grep -qE "$pat" "$file"
}

# Axis entries: tab-separated, newline-separated.
ENTRIES=""
add_axis() {
    # 1=id 2=framework 3=existing_spec_count 4=evidence_csv
    ENTRIES="${ENTRIES}${1}	${2}	${3}	${4}"$'\n'
}

# --- WEB ---------------------------------------------------------------------
# Heuristic: a frontend means there's a JS framework dep AND HTML output.
web_evidence=""
web_framework="none"
web_count=0
if has package.json; then
    if has_dep package.json '"next"' ; then web_evidence="next.js"
    elif has_dep package.json '"vite"' ; then web_evidence="vite"
    elif has_dep package.json '"react-scripts"' ; then web_evidence="cra"
    elif has_dep package.json '"@angular/core"' ; then web_evidence="angular"
    elif has_dep package.json '"vue"' ; then web_evidence="vue"
    elif has_dep package.json '"svelte"' ; then web_evidence="svelte"
    elif has_dep package.json '"remix-run"\|"@remix-run/"'; then web_evidence="remix"
    fi
    if [ -n "$web_evidence" ]; then
        # Framework already installed?
        if has_dep package.json '"@playwright/test"'; then web_framework="playwright"
        elif has_dep package.json '"cypress"'; then web_framework="cypress"
        elif has_dep package.json '"webdriverio"'; then web_framework="webdriverio"
        fi
        web_count=$(( $(count_files '*.spec.ts' 999) + $(count_files '*.spec.tsx' 999) + \
                      $(count_files '*.spec.js' 999) + $(count_files '*.spec.jsx' 999) + \
                      $(count_files '*.e2e.ts' 999) + $(count_files '*.cy.ts' 999) + \
                      $(count_files '*.cy.js' 999) ))
        [ -d e2e ] && web_evidence="$web_evidence,e2e/"
        [ -d tests/e2e ] && web_evidence="$web_evidence,tests/e2e/"
        [ -d cypress ] && web_evidence="$web_evidence,cypress/"
        add_axis web "$web_framework" "$web_count" "$web_evidence"
    fi
fi

# --- API ---------------------------------------------------------------------
# Heuristic: there's an HTTP API in the repo. Look for Next.js API routes,
# FastAPI / Flask, Express / Fastify, Gin, etc.
api_evidence=""
api_framework="none"
if [ -d app/api ] || [ -d pages/api ]; then
    api_evidence="next-api"
elif has_dep pyproject.toml 'fastapi\|flask\|starlette' ; then
    api_evidence="python-api"
elif has_dep package.json '"express"\|"fastify"\|"hono"\|"koa"' ; then
    api_evidence="node-api"
elif has go.mod && grep -rqE '\bnet/http\b\|"github\.com/gin-gonic/gin"\|"github\.com/labstack/echo' --include='*.go' . 2>/dev/null; then
    api_evidence="go-api"
elif has Cargo.toml && grep -rqE '"axum"\|"actix-web"\|"warp"\|"rocket"' Cargo.toml 2>/dev/null; then
    api_evidence="rust-api"
fi
if [ -n "$api_evidence" ]; then
    # Reuse Playwright if web already has it (Playwright tests API too).
    if [ "$web_framework" = "playwright" ]; then api_framework="playwright-api"
    elif [ "$(count_files '*.hurl' 1)" -gt 0 ]; then api_framework="hurl"
    elif has_dep package.json '"supertest"'; then api_framework="supertest"
    elif has_dep pyproject.toml 'pytest\|httpx'; then api_framework="pytest-httpx"
    fi
    # Reuse the same e2e spec count as web; or pure-api repos count *.hurl files.
    api_count=$(count_files '*.hurl' 999)
    add_axis api "$api_framework" "$api_count" "$api_evidence"
fi

# --- MOBILE ------------------------------------------------------------------
mobile_evidence=""
mobile_framework="none"
mobile_count=0

# React Native
if has_dep package.json '"react-native"' || has metro.config.js || has metro.config.ts; then
    mobile_evidence="react-native"
    has_dep package.json '"detox"' && mobile_framework="detox"
    [ -d ".maestro" ] && mobile_framework="maestro"
fi

# Native iOS (xcodeproj outside node_modules + UI tests target heuristic)
ios_proj=$(count_files '*.xcodeproj' 1)
if [ "$ios_proj" -gt 0 ]; then
    [ -n "$mobile_evidence" ] && mobile_evidence="$mobile_evidence,ios" || mobile_evidence="ios"
    # XCUITest detection — UI test target naming
    if grep -rqE 'XCUIApplication\(\)' --include='*.swift' . 2>/dev/null; then
        [ "$mobile_framework" = "none" ] && mobile_framework="xcuitest"
    fi
fi

# Native Android (settings.gradle or build.gradle with android plugin)
if has settings.gradle || has settings.gradle.kts || has_dep build.gradle 'com.android' || has_dep build.gradle.kts 'com.android'; then
    [ -n "$mobile_evidence" ] && mobile_evidence="$mobile_evidence,android" || mobile_evidence="android"
    if grep -rqE 'androidx.test.espresso' --include='*.kt' --include='*.java' --include='build.gradle*' . 2>/dev/null; then
        [ "$mobile_framework" = "none" ] && mobile_framework="espresso"
    fi
fi

# Maestro overrides per-platform if .maestro/ exists
[ -d ".maestro" ] && mobile_framework="maestro"

if [ -n "$mobile_evidence" ]; then
    case "$mobile_framework" in
        maestro)   mobile_count=$(count_files '*.yaml' 999) ;;
        detox)     mobile_count=$(count_files '*.e2e.{ts,js}' 999) ;;
        xcuitest)  mobile_count=$(( $(count_files '*UITests.swift' 999) + $(count_files '*UITest.swift' 999) )) ;;
        espresso)  mobile_count=$(grep -rl 'androidx.test.espresso' --include='*.kt' --include='*.java' . 2>/dev/null | wc -l | tr -d ' ') ;;
    esac
    add_axis mobile "$mobile_framework" "$mobile_count" "$mobile_evidence"
fi

# --- s5d integration check ---------------------------------------------------
S5D_FEATURE_COUNT=0
if [ -d .s5d/scenarios ]; then
    S5D_FEATURE_COUNT=$(find .s5d/scenarios -maxdepth 1 -name '*.feature' | wc -l | tr -d ' ')
fi

# --- JSON output --------------------------------------------------------------
printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "axes": ['
first=1
while IFS=$'\t' read -r id fw count ev; do
    [ -z "$id" ] && continue
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","framework":"%s","existing_spec_count":%s,"evidence":[' \
        "$id" "$fw" "$count"
    local_first=1
    IFS=',' read -ra evarr <<< "$ev"
    for e in "${evarr[@]}"; do
        [ -z "$e" ] && continue
        if [ "$local_first" -eq 1 ]; then local_first=0; else printf ','; fi
        printf '"%s"' "$(json_str "$e")"
    done
    printf ']}'
done <<< "$ENTRIES"
if [ "$first" -eq 0 ]; then printf '\n  '; fi
printf '],\n'
printf '  "s5d": {\n'
printf '    "feature_count": %d,\n' "$S5D_FEATURE_COUNT"
printf '    "scenarios_dir": ".s5d/scenarios"\n'
printf '  }\n'
printf '}\n'
