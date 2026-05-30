# Tribunal verdict — S5D cluster runtime, round 3 (confirmatory, 2026-05-30)

Runtime: Claude Code (arbiter) | Auditors: Codex (gpt-5.3, high) + Gemini (3-flash)
Scope: the two fixed scripts `skills/system-design/scripts/adr.sh` +
`skills/domain-refactor/scripts/plan.sh`. Confirms round-2 fixes and hunts regressions.

- **Gemini:** APPROVE / NONE — all 6 prior findings confirmed resolved, no regressions.
- **Codex:** CONCERNS / MEDIUM — one NEW finding (below), all else confirmed resolved.
- **ARBITER DECISION: APPROVE** — Codex's single finding was valid, fixed, and verified
  in this pass; Gemini already clean. No HIGH/CRITICAL anywhere across 3 rounds.

## Confirmed resolved (both auditors)

- adr.sh commodity allowlist blocks `../`; valid flat names pass.
- plan.sh atomic mktemp→mv: python crash leaves `$OUTPUT` untouched, exits non-zero, no `✓`.
- single EXIT trap cleans both temps (no leak), safe under `set -u` on bash 3.2 (no arrays).
- `shlex.quote` closes shell-injection-on-paste in the `write.sh` example.
- `san()` neutralizes backticks/HTML/links/control chars.
- add-hypothesis failures propagate non-zero, no `✓` on partial (both scripts).

## Round-3 finding → fixed this pass

- **[MED — Codex] `plan.sh` shq markdown-break** (write.sh example). `shlex.quote` made the
  path shell-safe but left backticks/newlines that close the markdown inline-code span and
  inject layout. **Fix:** `shq()` now strips control chars + backticks BEFORE `shlex.quote`.
  **Verified:** path `` src/`whoami`\na b;rm -rf $(x).ts `` → emitted
  `'src/whoami a b;rm -rf $(x).ts'` — no backtick in span (delimiter count = 2), newline
  collapsed, shell metachars inert inside single quotes.

## Cross-round disposition

Round 1 REJECT/HIGH → round 2 closed the 2 HIGH + 3 MED → round 3 closed the last MED.
Final state: **APPROVE**. Verified end-to-end under bash 3.2 and the live s5d CLI
(`--decision-spec` produces a valid spec with hypotheses; adr.sh produces a valid decision
spec; no temp leaks; failure paths exit non-zero without false success).

Note (out of tribunal scope, not blocking): `plan.sh` python uses the deprecated
`datetime.utcnow()` — emits a DeprecationWarning on Python 3.12+ to stderr only; plan
output is unaffected.
