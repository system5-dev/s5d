> **DEPRECATED.** This skill is superseded by the unified `/s5d` flow. Use `/s5d` instead.

# S5D Session Protocol

Operational protocol for human-AI collaboration across sessions. Defines how two processes (human + AI) share state, resolve conflicts, and maintain coherence.

Based on the coprocessor model: human and AI are complementary processors with different architectures. Human owns coherence and long-term memory. AI owns throughput and mechanical consistency. Files are the only shared memory that survives both.

**Prerequisite:** S5D must be initialized (`/s5d` bootstrap). This skill extends S5D with session-level protocol — it does not replace it.

---

## 1. WAL — Write-Ahead Log

The WAL records **continuation state** — what to do next, not what was done. Borrowed from database systems: record intention before execution, enabling recovery after session termination.

### Format

```markdown
# WAL — <project name>

## Current Phase
<spec-id>: <title> — <STATUS>

## Completed
- <spec-id>: COMPLETE
- <spec-id>§1-4: COMPLETE, all tests pass

## In Progress
- <spec-id>§5 (Verification):
  - DONE: <what's finished> (spec://<module>/<doc>#<section>)
  - TODO: <what's next>

## Blocked
- <issue>: awaiting <what> from <whom>

## Decisions Pending
- spec://<module>/<doc>#<section>: <question> — awaiting human decision

## Anti-Instructions
- Do NOT touch <component> — stable, tested, fragile
- Do NOT refactor <module> — out of scope

## REVIEW Markers Pending
- <file>:<line> — <AI concern> (see §4)
```

### Spec-ID Convention

WAL uses **bare IDs** for brevity: `PROP-003`, `FEAT-001§3`. Full `spec://` URIs are used in task descriptions, commit messages, and REVIEW markers. The mapping:
- WAL: `PROP-003§5` (short, scannable)
- Task/commit/REVIEW: `spec://oproto/PROP-003#verification.timeout` (precise, addressable)

### Rules

1. **Updated at session end.** Last action before closing. If session crashes without WAL update, see Recovery below.
2. **Contains continuation data, not history.** Git log is the history.
3. **Completed items collapse.** "PROP-001§1-4: COMPLETE" not four separate lines.
4. **Anti-instructions protect stable code.** AI reads these before touching anything.
5. **Lives at `specs/WAL.md`** (or project root if no specs/ directory). For multi-product repos: `specs/<product>/WAL.md`.
6. **Both processes write.** Human updates decisions/priorities. AI updates progress/blockers.
7. **Read-first protocol.** Every session starts by reading WAL. No exceptions.

### Recovery (Dirty WAL)

If a session ends abnormally (crash, timeout, kill), the WAL may be stale.

**Detection:** At session start, compare WAL's last commit timestamp with the most recent code commit. If code is newer than WAL — flag as potentially stale.

