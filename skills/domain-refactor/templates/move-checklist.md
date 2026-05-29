# Move Checklist

For every code-touching refactor slice. Tick each box BEFORE running `verify.sh`.

## Pre-move

- [ ] Working tree clean (`git status` empty).
- [ ] `verify.sh --capture` ran — baseline snapshot taken.
- [ ] s5d decision recorded if refactor ≥ 30 LOC (`s5d new decision.<id> --tier decision`).
- [ ] All affected files are **safe** or **risky** in `analyze.sh` output. If any are **unsafe** — STOP and write tests first.
- [ ] Use case `coverage.sh` baseline noted (for e2e use-case % regression check).

## Move

- [ ] `apply.sh move-component <src> <dst>` — single atomic move.
- [ ] `git status` shows ONLY the expected diff (one renamed file + import rewrites + map row update).
- [ ] No unrelated files modified — check carefully; the import-rewrite is best-effort and can over-match.

## Post-move

- [ ] `tsc --noEmit` passes.
- [ ] `bash ~/.agents/skills/unit-tests/scripts/run.sh` — unit tests pass; coverage threshold met.
- [ ] `bash ~/.agents/skills/e2e-tests/scripts/coverage.sh` — use-case % did not drop.
- [ ] `bash ~/.agents/skills/domain-refactor/scripts/verify.sh` — comparison vs. baseline passes.
- [ ] `bash ~/.agents/skills/domain-refactor/scripts/analyze.sh` — total violations did not increase.
- [ ] `bash ~/.agents/skills/domain-refactor/scripts/trace.sh <new-path>` — file resolves to its domain / capabilities / use cases.
- [ ] Architecture map updated in the SAME commit (path row, plus any contract / capability rename).
- [ ] Commit message references the s5d decision: `S5D-Spec: decision.<id>`.

## Rollback

If `verify.sh` fails or unit-tests break:

- [ ] `git reset --hard HEAD` (after confirming there are no uncommitted changes worth keeping).
- [ ] Diagnose: which check failed? Was it import path, type, runtime?
- [ ] Update the plan to address the root cause BEFORE retrying the move.
- [ ] If retried slice still fails: bring it to s5d as a new hypothesis ("refactor X requires Y prerequisite").
