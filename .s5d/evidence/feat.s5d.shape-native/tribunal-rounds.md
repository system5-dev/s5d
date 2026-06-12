# Tribunal — feat.s5d.shape-native (PR #14, 2026-06-12)

Codex adversarial review, arbiter-only (external panel blocked: Claude keychain
OSStatus -60008, Gemini interactive OAuth — same environment flakiness as prior
tribunals). Reviewer independently ran `cargo test --all` each round.

## Round 1 — REJECT (2 blockers, both confirmed and fixed)

1. **Path traversal via spec.id**: `scaffold_adversarial_review` used spec.id
   (file content, not a sanitized CLI arg) as a path segment —
   `id: ../../escape` wrote outside `.s5d/evidence/`. Fixed by `sanitize_id`
   enforced inside the scaffold; the same pre-existing pattern in
   `gates.rs run_gates` got the same guard. Pinned by
   `review_adversarial_rejects_path_traversal_spec_id`.
2. **MCP stories without project confinement**: `s5d_plan_stories` wrote via
   `load_spec_yaml_mcp` (no project resolution) — any absolute `*.s5d.yaml`
   was writable, diverging from CLI. Fixed by `load_spec_context_mcp`.

Non-blocking fixed: review numbering TOCTOU → `create_new` loop.
Non-blocking REFUTED: reviewer's claim that `rollback` sat outside the MCP
story schema properties was checked against source — the field is inside
`items.properties`; no change.

## Round 2 — REJECT (1 blocker, confirmed and fixed)

3. **Unsafe story ids**: story.id flowed into `workflow.phases[].id`, later a
   filesystem path component in run-task artifacts (`save_task_artifact`).
   Fixed by `sanitize_id` on every story id at the `apply_stories` ingress —
   covers CLI and MCP (shared fn). Pinned by
   `plan_stories_rejects_unsafe_story_ids`.

Round-1 fixes verified resolved in the same round.

## Round 3 — APPROVE

No blocking issues. Reviewer verified: shared ingress from both entries,
sanitization before phase push, downstream artifact paths, 215 tests passed
including the new negative tests and MCP wire parity.
