# S5D Skill Suite — stage routing

S5D ships a suite of stack-agnostic domain skills alongside the `s5d` control
plane. They are **not** separate tools to remember — each maps to a stage of the
S5D flow, and the control plane recommends (or invokes) the right one **at the
right place automatically**.

When you reach a stage below, pull the listed skill before doing that stage by
hand. Each skill is read-only / plan-emitting by default; the producer skills
(`system-design`, `domain-refactor`) emit S5D artifacts through the `s5d` CLI.

| S5D stage | Skill | Contributes | Producer? |
|-----------|-------|-------------|-----------|
| **Discover** | `ddd-refactor` | domain **modeling** quality — anemic models, value objects, missing ACL at integration seams | report |
| **Discover** | `domain-refactor` | **boundary** violations vs `.s5d/discovery/architecture-map.md` (god-components, drift, orphans) | emits decision skeleton via `s5d new` |
| **Discover** | `ai-technical-writer` | **docs** architecture — single home, `llms.txt`, front-matter, broken xrefs, orphans | report + `generate` |
| **Discover** | `infra-scan` | deploy planes, undeclared env vars, infra posture | report |
| **Discover** | `scaling-review` | scaling bottlenecks, hot paths | report |
| **Discover** | `security-scan` | security posture — SAST / SCA / secrets / IaC / SBOM | waivers (MCP `s5d_waiver` canonical) |
| **Discover → Spec** | `scenario-mine` | extract BDD (Given/When/Then) from existing code → `.s5d/scenarios/` + acceptance YAML | acceptance snippets (YAML-layer) |
| **Target / Decide** | `system-design` | interview-driven technology selection → **decision-tier ADR** | emits decision spec via `s5d new` + `s5d decision add-hypothesis` |
| **Spec** | `scenario-mine` | generate the ≥3 GWT acceptance scenarios a feature spec requires per use case | acceptance snippets |
| **Run / Verify (gates)** | `unit-tests`, `e2e-tests`, `code-quality` | install + run the gate suite (tests, lint/format) so local == CI | gate evidence |
| **Run / Verify (gates)** | `security-scan` | run scanners with severity gating; emit SARIF/SBOM/JUnit | gate evidence + waivers |

## How "automatic" works

1. **Discovery sweep.** When running `Discover`, run the Discover-row skills in
   detect/report mode to populate the architecture map and surface findings —
   this is the read-only assessment pass. Their reports become the evidence the
   Target/Decide stages reason over.
2. **Decision authoring.** At `Target`/`Decide`, `system-design` runs the
   interview and lands a decision-tier ADR through the CLI; `domain-refactor`
   lands its move plan the same way. No hand-rolled YAML.
3. **Spec acceptance.** At `Spec`, `scenario-mine` mines the GWT scenarios so the
   feature spec's `use_cases[].acceptance` is grounded in real code, not invented.
4. **Gate execution.** At `Run`/`Verify`, the test/lint/security skills are the
   gate commands `s5d run-gates` shells out to.

## Routing precedence

- Pick the skill whose `description:` trigger matches the request first (`/find`).
- If two skills overlap, the table's stage column disambiguates: modeling quality
  → `ddd-refactor`; boundary-vs-map → `domain-refactor`; markdown *syntax* →
  `code-quality`; docs *architecture* → `ai-technical-writer`.
- A skill with no matching stage need (e.g. no docs to organize) is simply skipped
  — the suite is a menu keyed by stage, not a mandatory checklist.
