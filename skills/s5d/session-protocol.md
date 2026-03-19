# Session Protocol Reference

How two complementary processes share state across sessions: the coprocessor model, WAL, spec addressing, conflict resolution, REVIEW markers, memory architecture, and validation.

---

## 1. Coprocessor Model

Two processors with complementary architectures working on one task.

**Human process** is optimized for: persistent memory (survives across sessions and years), semantic understanding (grasps intent, not just form), intuition and taste, slow deep verification, decisions under uncertainty, aesthetic judgment.

**Human process** is weak at: throughput, mechanical consistency, holding many details simultaneously (7±2 working memory limit), routine operations, fatigue resistance.

**AI process** is optimized for: throughput (thousands of lines per minute), mechanical consistency within a session, wide shallow knowledge across domains, routine operations, formal structures, tirelessness within a session.

**AI process** is weak at: persistent memory (zero retention between sessions), semantic understanding (reads letter, misses spirit), long-distance coherence within large context, detecting its own errors, autonomous initiative without specification.

**Key observation:** weaknesses are complementary, not overlapping.

| Human | AI |
|-------|-----|
| Slow but remembers | Fast but forgets |
| Sees the picture, misses details | Holds details, misses the picture |
| Decides under uncertainty | Executes under specification |
| Memory persists across sessions | Memory resets every session |

The protocol exists to bridge this gap. Every mechanism below is a direct consequence of one of these asymmetries.

---

## 2. WAL (Write-Ahead Log) — Full Specification

### Purpose

Records intention before execution. Enables recovery after session termination, which is not an edge case — it is inevitable. Every session ends with context loss. WAL is the bridge.

WAL contains continuation data, not history. Git log is history.

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
- <spec-id>§1-4: COMPLETE, all tests pass

