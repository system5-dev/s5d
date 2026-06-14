# Testing and Benchmarking Contract

S5D quality is covered by layered evidence, not by one test type pretending to
prove everything.

## Target State

| Layer | Purpose | Canonical surface |
|---|---|---|
| Unit tests | Pure Rust behavior, validators, routers, gates, scanners | `cargo test` in `rust/` |
| Integration tests | Full CLI/MCP/state-machine behavior in temporary repos | `rust/tests/standalone_repos.rs`, `rust/tests/mcp_wire.rs` |
| Docs/runtime sync tests | Drift guard between loaded skills, docs, and command surface | `rust/tests/docs_sync.rs` |
| Scenario tests | Behavioral batteries for routing and methodology edge cases | `rust/tests/routing_behavior.rs`, `.s5d/scenarios/` when mined |
| Fuzzy scenario evaluation | Human/model-judged behavior where exact equality is wrong | `s5d run benchmark` rubric scores, required scenario tags, and raw artifacts |
| Codebase coverage | Which source modules are governed by accepted specs | `.s5d/codebase/coverage.yaml` via `s5d codebase sync/check` |

The weakest link today is not the absence of Rust tests. It is keeping the
behavioral/scenario layer and assistant-effectiveness layer reproducible enough
that a later agent cannot replace evidence with a story.

## Benchmark Contract

`s5d run benchmark <suite.json|yaml>` scores fixed assistant variants against a
fixed rubric. The intended first comparison is:

- `native`: the assistant receives normal repo context and task instructions.
- `skill`: the assistant receives the same task plus the relevant S5D skill flow,
  WAL/spec protocol, and accepted state links.

Hold constant:

- model and service tier
- prompt/task text
- repo commit
- time or turn budget
- allowed tools
- verifier who fills the rubric

The benchmark proves only the frozen task set under those conditions. It does
not prove global superiority of a skill.

Each production benchmark suite should declare `required_tags` for the scenario
families it claims to cover. The scorer fails if no case carries one of those
tags, so "broad coverage" cannot silently shrink to one happy-path task.

## Suite Schema

```json
{
  "benchmark": "S5D skill vs native assistant",
  "scale": "criterion scores are judged 0.0..1.0; output is weighted percent",
  "required_tags": ["happy-path", "edge-case", "failure-handling", "scope-drift", "stale-intent"],
  "artifacts": [
    {"kind": "protocol", "path": "docs/testing-and-benchmarking.md"}
  ],
  "criteria": [
    {"id": "correctness", "weight": 4},
    {"id": "scope", "weight": 2},
    {"id": "evidence", "weight": 3},
    {"id": "tests", "weight": 1}
  ],
  "cases": [
    {
      "id": "T01",
      "name": "Review a diff against accepted intent",
      "tags": ["scope-drift"],
      "artifacts": [
        {"kind": "prompt", "path": "benchmarks/T01/prompt.md"}
      ],
      "runs": [
        {
          "id": "native",
          "scores": {"correctness": 0.5, "scope": 0.6, "evidence": 0.2, "tests": 0.4},
          "artifacts": [{"kind": "transcript", "path": "benchmarks/T01/native.md"}]
        },
        {
          "id": "skill",
          "scores": {"correctness": 0.9, "scope": 0.9, "evidence": 1.0, "tests": 0.8},
          "artifacts": [{"kind": "transcript", "path": "benchmarks/T01/skill.md"}]
        }
      ]
    }
  ]
}
```

Run:

```bash
s5d run benchmark examples/skill-benchmark.json
s5d run benchmark examples/skill-benchmark.json --format json
```

## Guardrails

- Missing `required_tags` coverage fails the run; a suite cannot claim scenario
  breadth without at least one case for each declared family.
- Missing criterion scores fail the run; silence is not treated as zero-quality
  evidence.
- Rubric scores are inputs and should be backed by raw artifacts: prompt,
  transcript, diff, test output, and reviewer notes.
- A passing benchmark is not an S5D gate. Bind it as decision evidence only when
  the task needs an explicit tradeoff record.
- Benchmark tasks should include happy path, edge cases, failure handling,
  scope-drift traps, and stale-intent replay.
