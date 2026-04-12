# Session Protocol Reference

WAL format, spec addressing, conflict resolution, REVIEW markers, and session lifecycle.

---

## 1. WAL (Write-Ahead Log)

Records intention before execution. Enables recovery after session termination. Contains continuation data, not history (git log is history).

### Location

```
specs/WAL.md                      # single-product repo
specs/<product>/WAL.md            # multi-product repo
```

### Format

```markdown
# WAL — <project>
last_commit_sha: <sha>
updated_at: <ISO-8601>
status: ACTIVE | AWAITING_HUMAN
pending: <none | human action required>
resume_from: <step or command>
resume_when: <observable condition>

## Current: <step name>
<spec-id>: <title> — <STATUS>

## Done
- <spec-id>: COMPLETE

## In Progress
- <spec-id>§5 (Verification):
  - DONE: <what> (spec://<module>/<doc>#<section>)
  - TODO: <what's next>

## Blocked
- <issue>: awaiting <what> from <whom>

## Decisions Pending
- spec://<ref>: <question>

## REVIEW Markers Pending
- <file>:<line> — <concern>

## Anti-Instructions
- Do NOT touch <component> — <reason> (owner: <name>, added: <date>)
```

### Rules

1. **Updated at session end — no exceptions.** Also at every human-wait checkpoint (decide, approve).
2. **Contains continuation data, not history.**
3. **Completed items collapse.** `§1-4: COMPLETE`, not four lines.
4. **Anti-instructions protect stable code.** Require owner and reason. Review monthly.
5. **Both processes write.** Human: decisions/priorities. AI: progress/blockers.
6. **Read-first: every session starts by reading WAL.**

### Human Wait State (`AWAITING_HUMAN`)

Required fields: `status: AWAITING_HUMAN`, `pending`, `resume_from`, `resume_when`.

Resume: re-read WAL + spec + record. Check if spec/preview changed while waiting. If yes, re-preview. Set `status: ACTIVE`, continue from `resume_from`. Never continue past `AWAITING_HUMAN` on assumption.

### WAL Saves vs. Ship Commits

WAL saves = local state persistence, no permission needed. Local working commits during Build = allowed. Push/deploy = explicit human permission (Step 6).

### Recovery (Dirty WAL)

**Detection:** compare `last_commit_sha` with `git rev-parse HEAD`. Different → stale.

**Procedure:** read stale WAL → `git log <old_sha>..HEAD` → reconstruct current state → update WAL → proceed.

### Concurrency

WAL is single-writer. Parallel agents get separate WALs (`specs/WAL-<agent-id>.md`). Human reconciles.

---

## 2. Boot Sequence

```
1. CLAUDE.md / BOOT.md     — how to read everything else
2. WAL.md                  — continuation state
3. Relevant specs          — ONLY those in WAL "In Progress"
4. Code                    — ONLY files relevant to current TODO
```

Do NOT load all specs. WAL tells what to load.

---

## 3. Session Lifecycle

```
SESSION START:
  1. Read WAL
  2. Read specs in "In Progress"
  3. Identify next TODO
  4. Begin work

SESSION END:
  1. Update WAL with progress
  2. Collapse completed items
  3. Add blockers / decisions pending
  4. Add anti-instructions for new stable code
  5. Commit WAL
```

---

## 4. spec:// URI

```
spec://<module>/<document>#<section>[.<subsection>]
```

Resolution: `.s5d/packages/` first → `specs/<module>/` → project root. First match wins. No match → report, don't guess.

Use in: WAL, commit messages (`Implements: spec://...`), REVIEW markers, task descriptions. No prose references when URI exists.

---

## 5. Conflict Resolution

**Priority:** Human > Spec > Code > Tests.

**AI disagrees with spec:** implement spec exactly, add REVIEW marker with `spec://` reference, report at checkpoint. AI does not resolve spec disagreements autonomously.

**Test contradicts spec:** escalate — REVIEW marker + WAL "Decisions Pending". Human decides.

**Spec changes mid-session:** stop, re-read, report delta if it affects current work.

**Verbal requirement change:** ask human to update spec file first. Verbal changes are not authoritative.

---

## 6. REVIEW Markers

Add when AI implements something it disagrees with, or makes a non-obvious tradeoff decision.

```rust
// REVIEW(spec://module/doc#section): spec says X, AI thinks Y.
// Implemented as specified.
```

```markdown
<!-- REVIEW: spec://ref — Concern: ... Implemented as specified. -->
```

For non-commentable files: `<filename>.review.md` sidecar.

Rules: always include `spec://` URI, state what-was-implemented vs. what-AI-prefers, track in WAL "REVIEW Markers Pending", remove when resolved.

---

## 7. Atomicity

One commit = one logical change. Each commit references its spec point:

```
[module] implement verification timeout

Implements: spec://oproto/PROP-003#verification.timeout
```

Don't mix unrelated spec points in one commit — it makes REVIEW diffs ambiguous.