## In Progress
- <spec-id>§5 (Verification):
  - DONE: <what> (spec://<module>/<doc>#<section>)
  - TODO: <what's next>

## Blocked
- <issue>: awaiting <what> from <whom>

## Decisions Pending
- spec://<ref>: <question> — awaiting human decision

## REVIEW Markers Pending
- <file>:<line> — <concern>

## Anti-Instructions
- Do NOT touch <component> — <reason>
- Do NOT refactor <module> — out of scope
```

### Rules

1. **Updated at session end — no exceptions.** Also update at every human-wait checkpoint (Step 2 decide, Step 4 approve). Between those checkpoints WAL is not committed. If session crashes: see Recovery below.
2. **Contains continuation data, not history.** Git log holds history; WAL holds what to do next.
3. **Completed items collapse.** Write `§1-4: COMPLETE` not four separate lines.
4. **Anti-instructions protect stable code from improvisation.** When code is reviewed and stable, add an anti-instruction so the next session does not touch it. Format: `Do NOT touch <X> — <reason> (owner: <name>, added: <date>)`. Anti-instructions require an owner and a reason. Review monthly — stale anti-instructions become architecture barnacles.
5. **Both processes write.** Human updates decisions and priorities. AI updates progress and blockers.
6. **Read-first: every session starts by reading WAL.** No exceptions.

### Human Wait State (`AWAITING_HUMAN`)

Use this state whenever the flow hits a hard human gate: Step 2 decision confirmation, Step 4 approval, or any explicit human-owned unblocker.

Required fields:

- `status: AWAITING_HUMAN`
- `pending: <exact human action>`
- `resume_from: <step name or command>`
- `resume_when: <what counts as an unblock>`

Example:

```markdown
# WAL — auth
last_commit_sha: 2d4f7c1
updated_at: 2026-04-11T12:00:00Z
status: AWAITING_HUMAN
pending: approve feat.refresh-rotation__20260411.s5d.yaml
resume_from: Step 4 — Build
resume_when: reviewer name supplied by Roman
```

Resume procedure:

1. Re-read WAL, the referenced spec, and the matching `.record.yaml`.
2. Check whether the spec or preview diff changed while waiting. If yes, re-run `s5d_preview` and then `s5d_approve`.
3. Change WAL back to `status: ACTIVE`, clear or rewrite `pending`, and continue from `resume_from`.
4. Never continue past an `AWAITING_HUMAN` checkpoint based on assumption or stale approval.

### WAL Saves vs. Ship Commits

WAL saves are local state persistence, not reviewable ship commits. WAL can be saved to disk and committed without human permission — it is a scratch pad, not a deliverable.

Local working commits during Build (Step 4) are allowed without explicit human permission — they are part of the build loop and do not leave the local machine.

Push to remote requires explicit human permission (Step 6). Deploy requires explicit human permission (Step 6). Neither is done autonomously.

### Spec-ID Convention

WAL body uses bare IDs (`PROP-003§5`) for scannability. Full `spec://` URIs are required in: commit messages, REVIEW markers, task descriptions, and WAL "Decisions Pending" entries.

### No-WAL Initialization

If `specs/WAL.md` does not exist (first session on a project), create it after Bootstrap with initial state:

```markdown
# WAL — <project>
last_commit_sha: <current HEAD>
updated_at: <ISO-8601>
status: ACTIVE
pending: none
resume_from: Bootstrap
resume_when: n/a

## Current: Bootstrap
<no spec yet> — IN PROGRESS

## Done
(empty)

## In Progress
- Domain analysis: discovering problem space
```

Do not skip WAL initialization even for short projects.

### Recovery (Dirty WAL)

**Detection:** WAL header stores `last_commit_sha: <sha>`. On session start, compare with `git rev-parse HEAD`. If different → WAL is stale.

If the header is missing entirely, treat the WAL as pre-header legacy state: read it, reconstruct current status from git/spec/record files, then write the header on the next save.

**Procedure:**
1. Read WAL as-is (stale state is the last known good state)
2. Read `git log <last_commit_sha>..HEAD` to see what changed
3. Reconstruct current state from the diff
4. Update WAL to reflect reality, update `last_commit_sha`
5. Proceed with normal session

### Concurrency

WAL is single-writer. If multiple agents work in parallel, each gets a separate WAL (`specs/WAL-<agent-id>.md`), or agents serialize access via branch-per-agent. Do not merge WAL files — the human reconciles after each agent completes.

---

## 3. Boot Sequence

Order matters. Minimize context window. Load only what the current TODO requires.

```
1. AGENTS.md / CLAUDE.md / BOOT.md   — how to read everything else (~500 tokens)
2. WAL.md                — continuation state (~1–3K tokens)
3. Relevant specs         — ONLY those listed in WAL "In Progress"
4. Code                   — ONLY files relevant to current TODO
```

Do NOT load all specs. Do NOT read the entire codebase. WAL tells exactly what to load. Loading more is not thoroughness — it is noise that degrades attention weight on signal.

**Context expansion.** The minimal-load rule can be relaxed when the current task requires understanding adjacent code. When expanding, record the reason in WAL:

```
CONTEXT EXPANDED: read <file> because <reason>
```

This keeps the expansion visible and intentional rather than silent scope creep.

---

## 4. Session Lifecycle

```
SESSION START:
  1. Read WAL
  2. Read specs referenced in "In Progress"
  3. Identify next TODO
  4. Begin work

SESSION END:
  1. Update WAL with progress
  2. Collapse completed items
  3. Add new blockers / decisions pending
  4. Add anti-instructions if new stable code was produced
  5. Commit WAL
```

Session boundaries are hard. A session that does not update WAL on exit is incomplete, regardless of how much code was written.

---

## 5. spec:// URI — Full Specification

### Format

```
spec://<module>/<document>#<section>[.<subsection>]
```

### Examples

```
spec://oproto/PROP-003#verification.timeout
spec://auth/FEAT-001#acceptance.scenario-2
spec://s5d/decision.redbook-expansion#problem
```

### Resolution Precedence

1. `.s5d/packages/<document>*.s5d.yaml` — managed specs (checked first)
2. `specs/<module>/<document>.md` — manual specs
3. Project root `<document>.md` — fallback

First match wins. No match → report to human, do not guess.

### Where to Use

- WAL "In Progress" DONE/TODO lines
- Commit messages: `Implements: spec://...`
- REVIEW markers (mandatory)
- Task descriptions

**Prose references banned when a URI exists.** "The timeout thing" is not an address. `spec://oproto/PROP-003#verification.timeout` is.

---

## 6. Conflict Resolution Protocol

### Priority Chain

```
1. Human overrides spec   — human can change spec at any time
2. Spec overrides code    — code must match spec, not vice versa
3. Tests = executable spec — test contradicts spec → bug in test OR spec, not both
```

### When AI Disagrees with Spec

1. Implement spec exactly as written
2. Add a REVIEW marker (see Section 8) with spec:// reference
3. Report concern to human at session checkpoint
4. Human resolves in the next cycle

AI does not resolve spec disagreements autonomously. AI implements, flags, reports.

### When Test Contradicts Spec

Escalate — do not pick one silently:
1. Add REVIEW marker on the test with the contradiction noted
2. Add to WAL "Decisions Pending"
3. Human decides whether to fix the test or update the spec

### When Spec Changes Mid-Session

1. AI detects changed spec (file modification or `git diff`)
2. AI stops current task immediately
3. AI re-reads the updated spec fully
4. If changes affect current work → report the delta, ask whether to continue or restart
5. If changes do not affect current work → continue, note the update in WAL

**Never silently modify a spec.** Spec is human-controlled. AI proposes changes via REVIEW; human approves.

### When Human Changes Requirement Verbally

If a human changes a requirement in chat, a review comment, or any non-spec channel: AI does not act on the verbal change. AI asks the human to update the spec file first. Verbal changes are not authoritative until written to spec.

---

## 7. Three Communication Planes

The two processes communicate across three distinct planes. Mixing them causes confusion.

### Control Plane (human → AI)

Carries intent and authority.

- `BOOT.md` / `CLAUDE.md` — how to read everything else (~500 tokens)
- `AGENTS.md` — repo-local operating contract when present
- `WAL.md` — continuation state (up to 3K tokens)
- Specs — what the system must do

### Data Plane (AI → human, verifiable)

Carries produced artifacts. Human verifies mechanically.

- Code — generated by AI, verified by human via diff
- Tests — generated by AI per specs, verified by passing
- Spec updates — AI proposes, human approves before merging

### Signal Plane (bidirectional)

Carries flags and state changes.

- REVIEW markers — AI flags concern, human resolves
- Git diff — change notification (both read)
- Broken tests — automated spec violation signal
- WAL updates — session boundary signal

---

## 8. REVIEW Markers — Full Specification

### When to Add

When AI implements something it disagrees with, or makes a non-obvious decision with tradeoffs the human should be aware of.

### Format in Code

```rust
// REVIEW(spec://module/doc#section): spec says X, AI thinks Y.
// Implemented as specified.
fn verification_timeout() -> Duration {
    Duration::from_secs(600) // per spec, not AI preference
}
```

### Format in Markdown

```
<!-- REVIEW: spec://oproto/PROP-003#verification.timeout
     Concern: 600s timeout may be too long for real-time use.
     Suggested: 120s with configurable override.
     Implemented as specified (600s). -->
```

### Non-Commentable Files

For generated files, binary formats, or files where inline comments are impossible — use a sidecar file: `<filename>.review.md` with the same format as inline REVIEW markers. Track sidecar files in WAL "REVIEW Markers Pending" identically to inline markers.

### Rules

- Always include a `spec://` URI referencing the governing spec point.
- Always state what was implemented (the spec's requirement) vs. what the AI would prefer and why.
- Track all pending markers in WAL under "REVIEW Markers Pending" with file and line.
- When human resolves: accept spec, update spec, or acknowledge and keep. Either way, remove the marker from code AND from WAL. Do not let markers accumulate.

---

## 9. Atomicity

One commit = one logical change. One thought. One diff.

Each commit references the spec point it satisfies:

```
[module] implement verification timeout

Implements: spec://oproto/PROP-003#verification.timeout
```

Do not mix unrelated changes in one commit. This is not cosmetic — it is the mechanism by which spec traceability is maintained. A commit that touches two unrelated spec points is two commits that were collapsed, making the REVIEW diff ambiguous.

---

## 10. Memory Architecture

### Shared Persistent Memory (files)

The only memory that survives both processes:

- **Specs** — what the system must do
- **Code** — what the system currently does
- **WAL** — what to do next and what decisions are pending
- **Git history** — what changed and why (expensive for AI to read; use selectively)

### Per-Process Working Memory

| Process | Medium | Capacity | Persistence |
|---------|--------|----------|-------------|
| Human | Brain | ~7 items | Across years |
| AI | Context window | ~200K tokens | One session only |

### Strategy

**Files are the only shared memory that survives both processes.** Everything that must cross a session boundary must be written to a file.

- Minimize load on both: short sessions for AI (fresh context window), specs instead of code for human (compressed representation), diff instead of full file read.
- Record decisions, not facts. Facts are recoverable from code. Decisions are not.
- Everything not recorded does not exist for AI at next session start.

### "Lost in the Middle" Effect

Attention weight is not uniform across the context window. Hot zones are the beginning and the end. Content loaded 100K tokens ago receives statistically negligible attention weight at the point of generation.

**Practical consequence:** short sessions (five sessions of 30 minutes) are more effective than long sessions (one session of 2.5 hours). Each new session starts with a fresh context window where WAL and current specs are at the hot zone.

Do not load specs "for context" if they are not referenced in the current TODO. They consume tokens and slip into the cold zone.

---

## 11. Effectiveness Metrics — Full

Collect at team level, not per-session. Aggregate weekly. Use to validate that the protocol is producing results, not just overhead.

| Metric | Formula | What It Measures |
|--------|---------|-----------------|
| Iteration Count | commits until "done" per task | Convergence speed |
| REVIEW Count | REVIEW markers per session | AI concern signaling (healthy) |
| Spec Violations | drift-check violations per session | Actual drift (unhealthy) |
| First-Pass Accuracy | first AI output passes review / total | AI alignment with intent |
| Human Review Time | minutes reviewing per task | Cognitive load on human |
| Session Efficiency | tasks completed / sessions | Throughput per context window |
| WAL Continuity | successful WAL resumes / total sessions | Protocol adoption |
| Spec Coverage | trace-check matched / total artifacts | Traceability |

A high REVIEW Count with low Spec Violations is a healthy signal — it means the AI is flagging disagreements correctly rather than silently drifting. A low REVIEW Count with high Spec Violations is the failure mode.

---

## 12. Validation Protocols

### A/B Test (Controlled Experiment)

Purpose: measure whether the protocol produces better outcomes than plain prompting.

Setup:
1. Select two comparable tasks from the same domain
2. Control: plain prompting, no specs, no WAL, no URI addressing
3. Treatment: full protocol (spec, WAL, URI, REVIEW markers)
4. Counterbalance order across pairs (pair 1: control first; pair 2: treatment first) to isolate sequence effects
5. Minimum 3 pairs for a reliable signal

Collect per task: iteration count, total tokens, human review time, defect count, first-pass accuracy.

### Replay Test (Reproducibility)

Purpose: measure whether the protocol produces consistent results across independent runs.

Setup:
1. Select a completed task with full git history
2. Clean-room: new session, only the original requirements as input, no access to prior implementation
3. Execute with full protocol
4. Compare result to original: architecture convergence, issues found, metrics, divergence points

Most valuable output from replay: WHERE and WHY results differ. Divergence points reveal where the protocol's specification is ambiguous or where the AI's behavior is non-deterministic in ways that matter.
