# Tribunal verdict — S5D skill-cluster runtime (2026-05-30)

Runtime: Claude Code (arbiter) | Auditors: Codex (gpt-5.3, medium) + Gemini (3-flash)
Scope: `skills/s5d/scripts/flatten.sh`, the 6 `*-assess` agent defs, and the
CLI-mediated producers `skills/system-design/scripts/adr.sh` + `skills/domain-refactor/scripts/plan.sh`.

- **Codex:** REJECT / HIGH
- **Gemini:** REJECT / HIGH (CONCERNS-to-REJECT)
- **ARBITER DECISION: REJECT / HIGH** — both converge; not approvable as-is. All findings are fixable.

This regenerates the verdict from the prior tribunal whose 5 must-fix were never
recorded in S5D and were lost. Recorded here so it cannot be lost again.

## Must-fix (consolidated, by severity)

1. **[HIGH] adr.sh YAML injection** (`adr.sh:140-174`, fallback heredoc). User-controlled
   `COMMODITY/CHOSEN/DECIDED_BY/RATIONALE/PRODUCT/ANSWERS_FILE/VARIANT_BLOB` written into a
   YAML heredoc with no escaping — newlines, `:`, `---`, anchors can inject/corrupt YAML.
   Fix: drop the raw-YAML fallback (CLI path is primary) or YAML-escape every interpolation.
2. **[HIGH] adr.sh arbitrary `--output` write** (`adr.sh:129-130`). Fallback `cat > "$OUTPUT"`
   after `mkdir -p` can overwrite any caller-writable path outside `.s5d/packages`.
   Fix: confine `--output` under `.s5d/packages/` (reject `..`, absolute escapes).
3. **[MED] Silent partial state** (`adr.sh:80-87`, `plan.sh:205-216`). `s5d add-hypothesis`
   failures are treated as warnings; scripts still print `✓ wrote` and exit 0.
   Fix: non-zero exit + no success message when any add-hypothesis fails.
4. **[MED] plan.sh markdown/prompt injection** (`plan.sh:91-166`). Analyze JSON fields
   (filenames/details) inserted raw into Markdown — backticks/links/HTML reach downstream
   agents/reviewers. Fix: escape/strip control text before interpolation.
5. **[MED] adr.sh commodity path traversal** (`adr.sh:34`). `YAML="$DECISIONS_DIR/${COMMODITY}.yaml"`
   — `../x` selects files outside `templates/decisions`. Fix: validate COMMODITY (no `/`, `..`).

## Lower / noted

- [LOW] `adr.sh:38-42`, `plan.sh:23-27` — `$2` read without presence check → unbound-var crash on `--chose`/`--output` with no value.
- [LOW] `plan.sh:31-35` — `.new` suffix overwrites an existing `.new`.
- [LOW] `flatten.sh:43-63` — assumes lowercase string severity; uppercase/non-string silently downgraded. Fix: `ascii_downcase` the severity before ranking.
- [LOW] agent def — "verbatim, but one-line lead allowed" weakens the machine-consumable contract.
- [ARBITER add] Gemini flagged: assess agents have `Bash` while `Write` is disallowed — `Bash` can write (`echo > f`), so the "read-only" invariant is NOT enforced, only instructed. Codex rated the agent low-risk. Arbiter: real limitation, instruction-level mitigation is the practical ceiling for a Bash-enabled agent; document it rather than claim hard read-only.

## Disposition

adr.sh / plan.sh are from feat.s5d.skill-suite-integration (CLI-mediated producers).
flatten.sh / agents are from feat.s5d.cluster-agent-decomposition. Fixes tracked as a
follow-up hardening pass; #1–#2 (HIGH) are blocking before these producers are trusted.
