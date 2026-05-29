#!/usr/bin/env bash
# detect.sh — deterministic stack detection.
# Emits JSON: {"root": "...", "stacks": [{"id": "js", "evidence": ["package.json"]}, ...],
#              "package_managers": {"js": "bun"}, "notes": ["..."]}
# No external dependencies (jq, python, etc.). Pure bash + POSIX utilities.

set -euo pipefail

ROOT="${1:-$(pwd)}"
cd "$ROOT"

# -- helpers --------------------------------------------------------------------

found_stacks=()
stack_evidence=()   # parallel array: "stackId|file1,file2,..."
package_managers=()
notes=()

add_stack() {
    local id="$1"; shift
    local evidence="$*"
    # Skip duplicates. Guard for empty array under set -u.
    if [ "${#found_stacks[@]}" -gt 0 ]; then
        for existing in "${found_stacks[@]}"; do
            [ "$existing" = "$id" ] && return 0
        done
    fi
    found_stacks+=("$id")
    stack_evidence+=("$id|$evidence")
}

add_pm() {
    local stack="$1"; local pm="$2"
    package_managers+=("$stack|$pm")
}

has() { [ -e "$1" ]; }

# Count files matching a glob in the tree, capped at LIMIT for speed.
# Excludes node_modules, .git, vendor, target, build, dist, .next, .venv.
count_files() {
    local pattern="$1"
    local limit="${2:-1}"
    # shellcheck disable=SC2046
    find . \
        -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
                   -o -name build -o -name dist -o -name .next -o -name .venv \
                   -o -name __pycache__ -o -name .gradle -o -name .mvn \
                   -o -name .claude -o -name .codex -o -name .codex-plugin \
                   -o -name .claude-plugin -o -name .s5d -o -name .agents \) -prune \
        -o -type f -name "$pattern" -print 2>/dev/null | head -n "$limit" | wc -l | tr -d ' '
}

# -- stack rules ----------------------------------------------------------------

# JS / TS
if has package.json; then
    ev="package.json"
    has tsconfig.json && ev="$ev,tsconfig.json"
    has jsconfig.json && ev="$ev,jsconfig.json"
    add_stack "js" "$ev"
    # Detect package manager
    if has bun.lockb || has bun.lock; then add_pm js bun
    elif has pnpm-lock.yaml; then add_pm js pnpm
    elif has yarn.lock; then add_pm js yarn
    elif has package-lock.json; then add_pm js npm
    else add_pm js npm
    fi
    has tsconfig.json && add_stack "ts" "tsconfig.json"
fi

# Python
if has pyproject.toml || has setup.py || has requirements.txt || has Pipfile; then
    ev=""
    has pyproject.toml && ev="${ev:+$ev,}pyproject.toml"
    has setup.py && ev="${ev:+$ev,}setup.py"
    has requirements.txt && ev="${ev:+$ev,}requirements.txt"
    has Pipfile && ev="${ev:+$ev,}Pipfile"
    add_stack "python" "$ev"
    if has uv.lock; then add_pm python uv
    elif has poetry.lock; then add_pm python poetry
    elif has Pipfile.lock; then add_pm python pipenv
    elif has requirements.txt; then add_pm python pip
    else add_pm python uv
    fi
fi

# Go
if has go.mod; then add_stack "go" "go.mod"; add_pm go "go modules"; fi

# Rust
if has Cargo.toml; then
    ev="Cargo.toml"
    has Cargo.lock && ev="$ev,Cargo.lock"
    add_stack "rust" "$ev"
    add_pm rust cargo
fi

# Java
if has pom.xml; then
    add_stack "java" "pom.xml"
    add_pm java maven
elif has build.gradle || has build.gradle.kts; then
    ev=""
    has build.gradle && ev="${ev:+$ev,}build.gradle"
    has build.gradle.kts && ev="${ev:+$ev,}build.gradle.kts"
    # Could be Kotlin or Java; check sources
    if [ "$(count_files '*.java' 1)" -gt 0 ]; then
        add_stack "java" "$ev"
        add_pm java gradle
    fi
fi

# Kotlin
if [ "$(count_files '*.kt' 1)" -gt 0 ] || has build.gradle.kts; then
    ev=""
    [ "$(count_files '*.kt' 1)" -gt 0 ] && ev="${ev:+$ev,}*.kt files"
    has build.gradle.kts && ev="${ev:+$ev,}build.gradle.kts"
    add_stack "kotlin" "$ev"
    add_pm kotlin gradle
fi

# .NET
if [ "$(count_files '*.csproj' 1)" -gt 0 ] || [ "$(count_files '*.fsproj' 1)" -gt 0 ] || \
   [ "$(count_files '*.vbproj' 1)" -gt 0 ] || [ "$(count_files '*.sln' 1)" -gt 0 ]; then
    ev=""
    [ "$(count_files '*.csproj' 1)" -gt 0 ] && ev="${ev:+$ev,}*.csproj"
    [ "$(count_files '*.fsproj' 1)" -gt 0 ] && ev="${ev:+$ev,}*.fsproj"
    [ "$(count_files '*.sln' 1)" -gt 0 ] && ev="${ev:+$ev,}*.sln"
    add_stack "dotnet" "$ev"
    add_pm dotnet "dotnet sdk"
