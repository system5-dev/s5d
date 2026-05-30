#!/usr/bin/env bash
# flatten.sh — distill a skill's analyze JSON into an anomalies-only markdown block.
#
# Deterministic (jq): keeps findings at/above a severity floor, drops the rest.
# This is the "flattener" from decision.skill-cluster-decomposition: the assess
# agents run their bash internally and pass the JSON through this filter so the
# orchestrator only ever sees anomalies, never raw dumps — and the distillation
# is a fixed rule, not LLM prose that can drift.
#
# Input shape (the cluster's analyze.sh contract):
#   { "root": "...", "findings": [ {"check","severity","path","detail","fix",...} ], "summary": {...} }
#
# Usage:
#   bash flatten.sh <analyze.json> [label]
#   <skill>/scripts/analyze.sh | bash flatten.sh - <label>
#
# Env:
#   FLATTEN_MIN_SEVERITY=medium   # floor: critical|high|medium|low|info (default medium)

set -uo pipefail

SRC="${1:-}"
LABEL="${2:-findings}"
MIN="${FLATTEN_MIN_SEVERITY:-medium}"

[ -z "$SRC" ] && { echo "usage: $0 <analyze.json|-> [label]" >&2; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "flatten: jq required" >&2; exit 2; }

if [ "$SRC" = "-" ]; then JSON="$(cat)"; else
  [ -f "$SRC" ] || { echo "flatten: no such file: $SRC" >&2; exit 2; }
  JSON="$(cat "$SRC")"
fi

# Severity ranks; anything at/above the floor survives.
RANK='{"critical":4,"high":3,"medium":2,"low":1,"info":0}'

echo "$JSON" | jq -r --argjson rank "$RANK" --arg min "$MIN" --arg label "$LABEL" '
  # SARIF level -> our severity vocabulary.
  {"error":"high","warning":"medium","note":"low","none":"info"} as $sarif
  # Normalize the three cluster output shapes into one item set:
  #   findings[]    (ai-technical-writer/ddd-refactor/infra-scan/scaling-review)
  #   violations[]  (domain-refactor)
  #   runs[].results[] SARIF (security-scan)
  | ( if (.findings | type) == "array" then
        [ .findings[] | {sev:(.severity // "info"), label:(.check // "?"), path:(.path // ""), detail:(.detail // ""), fix:(.fix // "")} ]
      elif (.violations | type) == "array" then
        [ .violations[] | {sev:(.severity // "info"), label:(.kind // "?"), path:(.path // ""), detail:(.detail // ""), fix:""} ]
      elif (.runs | type) == "array" then
        [ .runs[].results[]? | {sev:(($sarif[(.level // "warning")]) // "medium"), label:(.ruleId // "?"), path:((.locations[0].physicalLocation.artifactLocation.uri) // ""), detail:((.message.text) // ""), fix:""} ]
      else [] end ) as $items
  # ascii_downcase before ranking: cluster scripts emit severities in mixed case
  # (e.g. "HIGH", "High"); without normalizing, $rank lookup misses and silently
  # downgrades them to 0/info, dropping real anomalies (tribunal LOW, flatten.sh).
  | ($rank[($min|ascii_downcase)] // 2) as $floor
  | [ $items[] | select( ($rank[(.sev|ascii_downcase)] // 0) >= $floor ) ] as $kept
  | "## " + $label + " — " + ($kept | length | tostring) + " anomaly(ies) >= " + $min,
    ( if ($kept|length) == 0
      then "✓ none at/above " + $min
      else ($kept | sort_by(-($rank[(.sev|ascii_downcase)] // 0))[]
            | "- **[" + (.sev | ascii_upcase) + "]** "
              + .label
              + (if (.path|length>0) and (.path != "(repo)") then " (`" + .path + "`)" else "" end)
              + " — " + (.detail | gsub("\n";" "))
              + (if (.fix|length>0) then "  _fix:_ " + (.fix | gsub("\n";" ")) else "" end) )
      end )
'