**Recovery procedure:**
1. Read WAL as-is (it's the best available state)
2. Read `git log --oneline` since WAL's last commit
3. Reconstruct: mark completed items from git history, identify in-progress work
4. Update WAL to reflect reconstructed state
5. Proceed normally

### Session Lifecycle

```
SESSION START:
  1. Read WAL (5 min human, instant AI)
  2. Read specs referenced in "In Progress"
  3. Identify next TODO item
  4. Begin work

SESSION END:
  1. Update WAL with progress
  2. Collapse completed items
  3. Add any new blockers/decisions pending
  4. Add anti-instructions if new stable code was created
  5. Commit WAL update
```

### Boot Sequence

When AI starts a session, it loads context in this order:

```
1. BOOT.md / CLAUDE.md  — project instructions (~500 tokens)
2. WAL.md               — continuation state (~1-3K tokens)
3. Relevant specs        — only those referenced in WAL "In Progress"
4. Code                  — only files relevant to current TODO
```

This minimizes context window consumption. Do NOT load all specs. Do NOT read entire codebase. WAL tells you exactly what to load.

---

## 2. IPC — Files as Inter-Process Communication

Files are not documentation. They are the communication protocol between human and AI. Four requirements:

### 2.1 Addressability

Every spec element must be precisely locatable. When AI drifts, human must point to the exact violation in seconds.

**URI scheme:**
```
spec://<module>/<document>#<section>[.<subsection>]
```

Examples:
```
spec://oproto/PROP-003#verification.timeout
spec://s5d/decision.redbook-expansion#problem
spec://auth/FEAT-001#acceptance.scenario-2
```

**Rules:**
- Use in WAL, commit messages, REVIEW markers, and AI task descriptions
- Human uses URIs in task descriptions instead of prose ("implement spec://X" not "implement the timeout thing")

**Resolution precedence** (how AI maps spec:// URI to file path):
1. `.s5d/packages/<document>*.s5d.yaml` (S5D-managed specs — preferred)
2. `specs/<module>/<document>.md` (manual specs directory)
3. Project root `<document>.md` (fallback)

First match wins. If no file resolves — report to human, do not guess.

### 2.2 Atomicity

One commit = one logical change. One thought, one diff.

**Rules:**
- Each commit references the spec point it implements:
  ```
  [oproto] implement verification timeout
  Implements: spec://oproto/PROP-003#verification.timeout
  ```
- No mixing unrelated changes in one commit
- AI generates atomic commits naturally — enforce this, don't fight it

### 2.3 Conflict Resolution Protocol

Two writers will produce contradictions. This is normal. Explicit priority:

```
1. Human overrides spec    (human can change the spec)
2. Spec overrides code     (code must match spec)
3. Tests = executable spec (test contradicts spec → bug in test OR spec, not both)
```

**When AI disagrees with spec:**
1. Implement spec exactly as written
2. Add REVIEW marker (see §4)
3. Report to human with spec:// reference
4. Human resolves in next cycle

**Never silently modify a spec.** If AI thinks spec is wrong, the protocol is: implement, mark, report. Not: rewrite.

**When test contradicts spec:** Escalate — add REVIEW marker on the test, add to WAL "Decisions Pending". Do not silently pick one over the other.

**Mid-session spec change:** If human updates a spec while AI is implementing the old version:
1. AI detects changed spec (file modification timestamp or git diff)
2. AI stops current task immediately
3. AI re-reads the updated spec
4. If changes affect current work — report delta to human, ask whether to continue or restart task
5. If changes don't affect current work — continue, note in WAL that spec was updated mid-session

### 2.4 Visibility of Changes

Both processes must see fresh data, not cached versions.

- Each session reads fresh WAL (not cached from prior session)
- AI reads specs from disk, not from prior context
- Git diff is the canonical change signal
- Human reads diff before approving — always

---

## 3. Three IPC Levels

### Control Plane (human -> AI)
- **BOOT.md / CLAUDE.md** — how to read everything else (~500 tokens)
- **WAL.md** — continuation state (up to 3K tokens)
- **Specs** — what the system should do

### Data Plane (AI -> human, verifiable)
- **Code** — generated by AI, verified by human via diff
- **Tests** — generated by AI per specs, verified by passing
- **Spec updates** — AI proposes, human approves

### Signal Plane (bidirectional)
- **REVIEW markers** — AI flags concerns (see §4)
- **Git diff** — change notification
- **Broken tests** — automated spec violation signal
- **WAL updates** — session boundary signal

---

## 4. REVIEW Markers

When AI implements something it disagrees with, or makes a non-obvious decision:

```
<!-- REVIEW: spec://oproto/PROP-003#verification.timeout
     AI concern: 600s timeout may be too long for real-time chat.
     Suggested: 120s with configurable override.
     Implemented as specified (600s). -->
```

**In code:**
```rust
// REVIEW(spec://oproto/PROP-003#verification.timeout):
// 600s may be too long for real-time. Consider 120s.
// Implemented as specified.
fn verification_timeout() -> Duration {
    Duration::from_secs(600) // per spec, not AI preference
}
```

**Rules:**
- Always include spec:// URI reference
- Always state what was implemented (the spec) vs what AI would prefer
- Human resolves in next cycle: accept spec, update spec, or acknowledge and keep
- WAL tracks pending REVIEW markers under "REVIEW Markers Pending"
- Resolved markers are removed from code/WAL (not accumulated)

---

## 5. Effectiveness Metrics

How to measure whether S5D (and this session protocol) actually works.

### Metric Definitions

| Metric | Formula | What it measures |
|--------|---------|-----------------|
| **Iteration Count** | N commits until "done" for a task | Convergence speed — fewer = better |
| **REVIEW Count** | REVIEW markers added per session | AI concern signaling — healthy signal, not a failure |
| **Spec Violations** | `s5d drift-check` violations per session | Actual drift from spec — lower = better |
| **First-Pass Accuracy** | Tasks where first AI output passes human review / total tasks | AI alignment with intent |
| **Human Verification Time** | Minutes human spends reviewing AI output per task | Cognitive load on human |
| **Session Efficiency** | (tasks completed) / (sessions spent) | Throughput per context window |
| **WAL Continuity** | Sessions where AI successfully resumed from WAL / total sessions | Protocol adoption |
| **Spec Coverage** | `s5d trace check` matched artifacts / total artifacts | Traceability (artifact-level, not line-level) |

### Measurement Protocol

Collect metrics per session. Record in a metrics log:

```markdown
# Metrics — <date>

## Session
- Duration: <minutes>
- Tasks attempted: <N>
- Tasks completed: <N>
- WAL resume: yes/no

## Per Task
| Task | Iterations | First-pass | Review time | REVIEW markers |
|------|-----------|------------|-------------|----------------|
| ...  | ...       | yes/no     | <min>       | <count>        |
```

Aggregate weekly. Compare across periods to detect trends.

---

## 6. Validation Tests

Two test types to verify the framework works.

### 6.1 A/B Test (Controlled Experiment)

**Purpose:** Compare S5D+session protocol vs ad-hoc on the same type of task.

**Protocol:**

1. **Select task pair.** Two tasks of comparable complexity from the same domain. Not the same task twice — that's replay (§6.2).

2. **Control (no S5D):** Execute task A with plain prompting. No specs, no WAL, no URI scheme. Record:
   - Iterations to completion
   - Human verification time
   - Defects found after "done"
   - Total tokens consumed

3. **Treatment (with S5D):** Execute task B with full protocol. Spec first, WAL, URI references, REVIEW markers. Record same metrics.

4. **Compare.** Delta on each metric. Positive = S5D helped. Negative = overhead exceeded benefit.

**Validity requirements:**
- Same human, same AI model, same time of day (fatigue control)
- Tasks from same domain and comparable complexity (not trivial vs hard)
- Minimum 3 task pairs for statistical signal
- **Counterbalance order:** Alternate control/treatment across pairs (pair 1: control first, pair 2: treatment first, pair 3: control first). This prevents learning-order confound.
- Record raw data, not just summary

### 6.2 Replay Test (Reproducibility)

**Purpose:** Re-execute a completed task with S5D protocol. Compare with original execution.

**Protocol:**

1. **Select completed task.** Something already shipped. Must have git history to compare against.

2. **Clean-room replay.** New session, no memory of original implementation. Only inputs: original requirements + S5D protocol.

3. **Execute with full S5D:** Spec, WAL, URI scheme, REVIEW markers. Record all metrics.

4. **Compare with original:**
   - Did the replay produce the same architecture? (convergence)
   - Did it find issues the original missed? (quality improvement)
   - How do metrics compare? (efficiency)
   - Where did it diverge? (protocol impact points)

5. **Record divergence analysis.** Most valuable output: WHERE and WHY results differ. This reveals what the protocol adds vs what was already implicit.

**Validity requirements:**
- Clean-room: no peeking at original implementation during replay
- Same requirements document as input
- Different session from original (fresh context)
- Document all divergences, not just summary metrics

---

## Integration with S5D

This skill is a **session-level protocol layer** on top of S5D's decision/feature/trace lifecycle.

| S5D Concept | Session Protocol Extension |
|-------------|--------------------------|
| Decision (Steps 1-6) | WAL tracks pending decisions. Conflict protocol applies to spec disagreements. |
| Feature (Step 7+) | WAL tracks feature progress. REVIEW markers flag implementation concerns. Specs use URI scheme. |
| Trace | REVIEW markers are traceable via spec:// URIs. Drift detection includes REVIEW marker resolution. |
| Bootstrap | Boot sequence (§1) extends S5D's Phase 0a with WAL read. |

**Invoke:** `/s5d-session` for protocol reference. Use alongside `/s5d`, `/s5d-feature`, `/s5d-trace`.

---

## Do NOT (per-session rules)

- Skip WAL read at session start
- Skip WAL update at session end
- Silently modify specs without REVIEW marker
- Use prose references when spec:// URI exists
- Mix unrelated changes in one commit
- Accumulate resolved REVIEW markers (remove them)

## Do NOT (team-level, for framework validation)

- Measure effectiveness without both A/B and replay tests (§6)
- Draw conclusions from uncontrolled comparisons (see §6.1 counterbalancing)
