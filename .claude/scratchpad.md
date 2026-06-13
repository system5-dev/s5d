# Handoff — feat.s5d.thin-assurance-primitives (реорг)

**Status:** FRAMED, not designed/implemented. Pick up in a fresh session.
**Spec:** `.s5d/packages/feat.s5d.thin-assurance-primitives__20260613.s5d.yaml` (standard, proposed)
**Backs:** `decision.s5d.assurance-kernel-positioning` (committed `48ae529`) — winner hypothesis `s5d-repo-state-assurance-kernel-bmad-класс-внешний-skill-адаптер`, now spec_ref-linked to this feat (closes the earlier --force).

## Goal (one sentence)
Make S5D's MCP primitive surface complete + ergonomic enough that an external BMAD-class
workflow-skill can drive the full lifecycle (intake → shape → evidence → gates → decision →
reflect) **through MCP calls alone** — no hand-edited YAML, no opaque walls — while ALL
sequencing / persona / elicitation stays in the external skill (HOW) and ALL state /
verification stays in S5D (WHAT).

## Hard constraints (from the decision + note.blueprint-projection)
- control-plane ≠ orchestrator: S5D defines WHAT is verified; the agent/skill defines HOW. No role/phase sequencing inside S5D.
- S5D does NOT own telemetry / feature-flags / CI — it links via telemetry_refs / ROC.
- "Thin" applies to the assurance plumbing (gates/evidence/decision), NOT to elicitation+persona content (that lives in the external skill).
- Minimal-sufficient: close real gaps, do not build a workflow runtime.

## Structure outline (what to design)
1. **Primitive-API completeness audit** — which lifecycle steps lack an MCP setter/getter an external skill needs?
   - DONE this session: `s5d_set_acceptance` (was missing), actionable gate errors (were opaque).
   - KNOWN GAP found: `set-acceptance` only sets on an existing problem **Card**; standard-tier `generate_spec` does NOT create a Card (decision-tier does). So a feature skill can't init a problem card via MCP. Decide: should standard specs get a Card, or a `set-problem` that creates one?
   - TODO audit: can a skill set problem.constraints/targets, read gate status/results, list hypotheses, via MCP? Map the full surface vs the lifecycle.
2. **Ergonomics**: actionable errors (started in phase_gates.rs enforce_checks), idempotency, dry-run, machine-readable outputs.
3. **The external-skill contract**: define the canonical MCP call sequence a BMAD-class adapter uses = the "thin" reference. This is documentation/contract, not code in S5D.
4. **Boundary tests**: assert no orchestration leaks into S5D (the new primitives must not sequence).

## Decided scope — v1 (Roman, 2026-06-13)
1. **MCP-API completeness** (дополнение) — incremental gap-closing, NOT a new abstraction.
2. **Grow the elicitation kernel** (`s5d shape`) richer as an S5D primitive (BMAD's coaching strength) — растить.
3. **Write a reference BMAD-class adapter skill** to prove the surface — референсный.

## Vertical slices (each independently testable — built in order, /autopilot-able)
- **Slice 1 — problem-card primitive. ✅ DONE (2026-06-13, uncommitted).** `s5d decision set-problem` + MCP `s5d_set_problem` + lib `upsert_problem_card` (one helper, both surfaces) + unit test `upsert_problem_card_creates_then_updates`. Creates a ProblemCard on None/Text, updates fields on existing, requires --signal to create. Verified: feat.s5d.thin-assurance-primitives `problem: null` → card created (signal+acceptance), spec validates. Binary deployed+resigned to ~/bin/s5d. Files: lib.rs, bin/s5d.rs, mcp.rs. NOTE: `set-acceptance` (slice-0 fix) now overlaps set-problem --acceptance — fold/deprecate set-acceptance in a later cleanup.
- **Slice 2 — API-completeness audit + close gaps.** Map every lifecycle step (intake→shape→evidence→gates→decision→reflect) to its MCP primitive; add missing setters/getters (read gate results, list hypotheses, read decision, etc.). Test: a script drives a full lifecycle via MCP only, zero hand-edits.
- **Slice 3 — elicitation-kernel growth.** Enrich `s5d shape`: structured discovery (brain-dump intake, stakes calibration, gap/UNKNOWN surfacing) as a primitive the external skill calls — WITHOUT moving persona/sequencing into S5D. Test: shape emits a richer kernel from raw input.
- **Slice 4 — reference adapter skill.** A BMAD-class skill (analyst→pm→architect personas live IN the skill) that drives the full lifecycle purely via S5D MCP calls. Test: run end-to-end on a toy product; assert (a) full lifecycle completes via MCP, (b) no orchestration/persona leaked into S5D (boundary test).

Boundary invariant for every slice: S5D defines WHAT (state/verification); the skill defines HOW (sequence/persona/elicitation-content). No role/phase sequencing in S5D.

## Current repo state (2026-06-13, end of long session)
- main has `48ae529` (decision) + `4dfbc9c` (hardening: char-safe truncate, actionable errors, set-acceptance). NOT pushed.
- UNCOMMITTED now: decision spec re-dirtied (spec_ref linked to this feat) + this new feat spec/record. Commit with Roman's word (we're on main).
- `~/bin/s5d` updated+resigned with all fixes; live MCP server still on old binary (restart via /mcp for new tool to appear).
- Deploy gotcha: after `cp` to ~/bin/s5d → `codesign --force --sign -` (AMFI SIGKILL otherwise). Build via `cargo build --release --manifest-path rust/Cargo.toml` (Cargo.toml is in rust/, not repo root).

## Key file refs
- MCP tool surface + dispatch: `rust/src/mcp.rs` (defs ~158-515, handlers below).
- decide prereqs: `rust/src/phase_gates.rs:34 check_decide` (≥3 hyp, each-has-evidence, acceptance).
- problem card schema: `rust/src/models.rs:569 ProblemCard` (acceptance: Option<String>).
- truncate helper + test: `rust/src/lib.rs truncate_chars`.