fi

# Ruby
if has Gemfile || [ "$(count_files '*.gemspec' 1)" -gt 0 ]; then
    ev=""
    has Gemfile && ev="${ev:+$ev,}Gemfile"
    [ "$(count_files '*.gemspec' 1)" -gt 0 ] && ev="${ev:+$ev,}*.gemspec"
    add_stack "ruby" "$ev"
    add_pm ruby bundler
fi

# PHP
if has composer.json; then add_stack "php" "composer.json"; add_pm php composer; fi

# Swift
if has Package.swift || [ "$(count_files '*.xcodeproj' 1)" -gt 0 ]; then
    ev=""
    has Package.swift && ev="${ev:+$ev,}Package.swift"
    [ "$(count_files '*.xcodeproj' 1)" -gt 0 ] && ev="${ev:+$ev,}*.xcodeproj"
    add_stack "swift" "$ev"
    add_pm swift spm
fi

# Shell — only flag if there are non-trivial scripts (>=1 .sh file outside vendor)
sh_count=$(count_files '*.sh' 5)
if [ "$sh_count" -ge 1 ]; then add_stack "shell" "$sh_count+ .sh files"; fi

# Docker
docker_ev=""
has Dockerfile && docker_ev="${docker_ev:+$docker_ev,}Dockerfile"
has docker-compose.yml && docker_ev="${docker_ev:+$docker_ev,}docker-compose.yml"
has docker-compose.yaml && docker_ev="${docker_ev:+$docker_ev,}docker-compose.yaml"
has compose.yml && docker_ev="${docker_ev:+$docker_ev,}compose.yml"
has compose.yaml && docker_ev="${docker_ev:+$docker_ev,}compose.yaml"
if [ -z "$docker_ev" ] && [ "$(count_files 'Dockerfile*' 1)" -gt 0 ]; then
    docker_ev="Dockerfile* variants"
fi
[ -n "$docker_ev" ] && add_stack "docker" "$docker_ev"

# Terraform
if [ "$(count_files '*.tf' 1)" -gt 0 ]; then add_stack "terraform" "*.tf files"; fi

# YAML — almost any project has yaml, only flag if user explicitly wants linting.
# Default: skip unless YAML is heavy (>= 5 files outside CI configs).
yml_count=$(count_files '*.yml' 10)
yaml_count=$(count_files '*.yaml' 10)
total_yaml=$((yml_count + yaml_count))
if [ "$total_yaml" -ge 5 ]; then add_stack "yaml" "$total_yaml+ yaml files"; fi

# Markdown — same logic, only flag if docs-heavy
md_count=$(count_files '*.md' 10)
if [ "$md_count" -ge 5 ]; then add_stack "markdown" "$md_count+ .md files"; fi

# SQL
sql_count=$(count_files '*.sql' 3)
if [ "$sql_count" -ge 1 ]; then add_stack "sql" "$sql_count+ .sql files"; fi

# -- JSON output ----------------------------------------------------------------

# Escape backslashes and quotes for JSON.
json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

emit_stacks() {
    local first=1
    local entry id ev_csv evfiles
    [ "${#stack_evidence[@]}" -eq 0 ] && return 0
    for entry in "${stack_evidence[@]}"; do
        id="${entry%%|*}"
        ev_csv="${entry#*|}"
        if [ "$first" -eq 1 ]; then first=0; else printf ','; fi
        printf '\n    {"id":"%s","evidence":[' "$(json_str "$id")"
        local subfirst=1
        local evfiles=()
        if [ -n "$ev_csv" ]; then
            IFS=',' read -ra evfiles <<< "$ev_csv"
        fi
        if [ "${#evfiles[@]}" -gt 0 ]; then
            for f in "${evfiles[@]}"; do
                if [ "$subfirst" -eq 1 ]; then subfirst=0; else printf ','; fi
                printf '"%s"' "$(json_str "$f")"
            done
        fi
        printf ']}'
    done
}

emit_pms() {
    local first=1
    local entry stack pm
    [ "${#package_managers[@]}" -eq 0 ] && return 0
    for entry in "${package_managers[@]}"; do
        stack="${entry%%|*}"
        pm="${entry#*|}"
        if [ "$first" -eq 1 ]; then first=0; else printf ','; fi
        printf '\n    "%s":"%s"' "$(json_str "$stack")" "$(json_str "$pm")"
    done
}

printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "stacks": ['
emit_stacks
printf '\n  ],\n'
printf '  "package_managers": {'
emit_pms
printf '\n  }\n'
printf '}\n'
