# Tribunal verdict — S5D skill-cluster runtime, round 2 (post-fix, 2026-05-30)

Runtime: Claude Code (arbiter) | Auditors: Codex (gpt-5.3, high) + Gemini (3-flash)
Scope: `skills/system-design/scripts/adr.sh`, `skills/domain-refactor/scripts/plan.sh`,
`skills/s5d/scripts/flatten.sh`, and the 6 `*-assess` agent defs.
Follows round 1 (REJECT/HIGH, `cluster-runtime-20260530.md`).

- **Codex:** REJECT / MEDIUM
- **Gemini:** CONCERNS / MEDIUM
- **ARBITER DECISION (pre-fix): REJECT / MEDIUM** — both converge on one blocker
  (commodity path traversal); Codex adds three valid secondary findings.

## What round 1's HIGH fixes confirmed resolved (both auditors agree)

- adr.sh YAML injection + arbitrary `--output` write — **resolved** (raw-YAML fallback
  removed, CLI required; `--output` flag deleted entirely).
- plan.sh raw-YAML fallback — **resolved** (CLI required).
- plan.sh markdown/prompt injection — **resolved** (`san()` neutralizes
  backticks/HTML/links/control chars; verified on a hostile path).
- add-hypothesis exit-code propagation (adr.sh + plan.sh) — **resolved** (non-zero exit,
  no `✓` on partial; verified winner-missing → rc=1).
- flatten.sh mixed-case severity — **resolved** (`ascii_downcase` before ranking;
  verified `"HIGH"` now ranks high).
- agent read-only claim (hard-rules body) — **resolved** (honest instruction-level wording).

## Round-2 findings → all fixed this pass

1. **[MED, BLOCKER — both auditors] commodity path traversal** (`adr.sh`).
   `COMMODITY=$1` unvalidated → `../x` escaped `templates/decisions` and parsed any
   caller-readable `*.yaml` as a decision frame. This was round-1 MED #5, not closed.
   **Fix:** allowlist `case "$COMMODITY" in *[!A-Za-z0-9_-]*|"")` reject.
   **Verified:** `../../../etc/passwd` and `queue/../auth` → rc=2.
2. **[MED — Codex] partial output after `>` truncate** (`plan.sh`). Python crash on a
   malformed analyze report left a truncated `$OUTPUT`, then the script printed `✓`.
   **Fix:** generate into `mktemp`, propagate python rc, atomic `mv` only on success;
   single EXIT trap for both temps (bash traps don't stack).
   **Verified:** malformed JSON → rc=1, no `✓`, pre-existing output preserved, no temp leak.
3. **[LOW — Codex] unquoted path in copyable command** (`plan.sh`). The `write.sh <path>`
   example interpolated a raw path; spaces/`;`/`$()`/leading-dash could inject when pasted.
   **Fix:** `shq()` = `shlex.quote` on that path. **Verified:** `src/a b;rm -rf $(x).ts`
   → emitted single-quoted, inert.
4. **[LOW — Codex] frontmatter over-claim** (6 agent defs). `description:` still said
   "Read-only — never mutates the repo" while `tools:` includes Bash.
   **Fix:** softened to "Read-only assessment — analysis only … (instruction-enforced)".
   **Verified:** 0 absolute "never mutates" strings remain.

## Disposition

All round-2 findings are MEDIUM-or-below and all are fixed + deterministically verified
in this pass (evidence above). The HIGH class from round 1 stays resolved. A confirmatory
round 3 on the two fixed scripts is recommended to formalize REJECT→APPROVE.
