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
- vague product intent enters Shape before Target/Spec
- adversarial review reports become gate/evidence only when bound into S5D state

## Scenarios

| # | Scenario | Expected route | What it stresses |
|---|----------|----------------|------------------|
| P1 | "Should we store sessions in Redis or Postgres? Latency and revocation both matter." | `decision / prepare / Target` | ≥3 rival hypotheses; stop for human confirm; no code |
| P2 | "Implement OAuth2 login for the web app — shipped this week." | `high / prepare / Target` | urgency must not skip framing; "clear architecture" boundary; high-tier privacy/contract |
| P3 | "Off-by-one in the pagination helper — fix it." | out-of-scope | clean exit; no S5D ceremony for a tiny fix |
| P4 | "Add a shopping cart and checkout flow." | `high / prepare / Target` | semantic routing (checkout → payment → high); the LLM must catch what a keyword router may miss |
| P5 | "We already decided server-side token rotation. Now implement refresh-token rotation across auth and session domains." | `high / execute / Spec` | execute precondition needs a *confirmed* decision record; auto-waiver vs high-tier "no waivers" resolution; missing decision → STOP and frame |
| P6 | "Make the onboarding flow better." | `unclassified / shape / Shape` | intent too vague to fix domain count → Shape first; don't force a tier |
| P7 | "Review the agent's implementation before closing gate:review." | `current-tier / prepare / Adversarial Review` | review report is only a companion until findings are triaged and bound as S5D evidence/gate state |

## Known skill edges (resolved in SKILL.md §Route/§Decide/§Waiver)

These were surfaced by P1–P6 and have since been written into the skill:

- **P5 contradiction** — High "no waivers" governs *assurance gates*, not the
  Target/Decide auto-waiver, which points at a prior confirmed decision.
- **P2/P5 boundary** — execute requires a confirmed decision record or stated
  architecture, and enters at Spec.
- **P6 vagueness** — unreadable domain count routes to Shape first; Discover follows only when current architecture ownership is missing.
- **P7 review closure** — adversarial review closes only through fixed/deferred findings and S5D evidence/gate binding.
- **P1 product** — infer `--product` from `.s5d/` specs or the manifest when unset.

Re-run P1–P7 after any edit to SKILL.md's routing/waiver prose; a regression shows
up as a new ambiguity or a route that disagrees with the table above.
