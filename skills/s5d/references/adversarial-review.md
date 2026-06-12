# Adversarial Review

Adversarial review is a layered review pattern for material S5D changes. It is
not a separate source of truth. It becomes S5D evidence only when bound to a
spec, hypothesis, record, or gate result.

## Use It When

- Closing `gate:review`.
- Reviewing decision-tier or high-tier work.
- Reviewing a material feature diff.
- The implementation may have drifted from the spec.
- Acceptance scenarios are broad, safety-sensitive, or easy to satisfy
  superficially.
- An agent produced code and the weak link may be hidden in assumptions.

## Layers

| Layer | Inputs | Purpose |
|-------|--------|---------|
| Blind Diff Hunter | Diff only | Find suspicious changes, hidden coupling, unsafe defaults, missing tests, and obvious regressions without being biased by intent. |
| Edge Case Hunter | Diff plus relevant project files | Walk boundaries, nulls, permissions, concurrency, migrations, compatibility, failure states, and rollback paths. |
| Acceptance Auditor | Diff plus spec, Shape kernel, PRD/UX/story companions, and gate results | Verify that the implemented behavior actually satisfies the accepted intent and does not quietly change scope. |

Run all three for high-tier or decision-gated work. For smaller material changes,
run the layers that match the risk and record any skipped layer as a limitation.

## Finding Format

Each finding should be actionable:

```text
Severity: blocker|high|medium|low
Title: <short issue>
Evidence: <file/path/command/spec reference>
Violated constraint: <acceptance/gate/architecture/security invariant>
Impact: <what breaks or becomes risky>
Disposition: fix|required-defer|accepted-risk|not-a-bug
```

## Triage

- `blocker` or `high` findings fail the review gate until fixed or explicitly
  accepted by the human owner.
- `medium` findings must be fixed, deferred with owner/context, or converted
  into a follow-up spec.
- `low` findings can be noted, but still need a disposition.
- Untriaged findings mean the review is not closed.

## Evidence Binding

For decision/high-tier specs, bind review closure with:

```bash
s5d decision add-evidence <spec> \
  --hypothesis-id <id> \
  --evidence-type gate:review \
  --verdict pass \
  --content "<review summary and disposition>"
```

For feature specs, bind the result through the record or gate mechanism available
for that spec. If no binding surface exists yet, leave the review as a companion
report and mark the review gate unresolved.

## Failure Modes

- No spec or accepted intent: the Acceptance Auditor is blocked, not passed.
- Missing layer: record the missing layer as a limitation.
- Markdown-only report: useful companion input, not a passing gate.
- Unresolved finding: review gate remains open.
- Reviewer and implementer are the same agent without an explicit limitation:
  confidence is lower and must be stated.
