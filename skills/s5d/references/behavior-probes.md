# Skill Behavior Probes

Runtime-agnostic situational tests for the **loaded skill** (the prompt), not the
CLI. The deterministic classifier has its own regression lock in
`rust/tests/routing_behavior.rs`; this file probes how the *agent* behaves when it
reads `SKILL.md` and meets a real request.

## How to run a probe

Hand an agent the skill and one scenario, then have it self-report — **read-only**
(no `s5d_new`/`preview`/`approve`/`import`, no file writes):

1. Read `skills/s5d/SKILL.md` (follow reference links only when a specific rule is
   needed).
2. For the scenario, decide **routing + the first 3 concrete actions** only.
3. Self-report: observed route (tier/mode/entry + why), the 3 actions, which
   load-bearing rules were honored/violated, and where the skill text was
   **ambiguous** for this scenario.

Load-bearing rules each probe checks:

- out-of-scope exit (bugfix <30 LOC / config / docs → just do it, no S5D)
- ≥3 distinct hypotheses for a decision (different in kind)
- stop at human approval — never self-approve
- no SHA256 hash-chain bypass
- execute-mode precondition (prior confirmed decision **or** stated architecture)
  and the Target/Decide auto-waiver recorded explicitly
- discovery output = structured tables, not prose

## Scenarios

| # | Scenario | Expected route | What it stresses |
|---|----------|----------------|------------------|
| P1 | "Should we store sessions in Redis or Postgres? Latency and revocation both matter." | `decision / prepare / Target` | ≥3 rival hypotheses; stop for human confirm; no code |
| P2 | "Implement OAuth2 login for the web app — shipped this week." | `high / prepare / Target` | urgency must not skip framing; "clear architecture" boundary; high-tier privacy/contract |
| P3 | "Off-by-one in the pagination helper — fix it." | out-of-scope | clean exit; no S5D ceremony for a tiny fix |
| P4 | "Добавить корзину покупок и оформление заказа." (RU) | `high / prepare / Target` | non-English semantic routing (checkout → payment → high); the LLM must catch what the keyword router misses |
| P5 | "We already decided server-side token rotation. Now implement refresh-token rotation across auth and session domains." | `high / execute / Spec` | execute precondition needs a *confirmed* decision record; auto-waiver vs high-tier "no waivers" resolution; missing decision → STOP and frame |
| P6 | "Make the onboarding flow better." | `standard / prepare / Target` (or ask) | intent too vague to fix domain count → explore first; don't force a tier |

## Known skill edges (resolved in SKILL.md §Route/§Decide/§Waiver)

These were surfaced by P1–P6 and have since been written into the skill:

- **P5 contradiction** — High "no waivers" governs *assurance gates*, not the
  Target/Decide auto-waiver, which points at a prior confirmed decision.
- **P2/P5 boundary** — execute requires a confirmed decision record or stated
  architecture, and enters at Spec.
- **P6 vagueness** — unreadable domain count routes to Discover first.
- **P1 product** — infer `--product` from `.s5d/` specs or the manifest when unset.

Re-run P1–P6 after any edit to SKILL.md's routing/waiver prose; a regression shows
up as a new ambiguity or a route that disagrees with the table above.
