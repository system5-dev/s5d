#!/usr/bin/env bash
# write.sh — scaffold a test file for a given source file.
# Picks the right stack from the extension, copies the right stub template,
# substitutes {{MODULE}} / {{SOURCE_PATH}} / {{TIMESTAMP}}, writes to the
# canonical test path for the stack.
#
# Usage:
#   bash write.sh app/api/coi/route.ts
#   bash write.sh --dry-run src/utils/pricing.ts
#
# Scaffold contains a Background block (where applicable), one TODO-marked
# scenario, and a clear "delete this" comment. Agent / human fills in real
# assertions.

set -uo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$SKILL_DIR/templates"

DRY=0
SRC=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY=1; shift ;;
        -h|--help)
            sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
            exit 0
            ;;
        -*)
            echo "unknown flag: $1" >&2; exit 2 ;;
        *)
            SRC="$1"; shift ;;
    esac
done

[ -z "$SRC" ] && { echo "usage: $0 <source-file>" >&2; exit 2; }
[ ! -f "$SRC" ] && { echo "no such file: $SRC" >&2; exit 2; }

# Pick stack + stub + destination path from extension.
detect_target() {
    case "$SRC" in
        *.test.ts|*.test.tsx|*.test.js|*.test.jsx|*.spec.ts|*.spec.tsx|*.spec.js|*.spec.jsx)
            echo "→ $SRC is already a test file" >&2; exit 1 ;;
        *.ts|*.tsx)
            STACK=js; STUB="js/stub.test.ts"
            DST="${SRC%.*}.test.${SRC##*.}"
            ;;
        *.js|*.jsx)
            STACK=js; STUB="js/stub.test.ts"
            DST="${SRC%.*}.test.${SRC##*.}"
            ;;
        *.py)
            case "$(basename "$SRC")" in
                test_*) echo "→ already a test" >&2; exit 1 ;;
            esac
            STACK=python; STUB="python/stub_test.py"
            DST="$(dirname "$SRC")/test_$(basename "${SRC%.*}").py"
            ;;
        *.go)
            case "$SRC" in *_test.go) echo "→ already a test" >&2; exit 1;; esac
            STACK=go; STUB="go/stub_test.go"
            DST="${SRC%.*}_test.go"
            ;;
        *.rs)
            STACK=rust; STUB="rust/stub_tests.rs"
            DST="${SRC%.*}_tests.rs"
            ;;
        *.java)
            case "$SRC" in *Test.java|*Tests.java) echo "→ already a test" >&2; exit 1;; esac
            STACK=java; STUB="java/StubTest.java"
            # Java convention: src/test/java mirror of src/main/java
            DST=$(echo "$SRC" | sed -E 's#src/main/java/#src/test/java/#')
            DST="${DST%.*}Test.java"
            ;;
        *.kt)
            case "$SRC" in *Test.kt|*Tests.kt) echo "→ already a test" >&2; exit 1;; esac
            STACK=kotlin; STUB="kotlin/StubTest.kt"
            DST=$(echo "$SRC" | sed -E 's#src/main/kotlin/#src/test/kotlin/#')
            DST="${DST%.*}Test.kt"
            ;;
        *.cs)
            case "$SRC" in *Test.cs|*Tests.cs|*Spec.cs) echo "→ already a test" >&2; exit 1;; esac
            STACK=dotnet; STUB="dotnet/StubTest.cs"
            DST="${SRC%.*}Tests.cs"
            ;;
        *.rb)
            case "$SRC" in *_spec.rb|*_test.rb) echo "→ already a test" >&2; exit 1;; esac
            STACK=ruby; STUB="ruby/stub_spec.rb"
            DST="$(dirname "$SRC")/spec/$(basename "${SRC%.*}")_spec.rb"
            ;;
        *.php)
            case "$SRC" in *Test.php) echo "→ already a test" >&2; exit 1;; esac
            STACK=php; STUB="php/StubTest.php"
            DST=$(echo "$SRC" | sed -E 's#src/#tests/#')
            DST="${DST%.*}Test.php"
            ;;
        *.swift)
            case "$SRC" in *Tests.swift|*Test.swift) echo "→ already a test" >&2; exit 1;; esac
            STACK=swift; STUB="swift/StubTests.swift"
            DST=$(echo "$SRC" | sed -E 's#Sources/#Tests/#')
            DST="${DST%.*}Tests.swift"
            ;;
        *.sh|*.bash)
            STACK=shell; STUB="shell/stub.bats"
            DST="${SRC%.*}.bats"
            ;;
        *)
            echo "→ unsupported extension: $SRC" >&2; exit 1 ;;
    esac
}

detect_target

STUB_PATH="$TEMPLATE_DIR/$STUB"
[ ! -f "$STUB_PATH" ] && { echo "→ stub template missing: $STUB_PATH" >&2; exit 1; }

# Substitution variables
MODULE_NAME=$(basename "${SRC%.*}")
SOURCE_PATH="$SRC"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
RELATIVE_IMPORT=$(echo "$MODULE_NAME")   # Caller refines

echo "→ source:  $SRC" >&2
echo "→ stack:   $STACK" >&2
echo "→ stub:    $STUB_PATH" >&2
echo "→ dest:    $DST" >&2

if [ -e "$DST" ]; then
    echo "  ! destination exists; not overwriting. Inspect and merge manually." >&2
    exit 1
fi

if [ "$DRY" -eq 1 ]; then
    echo "── dry-run, would write: ──" >&2
    sed -e "s|{{MODULE}}|$MODULE_NAME|g" \
        -e "s|{{SOURCE_PATH}}|$SOURCE_PATH|g" \
        -e "s|{{TIMESTAMP}}|$NOW|g" "$STUB_PATH"
    exit 0
fi

mkdir -p "$(dirname "$DST")"
sed -e "s|{{MODULE}}|$MODULE_NAME|g" \
    -e "s|{{SOURCE_PATH}}|$SOURCE_PATH|g" \
    -e "s|{{TIMESTAMP}}|$NOW|g" "$STUB_PATH" > "$DST"

echo "✓ wrote $DST" >&2
echo "  next: open $DST, fill in TODOs, then run:" >&2
echo "        bash $SKILL_DIR/scripts/run.sh --stacks $STACK" >&2
