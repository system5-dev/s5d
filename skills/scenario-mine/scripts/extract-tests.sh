#!/usr/bin/env bash
# extract-tests.sh — pull test names from source files.
# Reads a newline-separated list of files on stdin (or via positional args).
# Output: JSON array of {file, line, kind, name, lang}.
#
# Supported patterns (regex via grep -nE):
#   JS/TS:  describe('NAME', ...) | it('NAME', ...) | test('NAME', ...) (single OR double quotes)
#   Python: def test_NAME(
#   Go:     func TestNAME( | Benchmark* | Example* | Fuzz*
#   Rust:   #[test] on one line, fn NAME on the next
#   Java:   @Test on its own line, void NAME( within 3 lines
#   .NET:   [Fact|Theory|TestMethod|Test] then void NAME( within 4 lines
#   Ruby:   it / describe / context / specify '...'
#   PHP:    public function testNAME(
#
# Usage:
#   bash extract-tests.sh path/to/a path/to/b
#   echo -e "a.test.ts\nb_test.go" | bash extract-tests.sh

set -uo pipefail   # NO -e: grep returns 1 on zero matches; that's expected.

if [ $# -gt 0 ]; then
    FILES="$*"
else
    FILES="$(cat)"
fi

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

# Emit one JSON object per line. Post-processor joins with commas + brackets.
emit_one() {
    local file="$1" line="$2" kind="$3" name="$4" lang="$5"
    printf '{"file":"%s","line":%d,"kind":"%s","name":"%s","lang":"%s"}\n' \
        "$(json_str "$file")" "$line" "$(json_str "$kind")" "$(json_str "$name")" "$lang"
}

scan_js() {
    local f="$1"
    grep -nE '^[[:space:]]*(describe|it|test)\.?[a-z]*\(['\''"]' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local kind name
            kind=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]*([a-z]+)\.?[a-z]*\(.*/\1/')
            name=$(printf '%s' "$rest" | sed -E "s/^[[:space:]]*[a-z]+\.?[a-z]*\(['\"]([^'\"]+)['\"].*/\1/")
            [ -n "$name" ] && [ "$name" != "$rest" ] && emit_one "$f" "$ln" "$kind" "$name" "js"
        done
}

scan_python() {
    local f="$1"
    grep -nE '^[[:space:]]*def[[:space:]]+test_[A-Za-z0-9_]+\(' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local name
            name=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]*def[[:space:]]+(test_[A-Za-z0-9_]+).*/\1/')
            emit_one "$f" "$ln" "test" "$name" "python"
        done
}

scan_go() {
    local f="$1"
    grep -nE '^func[[:space:]]+(Test|Benchmark|Example|Fuzz)[A-Za-z0-9_]+\(' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local name kind
            name=$(printf '%s' "$rest" | sed -E 's/^func[[:space:]]+([A-Za-z0-9_]+).*/\1/')
            kind=$(printf '%s' "$name" | sed -E 's/^(Test|Benchmark|Example|Fuzz).*/\1/' | tr 'A-Z' 'a-z')
            emit_one "$f" "$ln" "$kind" "$name" "go"
        done
}

scan_rust() {
    # Two-line: #[test] line, fn NAME on a subsequent line within 3 lines.
    local f="$1"
    awk '
        /^[[:space:]]*#\[test\]/ { marker=NR+3; next }
        marker >= NR && /^[[:space:]]*fn[[:space:]]+[A-Za-z0-9_]+/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "fn") {
                    n = $(i+1); sub(/\(.*/, "", n); sub(/<.*/, "", n)
                    print NR ":" n
                    marker = 0
                    next
                }
            }
        }
    ' "$f" 2>/dev/null \
        | while IFS=: read -r ln name; do
            emit_one "$f" "$ln" "test" "$name" "rust"
        done
}

scan_java() {
    local f="$1"
    awk '
        /^[[:space:]]*@Test/ { marker=NR+3; next }
        marker >= NR && /void[[:space:]]+[A-Za-z0-9_]+[[:space:]]*\(/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "void") {
                    n = $(i+1); sub(/\(.*/, "", n)
                    print NR ":" n
                    marker = 0
                    next
                }
            }
        }
    ' "$f" 2>/dev/null \
        | while IFS=: read -r ln name; do
            emit_one "$f" "$ln" "test" "$name" "java"
        done
}

scan_dotnet() {
    local f="$1"
    awk '
        /\[(Fact|Theory|TestMethod|Test)\]/ { marker=NR+4; next }
        marker >= NR && /void[[:space:]]+[A-Za-z0-9_]+[[:space:]]*\(/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "void") {
                    n = $(i+1); sub(/\(.*/, "", n)
                    print NR ":" n
                    marker = 0
                    next
                }
            }
        }
    ' "$f" 2>/dev/null \
        | while IFS=: read -r ln name; do
            emit_one "$f" "$ln" "test" "$name" "dotnet"
        done
}

scan_ruby() {
    local f="$1"
    grep -nE '^[[:space:]]*(it|describe|context|specify)[[:space:]]+['\''"]' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local kind name
            kind=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]*([a-z]+).*/\1/')
            name=$(printf '%s' "$rest" | sed -E "s/^[[:space:]]*[a-z]+[[:space:]]+['\"]([^'\"]+)['\"].*/\1/")
            [ -n "$name" ] && [ "$name" != "$rest" ] && emit_one "$f" "$ln" "$kind" "$name" "ruby"
        done
}

scan_php() {
    local f="$1"
    grep -nE 'public[[:space:]]+function[[:space:]]+test[A-Za-z0-9_]+\(' "$f" 2>/dev/null \
        | while IFS=: read -r ln rest; do
            local name
            name=$(printf '%s' "$rest" | sed -E 's/.*function[[:space:]]+(test[A-Za-z0-9_]+).*/\1/')
            emit_one "$f" "$ln" "test" "$name" "php"
        done
}

dispatch() {
    local f="$1"
    case "$f" in
        *.test.ts|*.test.tsx|*.test.js|*.test.jsx|*.spec.ts|*.spec.tsx|*.spec.js|*.spec.jsx) scan_js "$f" ;;
        *.py)    scan_python "$f" ;;
        *_test.go) scan_go "$f" ;;
        *.rs)    scan_rust "$f" ;;
        *Test.java|*Tests.java) scan_java "$f" ;;
        *Test.cs|*Tests.cs|*Spec.cs) scan_dotnet "$f" ;;
        *_spec.rb|*_test.rb) scan_ruby "$f" ;;
        *Test.php) scan_php "$f" ;;
        *) ;;
    esac
}

# Emit object-per-line, then wrap into a JSON array.
{
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        [ ! -f "$f" ] && continue
        dispatch "$f"
    done <<< "$FILES"
} | awk '
    BEGIN { print "[" }
    NR == 1 { printf "  %s", $0; next }
    { printf ",\n  %s", $0 }
    END { if (NR > 0) printf "\n"; print "]" }
'
