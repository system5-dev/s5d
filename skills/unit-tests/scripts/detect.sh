#!/usr/bin/env bash
# detect.sh — identify stacks AND which test framework + coverage tool is
# already installed in each. Output: JSON.
#
# {
#   "root": "...",
#   "stacks": [
#     { "id": "js", "package_manager": "npm",
#       "test_framework": "vitest|jest|none",
#       "coverage_tool": "v8|istanbul|none",
#       "existing_test_files": 7,
#       "evidence": ["package.json", "vitest.config.ts"] },
#     ...
#   ]
# }

set -uo pipefail   # NOT -e: many absence checks return 1 normally.

ROOT="${1:-$(pwd)}"
cd "$ROOT"

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

has() { [ -e "$1" ]; }

# Count files matching a pattern, pruned tree.
# Prunes build/cache dirs AND agent-tooling dirs (.claude, .codex, .s5d) so a
# skill's own template files don't accidentally register as user code.
count_files() {
    local pattern="$1"
    local limit="${2:-1}"
    find . -type d \( -name node_modules -o -name .git -o -name vendor -o -name target \
                       -o -name build -o -name dist -o -name .next -o -name .venv \
                       -o -name __pycache__ -o -name .gradle -o -name .mvn \
                       -o -name .claude -o -name .codex -o -name .codex-plugin \
                       -o -name .claude-plugin -o -name .s5d -o -name .agents \) -prune \
        -o -type f -name "$pattern" -print 2>/dev/null | head -n "$limit" | wc -l | tr -d ' '
}

# Grep a file for the presence of a dependency (best-effort).
has_dep() {
    local file="$1"; local pat="$2"
    [ -f "$file" ] && grep -qE "$pat" "$file"
}

# Detected stack entries: each is a tab-separated string of fields. Newline-separated.
ENTRIES=""

add_entry() {
    # 1=id 2=pm 3=framework 4=coverage 5=test_count 6=evidence_csv
    ENTRIES="${ENTRIES}${1}	${2}	${3}	${4}	${5}	${6}"$'\n'
}

# --- JS / TS ------------------------------------------------------------------
if has package.json; then
    pm="npm"
    if has bun.lockb || has bun.lock; then pm="bun"
    elif has pnpm-lock.yaml; then pm="pnpm"
    elif has yarn.lock; then pm="yarn"
    elif has package-lock.json; then pm="npm"
    fi

    framework="none"
    if has_dep package.json '"vitest"' || has vitest.config.ts || has vitest.config.js || has vitest.config.mts; then
        framework="vitest"
    elif has_dep package.json '"jest"' || has jest.config.js || has jest.config.ts || has jest.config.mjs; then
        framework="jest"
    elif has_dep package.json '"mocha"'; then
        framework="mocha"
    elif has_dep package.json '"@playwright/test"'; then
        framework="playwright"   # e2e, not unit, but worth flagging
    fi

    coverage="none"
    if has_dep package.json '"@vitest/coverage-v8"\|"@vitest/coverage-istanbul"'; then
        coverage="v8"
    elif [ "$framework" = "jest" ]; then
        coverage="istanbul"
    fi

    tc=$(( $(count_files '*.test.ts' 999) + $(count_files '*.test.tsx' 999) + \
           $(count_files '*.test.js' 999) + $(count_files '*.test.jsx' 999) + \
           $(count_files '*.spec.ts' 999) + $(count_files '*.spec.tsx' 999) + \
           $(count_files '*.spec.js' 999) + $(count_files '*.spec.jsx' 999) ))
    evidence="package.json"
    has tsconfig.json && evidence="$evidence,tsconfig.json"
    [ "$framework" != "none" ] && evidence="$evidence,${framework}-config"
    add_entry js "$pm" "$framework" "$coverage" "$tc" "$evidence"
fi

# --- Python -------------------------------------------------------------------
if has pyproject.toml || has setup.py || has requirements.txt || has Pipfile; then
    pm="uv"
    has poetry.lock && pm="poetry"
    has Pipfile.lock && pm="pipenv"
    has requirements.txt && [ ! -f uv.lock ] && [ ! -f poetry.lock ] && pm="pip"

    framework="none"
    if has_dep pyproject.toml 'pytest' || has pytest.ini || has setup.cfg; then framework="pytest"
    elif has_dep pyproject.toml 'unittest' || [ "$(count_files 'test_*.py' 1)" -gt 0 ]; then framework="pytest"
    fi

    coverage="none"
    if has_dep pyproject.toml 'pytest-cov\|coverage'; then coverage="coverage.py"
    fi

    tc=$(( $(count_files 'test_*.py' 999) + $(count_files '*_test.py' 999) ))
    add_entry python "$pm" "$framework" "$coverage" "$tc" "pyproject.toml"
fi

# --- Go -----------------------------------------------------------------------
if has go.mod; then
    framework="go-test"   # built-in
    coverage="go-cover"   # built-in
    tc=$(count_files '*_test.go' 999)
    add_entry go "go modules" "$framework" "$coverage" "$tc" "go.mod"
fi

# --- Rust ---------------------------------------------------------------------
if has Cargo.toml; then
    framework="cargo-test"   # built-in
    coverage="none"
    if command -v cargo-llvm-cov >/dev/null 2>&1; then coverage="cargo-llvm-cov"
    elif command -v cargo-tarpaulin >/dev/null 2>&1; then coverage="tarpaulin"
    fi
    # #[test] inside src/ + tests/ files
    src_tests=$(grep -rl '#\[test\]' src 2>/dev/null | wc -l | tr -d ' ')
    int_tests=$(count_files '*.rs' 999)   # tests/ folder ones; not perfect
    tc=$((src_tests + int_tests))
    add_entry rust cargo "$framework" "$coverage" "$tc" "Cargo.toml"
fi

# --- Java ---------------------------------------------------------------------
java_marker=""
has pom.xml && java_marker="pom.xml"
[ -z "$java_marker" ] && { has build.gradle && java_marker="build.gradle"; }
[ -z "$java_marker" ] && { has build.gradle.kts && java_marker="build.gradle.kts"; }
if [ -n "$java_marker" ] && [ "$(count_files '*.java' 1)" -gt 0 ]; then
    pm="maven"
    [ "$java_marker" != "pom.xml" ] && pm="gradle"
    framework="none"
    has_dep "$java_marker" 'junit-jupiter\|org.junit.jupiter' && framework="junit5"
    [ "$framework" = "none" ] && { has_dep "$java_marker" 'junit:junit\|junit-4' && framework="junit4"; }
    coverage="none"
    has_dep "$java_marker" 'jacoco' && coverage="jacoco"
    tc=$(count_files '*Test.java' 999)
    add_entry java "$pm" "$framework" "$coverage" "$tc" "$java_marker"
fi

# --- Kotlin -------------------------------------------------------------------
if [ "$(count_files '*.kt' 1)" -gt 0 ]; then
    framework="none"
    has_dep build.gradle.kts 'kotest' && framework="kotest"
    [ "$framework" = "none" ] && { has_dep build.gradle.kts 'junit-jupiter' && framework="junit5"; }
    coverage="none"
    has_dep build.gradle.kts 'kover' && coverage="kover"
    [ "$coverage" = "none" ] && { has_dep build.gradle.kts 'jacoco' && coverage="jacoco"; }
    tc=$(count_files '*Test.kt' 999)
    add_entry kotlin gradle "$framework" "$coverage" "$tc" "build.gradle.kts"
fi

# --- .NET ---------------------------------------------------------------------
if [ "$(count_files '*.csproj' 1)" -gt 0 ] || [ "$(count_files '*.sln' 1)" -gt 0 ]; then
    framework="none"
    # heuristic: any *Test.cs file using xUnit Fact/Theory
    if grep -rqE '\[(Fact|Theory)\]' --include='*.cs' . 2>/dev/null; then framework="xunit"
    elif grep -rqE '\[(TestMethod)\]' --include='*.cs' . 2>/dev/null; then framework="mstest"
    elif grep -rqE '\[(Test)\]' --include='*.cs' . 2>/dev/null; then framework="nunit"
    fi
    coverage="none"
    # coverlet is the de-facto coverage runner; ships with dotnet test
    grep -rqE 'coverlet' --include='*.csproj' . 2>/dev/null && coverage="coverlet"
    tc=$(count_files '*Test.cs' 999)
    add_entry dotnet "dotnet sdk" "$framework" "$coverage" "$tc" "*.csproj"
fi

# --- Ruby ---------------------------------------------------------------------
if has Gemfile; then
    framework="none"
    has_dep Gemfile 'rspec' && framework="rspec"
    [ "$framework" = "none" ] && { has_dep Gemfile 'minitest' && framework="minitest"; }
    coverage="none"
    has_dep Gemfile 'simplecov' && coverage="simplecov"
    tc=$(( $(count_files '*_spec.rb' 999) + $(count_files '*_test.rb' 999) ))
    add_entry ruby bundler "$framework" "$coverage" "$tc" "Gemfile"
fi

# --- PHP ----------------------------------------------------------------------
if has composer.json; then
    framework="none"
    has_dep composer.json 'phpunit/phpunit' && framework="phpunit"
    coverage="none"
    if has_dep composer.json 'phpunit/phpunit'; then
        # PHPUnit uses Xdebug or PCOV; can't detect from composer alone.
        coverage="xdebug-or-pcov"
    fi
    tc=$(count_files '*Test.php' 999)
    add_entry php composer "$framework" "$coverage" "$tc" "composer.json"
fi

# --- Swift --------------------------------------------------------------------
if has Package.swift || [ "$(count_files '*.xcodeproj' 1)" -gt 0 ]; then
    framework="none"
    # Swift Testing uses @Test macro; XCTest uses XCTestCase
    if grep -rqE '@Test' --include='*.swift' . 2>/dev/null; then framework="swift-testing"
    elif grep -rqE 'XCTestCase' --include='*.swift' . 2>/dev/null; then framework="xctest"
    fi
    coverage="none"
    # Xcode produces coverage when -enableCodeCoverage is set; can't probe from files
    tc=$(( $(count_files '*Tests.swift' 999) + $(count_files '*Test.swift' 999) ))
    add_entry swift spm "$framework" "$coverage" "$tc" "Package.swift"
fi

# --- Shell --------------------------------------------------------------------
if [ "$(count_files '*.bats' 1)" -gt 0 ]; then
    framework="bats"
    coverage="none"   # kcov optional, requires CLI install
    command -v kcov >/dev/null 2>&1 && coverage="kcov"
    tc=$(count_files '*.bats' 999)
    add_entry shell bash "$framework" "$coverage" "$tc" "*.bats"
fi

# --- JSON output --------------------------------------------------------------

printf '{\n'
printf '  "root": "%s",\n' "$(json_str "$ROOT")"
printf '  "stacks": ['
first=1
while IFS=$'\t' read -r id pm fw cov tc ev; do
    [ -z "$id" ] && continue
    if [ "$first" -eq 1 ]; then first=0; printf '\n'; else printf ',\n'; fi
    printf '    {"id":"%s","package_manager":"%s","test_framework":"%s",' "$id" "$pm" "$fw"
    printf '"coverage_tool":"%s","existing_test_files":%s,"evidence":[' "$cov" "$tc"
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
printf ']\n}\n'
